use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use tokio::sync::mpsc::Sender;
use zbus::object_server::SignalEmitter;
use zbus::{connection, fdo, interface, zvariant};

pub struct NotificationReceiver {
    pub(crate) sender: Sender<NotificationMsg>,
}

pub struct Notification {
    app_name: Box<str>,
    replaces_id: u32,
    app_icon: Box<str>,
    summary: Box<str>,
    body: Box<str>,
    actions: Vec<Box<str>>,
    hints: HashMap<Box<str>, zvariant::OwnedValue>,
    expire_timeout: i32,
}

impl Debug for Notification {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Notification")
            .field("app_name", &self.app_name)
            .field("replaces_id", &self.replaces_id)
            .field("app_icon", &self.app_icon)
            .field("summary", &self.summary)
            .field("body", &self.body)
            .field("actions", &self.actions)
            .field("hints", &self.hints.keys())
            .field("expire_timeout", &self.expire_timeout)
            .finish()
    }
}

#[derive(Debug)]
pub enum NotificationMsg {
    Notification(Notification),
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationReceiver {
    #[allow(clippy::too_many_arguments)]
    pub async fn notify(
        &mut self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<&str>,
        hints: HashMap<&str, zvariant::Value<'_>>,
        expire_timeout: i32,
    ) -> fdo::Result<u32> {
        println!("Received notification from {app_name} with content {summary}");
        self.sender
            .send(NotificationMsg::Notification(Notification {
                app_name: Box::from(app_name),
                replaces_id,
                app_icon: Box::from(app_icon),
                summary: Box::from(summary),
                body: Box::from(body),
                actions: actions.iter().map(|&s| Box::from(s)).collect(),
                hints: hints
                    .into_iter()
                    .map(|(s, val)| (Box::from(s), val.try_to_owned().unwrap()))
                    .collect(),
                expire_timeout,
            }))
            .await
            .expect("Could n ot send message, UI thread may have crashed");
        // TODO: Give back ID of notification. See https://specifications.freedesktop.org/notification-spec/latest/protocol.html
        Ok(55)
    }

    pub async fn close_notification(&self, id: u32) -> fdo::Result<()> {
        Ok(())
    }

    pub fn get_capabilities(&self) -> Vec<String> {
        println!("Get capabilities requested!");
        vec!["body".to_string(), "actions".to_string()]
    }
    pub fn get_server_information(&self) -> fdo::Result<(String, String, String, String)> {
        Ok((
            "NotificationDaemon".to_string(),
            "1.0".to_string(),
            "rnd".to_string(),
            "1.0".to_string(),
        ))
    }

    pub async fn update_history(&self) -> fdo::Result<()> {
        Ok(())
    }

    pub async fn open_history(&self) -> fdo::Result<()> {
        println!("Getting history");
        Ok(())
    }

    pub async fn close_history(&self) -> fdo::Result<()> {
        println!("Closing history");
        Ok(())
    }

    pub async fn toggle_history(&self) -> fdo::Result<()> {
        println!("Toggling history");
        Ok(())
    }

    pub async fn reply_close(&self, id: u32) -> fdo::Result<()> {
        println!("Closing reply window with id {id}");
        Ok(())
    }

    #[zbus(signal)]
    pub async fn action_invoked(
        ctx: &SignalEmitter<'_>,
        id: u32,
        action_key: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn notification_closed(
        ctx: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn notification_replied(
        ctx: &SignalEmitter<'_>,
        id: u32,
        message: &str,
    ) -> zbus::Result<()>;
}
