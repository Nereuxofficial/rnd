mod notification_receiver;

use crate::notification_receiver::{NotificationMsg, NotificationReceiver};
use iced::theme::{Custom, Palette};
use iced::widget::image;
use iced::widget::{column, container, row};
use iced::{event, ContentFit, Event, Size};
use iced::{gradient, window};
use iced::{Color, Element, Fill, Radians, Theme};
use iced_layershell::build_pattern::{daemon, MainSettings};
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::{LayerShellSettings, StartMode};
use iced_layershell::to_layer_message;
use iced_runtime::Task;
use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use zbus::connection;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let (sender, mut recv) = tokio::sync::mpsc::channel(5);
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

    loop {
        if let Some(msg) = recv.recv().await {
            handle_msg(msg)
        }
    }
    Ok(())
}

fn handle_msg(notification_msg: NotificationMsg) {
    println!("Received {:?}", notification_msg);
    spawn_popup()
}

fn spawn_popup() {
    daemon(
        Gradient::namespace,
        Gradient::update,
        Gradient::view,
        Gradient::remove_id,
    )
    .subscription(Gradient::subscription)
    .settings(MainSettings {
        layer_settings: LayerShellSettings {
            size: Some((600, 300)),
            anchor: Anchor::Top,
            start_mode: StartMode::Active,
            margin: (10, 10, 10, 10),
            ..Default::default()
        },
        ..Default::default()
    })
    .theme(|_| {
        Theme::Custom(Arc::new(Custom::new(
            "Transparency".to_string(),
            Palette {
                background: Color::new(0.1, 0.1, 0.1, 0.7),
                text: Default::default(),
                primary: Default::default(),
                success: Default::default(),
                danger: Default::default(),
            },
        )))
    })
    .run_with(|| (Gradient::new(), Task::none()))
    .expect("TODO: panic message");
}

#[derive(Debug, Clone)]
struct Gradient {
    ids: HashMap<window::Id, ()>,
    start: Color,
    end: Color,
    angle: Radians,
    transparent: bool,
}

#[to_layer_message(multi)]
#[derive(Debug, Clone)]
enum Message {
    Click,
    IcedEvent(Event),
}

impl Gradient {
    fn new() -> Self {
        Self {
            ids: HashMap::new(),
            start: Color::new(1.0, 0.0, 0.0, 0.5),
            end: Color::new(0.0, 0.0, 1.0, 0.5),
            angle: Radians(0.0),
            transparent: true,
        }
    }

    fn namespace(&self) -> String {
        "rnd - Rust Notification Daemon".to_string()
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Click => {}
            Message::IcedEvent(e) => println!("{e:?}"),
            Message::AnchorChange { .. } => {}
            Message::AnchorSizeChange { .. } => {}
            Message::LayerChange { .. } => {}
            Message::MarginChange { .. } => {}
            Message::SizeChange { .. } => {}
            Message::VirtualKeyboardPressed { .. } => {}
            Message::NewLayerShell { .. } => {}
            Message::NewPopUp { .. } => {}
            Message::NewMenu { .. } => {}
            Message::RemoveWindow(_) => {}
            Message::ForgetLastOutput => {}
        }
    }

    fn view(&self, id: iced::window::Id) -> Element<Message> {
        let Self {
            ids,
            start,
            end,
            angle,
            transparent,
        } = self.clone();
        let image = Element::from(
            image(format!("{}/static/ferris.png", env!("CARGO_MANIFEST_DIR")))
                .content_fit(ContentFit::Contain),
        );

        let content = row![image, "Hello World!"];

        let gradient_box = container(content)
            .style(move |_theme| {
                let gradient = gradient::Linear::new(angle)
                    .add_stop(0.0, start)
                    .add_stop(1.0, end);
                gradient.into()
            })
            .width(Fill)
            .height(Fill);

        column![gradient_box,].into()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        event::listen().map(Message::IcedEvent)
    }

    fn remove_id(&mut self, id: iced::window::Id) {
        println!("Should remove {id}");
    }
}

impl Default for Gradient {
    fn default() -> Self {
        Self::new()
    }
}
