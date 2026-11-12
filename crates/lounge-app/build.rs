fn main() {
    #[cfg(windows)]
    {
        let icon = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../packaging/windows/lounge-native.ico");
        let mut resource = winres::WindowsResource::new();
        resource.set_icon(icon.to_str().expect("icon path"));
        resource.set("ProductName", "Lounge Native");
        resource.set("FileDescription", "Lounge Native");
        resource.set("OriginalFilename", "lounge-native-app.exe");
        resource.compile().expect("embed Windows resources");
    }
}
