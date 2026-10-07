//! gus-combobox is std only: no dependencies of any kind.

/// Every TOML table header in `manifest` that declares dependencies of any kind.
fn dependency_tables(manifest: &str) -> Vec<&str> {
    manifest.lines().map(str::trim).filter(|line| line.starts_with('[') && line.contains("dependencies")).collect()
}

#[test]
fn manifest_has_no_dependencies() {
    assert_eq!(
        dependency_tables(include_str!("../Cargo.toml")),
        Vec::<&str>::new(),
        "gus-combobox must stay std-only, dev-dependencies included"
    );
}

#[test]
fn detector_catches_every_dependency_table_form() {
    let tables =
        ["[dependencies]", "[dev-dependencies]", "[build-dependencies]", "[target.'cfg(unix)'.dependencies]", "[dependencies.foo]"];
    for table in tables {
        assert_eq!(dependency_tables(table), vec![table], "missed {table}");
    }
}
