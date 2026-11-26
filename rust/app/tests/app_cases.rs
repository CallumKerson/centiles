#[test]
fn recorded_cases() {
    trycmd::TestCases::new()
        .register_bin(
            "centiles",
            std::path::Path::new(env!("CARGO_BIN_EXE_centiles")),
        )
        .case("../app-cases/*.toml");
}
