//! See <https://specifications.freedesktop.org/notification-spec/latest/protocol.html>
use crate::BusSender;
use iced::window;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use zbus::object_server::SignalEmitter;
use zbus::{fdo, interface, zvariant};

pub struct NotificationReceiver {
    pub(crate) sender: BusSender,
}

#[derive(Clone)]
pub struct Notification {
    pub id: window::Id,
    pub app_name: Box<str>,
    // TODO: This should be checked beforehand and made into some sort of Update for the notification
    pub replaces_id: u32,
    pub app_icon: Box<str>,
    pub summary: Box<str>,
    pub body: Box<str>,
    pub actions: Vec<Box<str>>,
    pub hints: HashMap<Box<str>, zvariant::OwnedValue>,
    pub expire_timeout: i32,
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

#[derive(Debug, Clone)]
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
        let id = window::Id::unique();
        self.sender
            .send(NotificationMsg::Notification(Notification {
                id,
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
            .expect("Could not send message, UI task may have crashed");
        // Since id does not expose a way to get the inner u64, we need to do this dumb conversion
        // This is a lossy conversion,
        // FIXME: Throw error if it does not fit in u32
        Ok(id.to_string().parse().unwrap())
    }

    pub async fn close_notification(&self, _id: u32) -> fdo::Result<()> {
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
