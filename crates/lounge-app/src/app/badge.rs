use std::process::Command;

pub fn update(count: u32) {
    std::thread::spawn(move || {
        let count_visible = if count > 0 { "true" } else { "false" };
        let properties =
            format!("{{'count': <int64 {count}>, 'count-visible': <{count_visible}>}}");
        let _ = Command::new("gdbus")
            .arg("call")
            .arg("--session")
            .arg("--dest")
            .arg("com.canonical.Unity")
            .arg("--object-path")
            .arg("/com/canonical/Unity/LauncherEntry")
            .arg("--method")
            .arg("com.canonical.Unity.LauncherEntry.Update")
            .arg("application://lounge-native.desktop")
            .arg(properties)
            .status();
    });
}
