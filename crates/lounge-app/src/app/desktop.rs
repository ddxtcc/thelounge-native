use futures::StreamExt;

use super::*;

impl LoungeApp {
    pub(super) fn watch_notifications(
        &mut self,
        mut receiver: futures::channel::mpsc::UnboundedReceiver<i64>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this, cx| {
            while let Some(channel) = receiver.next().await {
                let _ = this.update(cx, |this, cx| {
                    this.pending_channel_switch = Some(channel);
                    this.pending_show_window = true;
                    cx.notify();
                });
            }
        })
        .detach();
    }

    #[cfg(target_os = "linux")]
    pub(super) fn watch_tray(
        &mut self,
        mut receiver: futures::channel::mpsc::UnboundedReceiver<tray::TrayAction>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this, cx| {
            while let Some(action) = receiver.next().await {
                let _ = this.update(cx, |this, cx| match action {
                    tray::TrayAction::Show => {
                        this.pending_show_window = true;
                        cx.notify();
                    }
                    tray::TrayAction::Quit => cx.quit(),
                });
            }
        })
        .detach();
    }

    #[cfg(target_os = "windows")]
    pub(super) fn watch_tray(
        &mut self,
        mut receiver: futures::channel::mpsc::UnboundedReceiver<tray_windows::TrayAction>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this, cx| {
            while let Some(action) = receiver.next().await {
                let _ = this.update(cx, |this, cx| match action {
                    tray_windows::TrayAction::Show => {
                        this.pending_show_window = true;
                        cx.notify();
                    }
                    tray_windows::TrayAction::Quit => cx.quit(),
                });
            }
        })
        .detach();
    }

    pub(super) fn set_tray_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.config.show_tray_icon = enabled;
        self.config.save();
        #[cfg(target_os = "linux")]
        {
            if enabled {
                if self.tray_handle.is_none()
                    && let Some((handle, receiver)) = crate::app::tray::spawn()
                {
                    let unread = self.state.total_unread();
                    handle.update(|tray| tray.unread = unread);
                    self.tray_handle = Some(handle);
                    self.watch_tray(receiver, cx);
                }
            } else if let Some(handle) = self.tray_handle.take() {
                handle.shutdown().wait();
            }
        }
        #[cfg(target_os = "windows")]
        {
            if enabled {
                if self.tray_handle.is_none()
                    && let Some((handle, receiver)) = crate::app::tray_windows::spawn()
                {
                    handle.set_unread(self.state.total_unread());
                    self.tray_handle = Some(handle);
                    self.watch_tray(receiver, cx);
                }
            } else if let Some(handle) = self.tray_handle.take() {
                handle.stop();
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = enabled;
            let _ = cx;
        }
        cx.notify();
    }

    pub(super) fn set_global_hotkey_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            use global_hotkey::hotkey::HotKey;
            self.config.enable_global_hotkey = enabled;
            self.config.save();
            if enabled {
                let spec = self.config.global_hotkey.clone();
                self.set_global_hotkey(&spec, cx);
                return;
            }
            if let Some(manager) = self.hotkey_manager.take()
                && let Ok(hotkey) = self.config.global_hotkey.parse::<HotKey>()
            {
                let _ = manager.unregister(hotkey);
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = enabled;
            let _ = cx;
        }
        cx.notify();
    }

    pub(super) fn open_hotkey_palette(&mut self, return_to_settings: bool, cx: &mut Context<Self>) {
        self.hotkey_settings_return = return_to_settings;
        self.open_palette(PaletteMode::Hotkey, cx);
    }

    pub(super) fn set_global_hotkey(&mut self, spec: &str, cx: &mut Context<Self>) -> bool {
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = spec;
            let _ = cx;
            self.toast("Global shortcuts are not available on this platform.");
            return false;
        }
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            use global_hotkey::{
                GlobalHotKeyManager,
                hotkey::{HotKey, Modifiers},
            };
            let Ok(hotkey) = spec.parse::<HotKey>() else {
                self.toast("Invalid shortcut. Try control+alt+l.");
                return false;
            };
            let primary = hotkey.mods.contains(Modifiers::CONTROL)
                || hotkey.mods.contains(Modifiers::ALT)
                || hotkey.mods.contains(Modifiers::META);
            if !primary {
                self.toast("Include Ctrl, Alt, or Meta.");
                return false;
            }
            let old = self.config.global_hotkey.parse::<HotKey>().ok();
            let manager = match self.hotkey_manager.take() {
                Some(manager) => manager,
                None => match GlobalHotKeyManager::new() {
                    Ok(manager) => manager,
                    Err(error) => {
                        self.toast(format!("Could not enable global shortcut: {error}"));
                        return false;
                    }
                },
            };
            if let Some(old) = old {
                let _ = manager.unregister(old);
            }
            if let Err(error) = manager.register(hotkey) {
                if let Some(old) = old {
                    let _ = manager.register(old);
                }
                self.hotkey_manager = Some(manager);
                self.toast(format!("Shortcut unavailable: {error}"));
                return false;
            }
            self.hotkey_manager = Some(manager);
            self.config.global_hotkey = hotkey.into_string();
            self.config.enable_global_hotkey = true;
            self.config.save();
            self.toast(format!("Global shortcut: {}", self.config.global_hotkey));
            cx.notify();
            true
        }
    }
}
