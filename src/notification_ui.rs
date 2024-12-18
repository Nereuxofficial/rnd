use crate::notification_receiver::{Notification, NotificationMsg};
use crate::BusReceiver;
use iced::futures::Stream;
use iced::futures::StreamExt;
use iced::theme::{Custom, Palette};
use iced::widget::{column, container, rich_text, row, text};
use iced::widget::{image, Column};
use iced::{event, font, ContentFit, Event, Font, Pixels, Size};
use iced::{gradient, window};
use iced::{Color, Element, Fill, Radians, Theme};
use iced_layershell::build_pattern::{daemon, MainSettings};
use iced_layershell::reexport::{Anchor, Layer, NewLayerShellSettings};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::{to_layer_message, MultiApplication};
use iced_runtime::core::alignment::Horizontal;
use iced_runtime::core::image::Handle;
use iced_runtime::futures::Subscription;
use iced_runtime::Task;
use std::collections::HashMap;
use std::path::PathBuf;
use std::string::ToString;
use std::sync::Arc;
use tokio_stream::wrappers::BroadcastStream;
use tracing::info;
use tracing::log::debug;

pub fn spawn_popup(receiver: BusReceiver) {
    Gradient::run(Settings {
        layer_settings: LayerShellSettings {
            start_mode: StartMode::Background,
            ..Default::default()
        },
        id: Some("main".to_string()),
        flags: Flags {
            bus_receiver: receiver,
        },
        fonts: Vec::new(),
        default_font: Font::default(),
        default_text_size: Pixels(16.0),
        antialiasing: false,
        virtual_keyboard_support: None,
    })
    .unwrap()
}

#[derive(Debug)]
struct Gradient {
    ids: HashMap<window::Id, Notification>,
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
    NewWindow {
        settings: NewLayerShellSettings,
        id: window::Id,
    },
}

struct Flags {
    bus_receiver: BusReceiver,
}

impl MultiApplication for Gradient {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = Flags;
    type Theme = Theme;

    fn new(flags: Self::Flags) -> (Gradient, Task<Message>) {
        (
            Self {
                ids: HashMap::new(),
                receiver: flags.bus_receiver,
                start: Color::new(1.0, 0.0, 0.0, 0.5),
                end: Color::new(0.0, 0.0, 1.0, 0.5),
                angle: Radians(0.0),
            },
            Task::none(),
        )
    }

    fn namespace(&self) -> String {
        "rnd - Rust Notification Daemon".to_string()
    }

    fn remove_id(&mut self, id: window::Id) {
        println!("Should remove {id}");
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Notification(msg) => {
                match msg {
                    // TODO: Insert real ID here
                    NotificationMsg::Notification(n) => {
                        info!("Received notification: {:#?}", n);
                        self.ids.insert(n.id, n.clone());
                        Task::done(Message::NewLayerShell {
                            settings: NewLayerShellSettings {
                                size: Some((500, 200)),
                                anchor: Anchor::Top,
                                layer: Layer::Top,
                                margin: Some((100, 100, 100, 100)),
                                ..Default::default()
                            },
                            id: n.id,
                        })
                    }
                }
            }
            _ => Task::none(),
        }
    }

    fn view(&self, id: window::Id) -> Element<Message> {
        let Self {
            ids,
            start,
            end,
            angle,
            ..
        } = self;
        let mut column = Column::new();
        let gradient_boxes = self
            .ids
            .iter()
            .map(|(id, notification)| NotificationBox::render_notification_box(&notification))
            .collect::<Vec<_>>();
        column = column.extend(gradient_boxes);

        Element::from(column)
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
}

struct NotificationBox;

impl NotificationBox {
    fn render_notification_box<'a>(notification: &'a Notification) -> Element<'a, Message> {
        let start = Color::new(1.0, 0.0, 0.0, 0.5);
        let end = Color::new(0.0, 0.0, 1.0, 0.5);
        let angle = Radians(0.0);
        let image: iced::widget::Image<Handle> = image(PathBuf::from(format!(
            "{}/static/ferris.png",
            env!("CARGO_MANIFEST_DIR")
        )))
        .content_fit(ContentFit::Contain);
        let image: Element<'_, Message, Theme, iced::Renderer> = Element::from(image);

        let row = row![image, text!("{}", notification.summary.as_ref())];

        let text = rich_text!(notification.app_name.as_ref()).font(Font {
            weight: font::Weight::Bold,
            ..Font::default()
        });

        let column = column![text, row].align_x(Horizontal::Center);

        container(column)
            .style(move |_theme| {
                let gradient = gradient::Linear::new(angle)
                    .add_stop(0.0, start)
                    .add_stop(1.0, end);
                gradient.into()
            })
            .width(Fill)
            .height(Fill)
            .into()
    }
}

// Create a stream of messages from the notification receiver
fn receive_messages(recv: BusReceiver) -> impl Stream<Item = NotificationMsg> {
    BroadcastStream::new(recv).map(|msg| msg.unwrap())
}
