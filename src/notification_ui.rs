use crate::notification_receiver::{NotificationMsg, NotificationReceiver};
use crate::BusReceiver;
use iced::futures::Stream;
use iced::futures::StreamExt;
use iced::theme::{Custom, Palette};
use iced::widget::{column, container, row};
use iced::widget::{image, Column};
use iced::{event, ContentFit, Event, Size};
use iced::{gradient, window};
use iced::{Color, Element, Fill, Radians, Theme};
use iced_layershell::build_pattern::{daemon, MainSettings};
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::{LayerShellSettings, StartMode};
use iced_layershell::to_layer_message;
use iced_runtime::futures::subscription::{EventStream, Hasher, Recipe};
use iced_runtime::futures::{BoxStream, Subscription};
use iced_runtime::Task;
use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;
use tokio_stream::wrappers::{BroadcastStream, ReceiverStream};

pub fn spawn_popup(receiver: BusReceiver) {
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
    .run_with(|| (Gradient::new(receiver), Task::none()))
    .expect("TODO: panic message");
}

#[derive(Debug)]
struct Gradient {
    ids: HashMap<window::Id, ()>,
    // Has to be Option, since the Receiver stream needs to take ownership of it.
    receiver: BusReceiver,
    start: Color,
    end: Color,
    angle: Radians,
}

#[to_layer_message(multi)]
#[derive(Debug, Clone)]
enum Message {
    IcedEvent(Event),
    Notification(NotificationMsg),
}

impl Gradient {
    fn new(receiver: BusReceiver) -> Self {
        Self {
            ids: HashMap::new(),
            receiver,
            start: Color::new(1.0, 0.0, 0.0, 0.5),
            end: Color::new(0.0, 0.0, 1.0, 0.5),
            angle: Radians(0.0),
        }
    }

    fn namespace(&self) -> String {
        "rnd - Rust Notification Daemon".to_string()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::IcedEvent(e) => {}
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
            Message::Notification(msg) => {
                println!("Received notification in UI: {:#?}", msg);
                match msg {
                    // TODO: Insert real ID here
                    NotificationMsg::Notification(n) => self.ids.insert(n.id, ()).unwrap(),
                }
            }
        }
        Task::none()
    }

    fn view(&self, id: iced::window::Id) -> Element<Message> {
        let Self {
            ids,
            start,
            end,
            angle,
            ..
        } = self;
        let gradient_boxes = self
            .ids
            .iter()
            .map(|_| {
                let image = Element::from(
                    image(format!("{}/static/ferris.png", env!("CARGO_MANIFEST_DIR")))
                        .content_fit(ContentFit::Contain),
                );

                let content = row![image, "Hello World!"];

                container(content)
                    .style(move |_theme| {
                        let gradient = gradient::Linear::new(*angle)
                            .add_stop(0.0, *start)
                            .add_stop(1.0, *end);
                        gradient.into()
                    })
                    .width(Fill)
                    .height(Fill)
                    .into()
            })
            .collect::<Vec<_>>();
        Element::from(Column::from_vec(gradient_boxes).into())
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            event::listen().map(Message::IcedEvent),
            Subscription::run_with_id(
                1,
                receive_messages(self.receiver.resubscribe()).map(Message::Notification),
            ),
        ])
    }

    fn remove_id(&mut self, id: window::Id) {
        println!("Should remove {id}");
    }
}

struct BusStream(BroadcastStream<NotificationMsg>);

// Create a stream of messages from the notification receiver
fn receive_messages(recv: BusReceiver) -> impl Stream<Item = NotificationMsg> {
    BroadcastStream::new(recv).map(|msg| msg.unwrap())
}
