use std::fs;
use std::path::{Path, PathBuf};

use ray_task::shortcut::{Outcome, Shortcut};

fn fake_exe(dir: &Path) -> PathBuf {
    let exe = dir.join(if cfg!(windows) { "ray-task.exe" } else { "ray-task" });
    fs::create_dir_all(dir).unwrap();
    fs::write(&exe, b"").unwrap();
    exe
}

fn shortcut(exe: PathBuf, data_dir: &Path) -> Shortcut {
    Shortcut { exe, data_dir: data_dir.to_path_buf(), system_dirs: vec![] }
}

#[test]
fn skips_binaries_built_by_cargo() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Cargo.toml"), "[workspace]").unwrap();
    let exe = fake_exe(&root.path().join("target").join("release"));
    let data = root.path().join("data");

    assert!(matches!(shortcut(exe, &data).ensure().unwrap(), Outcome::Skipped(_)));
    assert!(!data.exists(), "nada deve ser escrito para build de desenvolvimento");
}

#[test]
fn skips_system_installs() {
    let root = tempfile::tempdir().unwrap();
    let system = root.path().join("Program Files").join("ray-task");
    let exe = fake_exe(&system.join("bin"));
    let data = root.path().join("data");
    let s = Shortcut { exe, data_dir: data.clone(), system_dirs: vec![root.path().join("Program Files")] };

    assert!(matches!(s.ensure().unwrap(), Outcome::Skipped(_)));
    assert!(!data.exists());
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;

    #[test]
    fn creates_menu_entry_and_icons_pointing_at_the_exe() {
        let root = tempfile::tempdir().unwrap();
        let exe = fake_exe(&root.path().join("my bin"));
        let data = root.path().join("data");

        assert_eq!(shortcut(exe.clone(), &data).ensure().unwrap(), Outcome::Created);

        let entry = fs::read_to_string(data.join("applications/ray-task.desktop")).unwrap();
        assert!(entry.lines().any(|l| l == format!("Exec=\"{}\"", exe.display())), "Exec absoluto e entre aspas:\n{entry}");
        assert!(entry.contains("Icon=ray-task"));
        for n in [16, 32, 48, 64, 128, 256, 512] {
            let icon = data.join(format!("icons/hicolor/{n}x{n}/apps/ray-task.png"));
            assert!(fs::metadata(&icon).unwrap().len() > 0, "{} vazio", icon.display());
        }
    }

    #[test]
    fn escapes_percent_signs_in_the_exec_path() {
        let root = tempfile::tempdir().unwrap();
        let exe = fake_exe(&root.path().join("100%"));
        let data = root.path().join("data");

        shortcut(exe, &data).ensure().unwrap();

        let entry = fs::read_to_string(data.join("applications/ray-task.desktop")).unwrap();
        assert!(entry.contains("/100%%/ray-task\""), "% vira %% no Exec:\n{entry}");
    }

    #[test]
    fn keeps_an_existing_menu_entry() {
        let root = tempfile::tempdir().unwrap();
        let exe = fake_exe(&root.path().join("bin"));
        let data = root.path().join("data");
        let entry = data.join("applications/ray-task.desktop");
        fs::create_dir_all(entry.parent().unwrap()).unwrap();
        fs::write(&entry, "[Desktop Entry]\nExec=custom\n").unwrap();

        assert_eq!(shortcut(exe, &data).ensure().unwrap(), Outcome::AlreadyPresent);
        assert_eq!(fs::read_to_string(&entry).unwrap(), "[Desktop Entry]\nExec=custom\n");
    }
}

#[cfg(windows)]
mod windows {
    use super::*;

    #[test]
    fn creates_a_start_menu_link() {
        let root = tempfile::tempdir().unwrap();
        let exe = fake_exe(&root.path().join("bin"));
        let data = root.path().join("AppData");

        assert_eq!(shortcut(exe, &data).ensure().unwrap(), Outcome::Created);

        let lnk = data.join(r"Microsoft\Windows\Start Menu\Programs\ray-task.lnk");
        assert!(fs::metadata(&lnk).unwrap().len() > 0);
    }
}
