//! gus-list is std only: no runtime dependencies, ever.

#[test]
fn manifest_has_no_dependencies() {
    let manifest = include_str!("../Cargo.toml");
    assert!(!manifest.contains("[dependencies]"), "gus-list must stay std-only");
    assert!(!manifest.contains("[dev-dependencies]"), "keep gus-list builds light: no dev-dependencies either");
}
