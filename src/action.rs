use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    #[serde(rename = "View")]
    View,
    Unsupported(Box<str>),
}
