#![feature(str_as_str)]

mod image;
mod notification_receiver;
mod notification_ui;

use crate::notification_receiver::{NotificationMsg, NotificationReceiver};
use crate::notification_ui::spawn_popup;
use zbus::connection;

pub type BusReceiver = tokio::sync::broadcast::Receiver<NotificationMsg>;
pub type BusSender = tokio::sync::broadcast::Sender<NotificationMsg>;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let (sender, recv) = tokio::sync::broadcast::channel(64);
    let con = connection::Builder::session()?
        .name("org.freedesktop.Notifications")
        .unwrap()
        .serve_at(
            "/org/freedesktop/Notifications",
            NotificationReceiver { sender },
        )?
        .build()
        .await
        .expect(
            "Could not register notification daemon. Try to kill your running notification daemon.",
        );
    spawn_popup(recv);
    Ok(())
}
