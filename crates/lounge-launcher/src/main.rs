#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod windows {
    use std::ffi::{OsStr, c_void};
    use std::path::{Path, PathBuf};
    use std::process::Command;

    const GIO_MODULES: &str = "lib\\gio\\modules";
    const PLUGIN_DIR: &str = "lib\\gstreamer-1.0";

    pub fn run() {
        let Some(root) = gstreamer_root() else {
            message(
                "Lounge Native needs the GStreamer runtime for audio and video.\n\n\
                 Install the GStreamer 1.26.7 MSVC x86_64 runtime from\n\
                 https://gstreamer.freedesktop.org/download/#windows and run\n\
                 lounge-native.exe again.",
            );
            std::process::exit(1);
        };
        configure(&root);

        let Some(launcher) = std::env::current_exe().ok() else {
            message("Could not locate the Lounge Native application directory.");
            std::process::exit(1);
        };
        let Some(dir) = launcher.parent() else {
            message("Could not locate the Lounge Native application directory.");
            std::process::exit(1);
        };
        let app = dir.join("lounge-native-app.exe");
        if !app.is_file() {
            message("Could not find lounge-native-app.exe next to the launcher.");
            std::process::exit(1);
        }
        let mut command = Command::new(&app);
        command.args(std::env::args_os().skip(1));
        if let Some(parent) = app.parent() {
            command.current_dir(parent);
        }
        if let Err(error) = command.spawn() {
            message(&format!("Could not start Lounge Native:\n{error}"));
            std::process::exit(1);
        }
    }

    fn configure(root: &Path) {
        let bin = root.join("bin");
        let mut paths = vec![bin];
        if let Some(path) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&path));
        }
        if let Ok(path) = std::env::join_paths(paths) {
            unsafe {
                std::env::set_var("PATH", path);
            }
        }

        let plugins = root.join(PLUGIN_DIR);
        if plugins.is_dir() {
            unsafe {
                std::env::set_var("GST_PLUGIN_SYSTEM_PATH", &plugins);
                std::env::set_var("GST_PLUGIN_SYSTEM_PATH_1_0", &plugins);
                std::env::set_var("GST_PLUGIN_PATH", &plugins);
                std::env::set_var("GST_PLUGIN_PATH_1_0", &plugins);
            }
        }

        if let Some(scanner) = find_file(root, OsStr::new("gst-plugin-scanner.exe")) {
            unsafe {
                std::env::set_var("GST_PLUGIN_SCANNER", &scanner);
                std::env::set_var("GST_PLUGIN_SCANNER_1_0", &scanner);
            }
        }

        let gio = root.join(GIO_MODULES);
        if gio.is_dir() {
            unsafe {
                std::env::set_var("GIO_EXTRA_MODULES", &gio);
            }
        }
    }

    fn gstreamer_root() -> Option<PathBuf> {
        let mut candidates = Vec::new();
        for name in [
            "GSTREAMER_1_0_ROOT_MSVC_X86_64",
            "GSTREAMER_ROOT_X86_64",
            "GSTREAMER_ROOT",
            "GSTREAMER_1_0_ROOT_MINGW_X86_64",
        ] {
            if let Some(root) = std::env::var_os(name) {
                candidates.push(PathBuf::from(root));
            }
        }
        candidates.extend([
            PathBuf::from(r"C:\gstreamer\1.0\msvc_x86_64"),
            PathBuf::from(r"C:\Program Files\gstreamer\1.0\msvc_x86_64"),
            PathBuf::from(r"C:\Program Files (x86)\gstreamer\1.0\msvc_x86_64"),
        ]);
        candidates.into_iter().find(|root| {
            root.join("bin").join("gstreamer-1.0-0.dll").is_file()
                || root.join("bin").join("gstreamer-1.0.dll").is_file()
        })
    }

    fn find_file(root: &Path, name: &OsStr) -> Option<PathBuf> {
        let mut pending = vec![(root.to_path_buf(), 0usize)];
        while let Some((dir, depth)) = pending.pop() {
            if depth > 4 {
                continue;
            }
            let entries = std::fs::read_dir(&dir).ok()?;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.file_name() == Some(name) {
                    return Some(path);
                }
                if path.is_dir() {
                    pending.push((path, depth + 1));
                }
            }
        }
        None
    }

    fn message(text: &str) {
        let title = wide("Lounge Native");
        let body = wide(text);
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                body.as_ptr(),
                title.as_ptr(),
                0x0000_0010,
            );
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(hwnd: *mut c_void, text: *const u16, caption: *const u16, utype: u32)
        -> i32;
    }
}

#[cfg(windows)]
fn main() {
    windows::run();
}

#[cfg(not(windows))]
fn main() {}
