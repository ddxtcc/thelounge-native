#![cfg(target_os = "linux")]

use std::sync::mpsc;

use ksni::{
    Category, Status, Tray,
    blocking::{Handle, TrayMethods},
    menu::{MenuItem, StandardItem},
};

#[derive(Clone, Copy, Debug)]
pub enum TrayAction {
    Show,
    Quit,
}

pub struct LoungeTray {
    actions: mpsc::Sender<TrayAction>,
    pub unread: u32,
}

impl Tray for LoungeTray {
    fn id(&self) -> String {
        "lounge-native".into()
    }

    fn category(&self) -> Category {
        Category::Communications
    }

    fn title(&self) -> String {
        if self.unread > 0 {
            format!("Lounge Native ({})", self.unread)
        } else {
            "Lounge Native".into()
        }
    }

    fn status(&self) -> Status {
        if self.unread > 0 {
            Status::NeedsAttention
        } else {
            Status::Active
        }
    }

    fn icon_name(&self) -> String {
        "lounge-native".into()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.actions.send(TrayAction::Show);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Show Lounge Native".into(),
                activate: Box::new(|this: &mut LoungeTray| {
                    let _ = this.actions.send(TrayAction::Show);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|this: &mut LoungeTray| {
                    let _ = this.actions.send(TrayAction::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub fn spawn() -> Option<(Handle<LoungeTray>, mpsc::Receiver<TrayAction>)> {
    let (tx, rx) = mpsc::channel();
    let handle = LoungeTray {
        actions: tx,
        unread: 0,
    }
    .spawn()
    .ok()?;
    Some((handle, rx))
}
