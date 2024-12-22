mod image;
mod notification_receiver;
mod notification_ui;

use crate::notification_receiver::{NotificationMsg, NotificationReceiver};
use crate::notification_ui::spawn_popup;
use color_eyre::Result;
use zbus::connection;

pub type BusReceiver = tokio::sync::broadcast::Receiver<NotificationMsg>;
pub type BusSender = tokio::sync::broadcast::Sender<NotificationMsg>;

#[tokio::main]
pub async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;
    let (sender, recv) = tokio::sync::broadcast::channel(64);
    let dbus_service = NotificationReceiver { sender };
    let con = connection::Builder::session()?
        .name("org.freedesktop.Notifications")
        .unwrap()
        .serve_at("/org/freedesktop/Notifications", dbus_service)?
        .build()
        .await
        .expect(
            "Could not register notification daemon. Try to kill your running notification daemon.",
        );
    spawn_popup(
        recv,
        con.object_server()
            .interface("/org/freedesktop/Notifications")
            .await?,
    );
    Ok(())
}
