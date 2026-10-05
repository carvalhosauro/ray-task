//! Atalho no menu do sistema na primeira execução: os instaladores de uma linha
//! (`curl | sh`, `irm | iex`) só copiam o binário, e o app ficava invisível no menu.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Desliga a criação do atalho (ex.: quem gerencia o menu por conta própria).
pub const OPT_OUT_VAR: &str = "RAY_TASK_NO_SHORTCUT";

pub struct Shortcut {
    /// Binário que o atalho abre.
    pub exe: PathBuf,
    /// `dirs::data_dir()`: ~/.local/share no Linux, %APPDATA% no Windows.
    pub data_dir: PathBuf,
    /// Instalações gerenciadas (MSI, pacote da distro) já trazem o próprio atalho.
    pub system_dirs: Vec<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Created,
    AlreadyPresent,
    Skipped(&'static str),
}

impl Shortcut {
    pub fn from_env() -> Option<Self> {
        if std::env::var_os(OPT_OUT_VAR).is_some() {
            return None;
        }
        let exe = std::env::current_exe().ok()?;
        let data_dir = dirs::data_dir()?;
        let system_dirs = if cfg!(windows) {
            ["ProgramFiles", "ProgramFiles(x86)"].iter().filter_map(std::env::var_os).map(PathBuf::from).collect()
        } else {
            vec![PathBuf::from("/usr")]
        };
        Some(Self { exe, data_dir, system_dirs })
    }

    pub fn ensure(&self) -> io::Result<Outcome> {
        if is_cargo_build(&self.exe) {
            return Ok(Outcome::Skipped("binário de desenvolvimento (target/ do cargo)"));
        }
        if self.system_dirs.iter().any(|dir| self.exe.starts_with(dir)) {
            return Ok(Outcome::Skipped("instalação do sistema traz o próprio atalho"));
        }
        platform::ensure(self)
    }
}

/// `cargo build`/`cargo run` deixam o binário em <workspace>/target/<perfil>/.
fn is_cargo_build(exe: &Path) -> bool {
    exe.ancestors()
        .any(|dir| dir.file_name().is_some_and(|name| name == "target") && dir.parent().is_some_and(|p| p.join("Cargo.toml").is_file()))
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;

    const TEMPLATE: &str = include_str!("../../../packaging/ray-task.desktop");
    const ICONS: [(u32, &[u8]); 7] = [
        (16, include_bytes!("../assets/icon/ray-task-16.png")),
        (32, include_bytes!("../assets/icon/ray-task-32.png")),
        (48, include_bytes!("../assets/icon/ray-task-48.png")),
        (64, include_bytes!("../assets/icon/ray-task-64.png")),
        (128, include_bytes!("../assets/icon/ray-task-128.png")),
        (256, include_bytes!("../assets/icon/ray-task-256.png")),
        (512, include_bytes!("../assets/icon/ray-task-512.png")),
    ];

    pub fn ensure(s: &Shortcut) -> io::Result<Outcome> {
        let entry = s.data_dir.join("applications").join("ray-task.desktop");
        if entry.exists() {
            return Ok(Outcome::AlreadyPresent);
        }
        for (size, png) in ICONS {
            let icon = s.data_dir.join(format!("icons/hicolor/{size}x{size}/apps/ray-task.png"));
            if !icon.exists() {
                fs::create_dir_all(icon.parent().expect("caminho com pasta"))?;
                fs::write(&icon, png)?;
            }
        }
        fs::create_dir_all(entry.parent().expect("caminho com pasta"))?;
        fs::write(&entry, desktop_entry(&s.exe))?;
        Ok(Outcome::Created)
    }

    /// O modelo do repositório com `Exec` absoluto: sessões gráficas costumam não ter
    /// ~/.cargo/bin no PATH.
    fn desktop_entry(exe: &Path) -> String {
        let exec = format!("Exec={}", quote_exec(&exe.to_string_lossy()));
        TEMPLATE.lines().map(|line| if line.starts_with("Exec=") { exec.as_str() } else { line }).collect::<Vec<_>>().join("\n") + "\n"
    }

    /// Desktop Entry spec: argumento entre aspas escapa `"`, `` ` ``, `$` e `\`; o valor
    /// é string (`\` dobra de novo) e `%` vira `%%` para não ser lido como código de campo.
    fn quote_exec(path: &str) -> String {
        let mut out = String::from("\"");
        for c in path.chars() {
            match c {
                '"' | '`' | '$' => out.push_str(&format!("\\\\{c}")),
                '\\' => out.push_str("\\\\\\\\"),
                '%' => out.push_str("%%"),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }
}

#[cfg(windows)]
mod platform {
    use super::*;

    pub fn ensure(s: &Shortcut) -> io::Result<Outcome> {
        let lnk = s.data_dir.join(r"Microsoft\Windows\Start Menu\Programs\ray-task.lnk");
        if lnk.exists() {
            return Ok(Outcome::AlreadyPresent);
        }
        fs::create_dir_all(lnk.parent().expect("caminho com pasta"))?;
        mslnk::ShellLink::new(&s.exe).and_then(|link| link.create_lnk(&lnk)).map_err(io::Error::other)?;
        Ok(Outcome::Created)
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod platform {
    use super::*;

    pub fn ensure(_: &Shortcut) -> io::Result<Outcome> {
        Ok(Outcome::Skipped("sem atalho de menu nesta plataforma"))
    }
}
