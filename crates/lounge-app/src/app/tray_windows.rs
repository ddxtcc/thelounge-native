use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use tray_icon::{
    Icon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem},
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
};

#[derive(Clone, Copy, Debug)]
pub enum TrayAction {
    Show,
    Quit,
}

enum Command {
    Unread(u32),
    Stop,
}

pub struct TrayHandle {
    commands: mpsc::Sender<Command>,
}

impl TrayHandle {
    pub fn set_unread(&self, unread: u32) {
        let _ = self.commands.send(Command::Unread(unread));
    }

    pub fn stop(&self) {
        let _ = self.commands.send(Command::Stop);
    }
}

fn tray_icon() -> Option<Icon> {
    let width = 16u32;
    let height = 16u32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let inside = (4..12).contains(&x) && (3..13).contains(&y);
            let dot = (6..10).contains(&x) && (6..10).contains(&y);
            let color = if dot {
                [255, 255, 255, 255]
            } else if inside {
                [122, 162, 247, 255]
            } else {
                [0, 0, 0, 0]
            };
            rgba.extend_from_slice(&color);
        }
    }
    Icon::from_rgba(rgba, width, height).ok()
}

pub fn spawn() -> Option<(TrayHandle, mpsc::Receiver<TrayAction>)> {
    let (actions, action_rx) = mpsc::channel();
    let (commands, command_rx) = mpsc::channel();
    let (ready, ready_rx) = mpsc::channel();

    thread::spawn(move || {
        let icon = match tray_icon() {
            Some(icon) => icon,
            None => {
                let _ = ready.send(false);
                return;
            }
        };
        let menu = Menu::new();
        let show = MenuItem::with_id("show", "Show Lounge Native", true, None);
        let quit = MenuItem::with_id("quit", "Quit", true, None);
        if menu.append(&show).is_err() || menu.append(&quit).is_err() {
            let _ = ready.send(false);
            return;
        }
        let tray = match TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(true)
            .with_tooltip("Lounge Native")
            .with_icon(icon)
            .build()
        {
            Ok(tray) => tray,
            Err(_) => {
                let _ = ready.send(false);
                return;
            }
        };
        let _ = ready.send(true);

        let mut message = MSG::default();
        loop {
            while let Ok(event) = MenuEvent::receiver().try_recv() {
                if event.id.0 == "show" {
                    let _ = actions.send(TrayAction::Show);
                } else if event.id.0 == "quit" {
                    let _ = actions.send(TrayAction::Quit);
                }
            }
            while let Ok(event) = TrayIconEvent::receiver().try_recv() {
                if let TrayIconEvent::DoubleClick { .. } = event {
                    let _ = actions.send(TrayAction::Show);
                }
            }
            while let Ok(command) = command_rx.try_recv() {
                match command {
                    Command::Unread(unread) => {
                        let tooltip = if unread > 0 {
                            format!("Lounge Native ({unread})")
                        } else {
                            "Lounge Native".to_string()
                        };
                        let _ = tray.set_tooltip(Some(tooltip));
                    }
                    Command::Stop => return,
                }
            }
            unsafe {
                while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            thread::sleep(Duration::from_millis(50));
        }
    });

    if ready_rx.recv_timeout(Duration::from_secs(2)).ok()? {
        Some((TrayHandle { commands }, action_rx))
    } else {
        None
    }
}
