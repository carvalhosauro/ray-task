fn main() {
    // Debug info do Slint só no perfil debug: os testes de UI usam a API ElementHandle
    // (achar elementos por rótulo, posição real). O release fica igual.
    let debug_profile = std::env::var("PROFILE").is_ok_and(|p| p == "debug");
    let config = slint_build::CompilerConfiguration::new().with_style("cupertino".into()).with_debug_info(debug_profile);
    slint_build::compile_with_config("ui/app.slint", config).expect("falha ao compilar a UI Slint");
}
