fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let out = std::env::var("OUT_DIR").expect("OUT_DIR");
        let icon = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../packaging/windows/lounge-native.ico");
        println!("cargo:rerun-if-changed={}", icon.display());
        let resource = std::path::Path::new(&out).join("lounge-native.rc");
        std::fs::write(&resource, format!("1 ICON \"{}\"\n", icon.display()))
            .expect("write resource script");
        let _ = embed_resource::compile(&resource, embed_resource::NONE);
    }
}
