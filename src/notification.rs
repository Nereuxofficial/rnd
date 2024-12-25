use iced::window;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::time::Instant;
use zbus::zvariant;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Expiry {
    Never,
    Miliseconds(u128),
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Notification {
    #[serde(skip_serializing, deserialize_with = "generate_window_id")]
    pub id: window::Id,
    pub app_name: Box<str>,
    // TODO: This should be checked beforehand and made into some sort of Update for the notification
    pub replaces_id: u32,
    pub app_icon: Box<str>,
    pub summary: Box<str>,
    pub body: Box<str>,
    pub actions: HashMap<Box<str>, Box<str>>,
    pub hints: HashMap<Box<str>, zvariant::OwnedValue>,
    #[serde(skip_serializing, deserialize_with = "generate_new_instant")]
    pub start_time: Instant,
    pub expire_timeout: Expiry,
}

fn generate_new_instant<'de, D>(_de: D) -> Result<Instant, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Instant::now())
}

fn generate_window_id<'de, D>(_de: D) -> Result<window::Id, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(window::Id::unique())
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
