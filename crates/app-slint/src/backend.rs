use std::ffi::OsStr;

/// Renderer padrão quando o usuário não escolhe um com `SLINT_BACKEND`.
///
/// Software, por causa do orçamento de RAM (spec §1: < 40 MB com 5.000 tarefas): o renderer
/// de GPU sozinho passa de 80 MB; o de software fica em ~39 MB, com visual idêntico.
pub const DEFAULT_BACKEND: &str = "winit-software";

/// Backend a forçar, dado o valor de `SLINT_BACKEND`: nenhum se o usuário definiu um.
pub fn default_backend(env: Option<&OsStr>) -> Option<&'static str> {
    match env {
        Some(v) if !v.is_empty() => None,
        _ => Some(DEFAULT_BACKEND),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn software_unless_user_chose() {
        assert_eq!(default_backend(None), Some("winit-software"));
        assert_eq!(default_backend(Some(OsStr::new(""))), Some("winit-software"));
        assert_eq!(default_backend(Some(OsStr::new("winit-femtovg"))), None);
    }
}
