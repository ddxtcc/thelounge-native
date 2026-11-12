fn main() {
    #[cfg(windows)]
    {
        let icon = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../packaging/windows/lounge-native.ico");
        let mut resource = winres::WindowsResource::new();
        resource.set_icon(icon.to_str().expect("icon path"));
        resource.set("ProductName", "Lounge Native");
        resource.set("FileDescription", "Lounge Native launcher");
        resource.set("OriginalFilename", "lounge-native.exe");
        resource.compile().expect("embed Windows resources");
    }
}
