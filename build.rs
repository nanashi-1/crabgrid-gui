fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_style("fluent".to_string());
    slint_build::compile_with_config("ui/appwindow.slint", config).unwrap();

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon.ico");
        let _ = res.compile();
    }
}
