use crate::image::Image;
use crate::notification_receiver::{Expiry, Notification, NotificationMsg};
use crate::BusReceiver;
use iced::futures::Stream;
use iced::futures::StreamExt;
use iced::widget::image;
use iced::widget::{column, container, rich_text, text, Container, Row};
use iced::{event, font, ContentFit, Event, Font, Pixels, Shrink};
use iced::{gradient, window};
use iced::{Color, Element, Fill, Radians, Theme};
use iced_layershell::reexport::{Anchor, Layer, NewLayerShellSettings};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::{to_layer_message, MultiApplication};
use iced_runtime::core::alignment::Horizontal;
use iced_runtime::core::image::Handle;
use iced_runtime::futures::Subscription;
use iced_runtime::window::Action as WindowAction;
use iced_runtime::{Action, Task};
use std::collections::HashMap;
use std::path::PathBuf;
use std::string::ToString;
use std::task::Poll;
use tokio::time::Instant;
use tokio_stream::wrappers::BroadcastStream;
use tracing::info;
use zbus::zvariant::OwnedValue;

const HEIGHT: u32 = 150;
const TICK_LENGTH: u128 = 100;

pub fn spawn_popup(receiver: BusReceiver) {
    NotificationUi::run(Settings {
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
struct NotificationUi {
    ids: HashMap<window::Id, Notification>,
    // Has to be Option, since the Receiver stream needs to take ownership of it.
    receiver: BusReceiver,
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
    CloseWindow(window::Id),
    TickElapsed,
}

struct Flags {
    bus_receiver: BusReceiver,
}

impl MultiApplication for NotificationUi {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = Flags;
    type Theme = Theme;

    fn new(flags: Self::Flags) -> (NotificationUi, Task<Message>) {
        (
            Self {
                ids: HashMap::new(),
                receiver: flags.bus_receiver,
            },
            Task::none(),
        )
    }

    fn namespace(&self) -> String {
        "rnd".to_string()
    }

    fn remove_id(&mut self, id: window::Id) {
        info!("Removing id: {}", id);
        self.ids.remove(&id);
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RemoveWindow(id) => {
                iced_runtime::task::effect(Action::Window(WindowAction::Close(id)))
            }
            Message::CloseWindow(id) => {
                self.remove_id(id);
                iced_runtime::task::effect(Action::Window(WindowAction::Close(id)))
            }
            Message::Notification(msg) => {
                match msg {
                    // TODO: Insert real ID here
                    NotificationMsg::Notification(n) => {
                        info!("Received notification: {:#?}", n);
                        self.ids.insert(n.id, n.clone());
                        Task::done(Message::NewLayerShell {
                            settings: NewLayerShellSettings {
                                size: Some((400, HEIGHT)),
                                anchor: Anchor::Top,
                                layer: Layer::Top,
                                margin: Some((
                                    HEIGHT as i32 * self.ids.len() as i32,
                                    100,
                                    100,
                                    100,
                                )),
                                ..Default::default()
                            },
                            id: n.id,
                        })
                    }
                }
            }
            Message::TickElapsed => {
                let mut tasks = vec![];
                self.ids.retain(|id, n| match n.expire_timeout {
                    Expiry::Never => true,
                    Expiry::Miliseconds(ms) => {
                        if Instant::now().duration_since(n.start_time).as_millis() > ms {
                            info!(
                                "Removing notification: {}: {} due to timeout of {}ms",
                                n.app_name, n.summary, ms
                            );
                            tasks.push(iced_runtime::task::effect(Action::Window(
                                WindowAction::Close(*id),
                            )));
                            false
                        } else {
                            true
                        }
                    }
                });
                Task::batch(tasks)
            }
            _ => Task::none(),
        }
    }

    fn view(&self, id: window::Id) -> Element<Message> {
        let Self { ids, .. } = self;

        let notification_box = ids
            .get(&id)
            .map(|notification| NotificationBox::render_notification_box(notification))
            .unwrap_or_else(|| {
                info!("Rendering: Notification {} not found", id);
                column![].into()
            });

        Element::from(column![notification_box])
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            event::listen().map(Message::IcedEvent),
            Subscription::run(|| {
                DelayStream {
                    start: Instant::now(),
                    time_between: TICK_LENGTH,
                }
                .map(|_| Message::TickElapsed)
            }),
            Subscription::run_with_id(
                1,
                receive_messages(self.receiver.resubscribe()).map(Message::Notification),
            ),
        ])
    }
}

struct DelayStream {
    start: Instant,
    time_between: u128,
}

impl Stream for DelayStream {
    type Item = ();

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if Instant::now().duration_since(self.start).as_millis() >= self.time_between {
            self.get_mut().start = Instant::now();
            Poll::Ready(Some(()))
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

struct NotificationBox;

impl NotificationBox {
    fn get_image(notification: &Notification) -> Option<iced::widget::Image<Handle>> {
        // TODO: Handle Order according to spec https://specifications.freedesktop.org/notification-spec/latest/icons-and-images.html
        let res: Option<&OwnedValue> = notification
            .hints
            .get("image-data")
            .or(notification.hints.get("icon_data"));
        if let Some(owned_val) = res {
            let processed_img = Image::try_from(owned_val.try_clone().unwrap()).unwrap();
            let image = image(Handle::from_rgba(
                processed_img.width as u32,
                processed_img.height as u32,
                processed_img.pixels,
            ))
            .width(processed_img.width as f32)
            .height(processed_img.height as f32)
            .content_fit(ContentFit::Contain);
            Some(image)
        } else if !notification.app_icon.is_empty() {
            // TODO: Is there a try_from because i think this crashes if the path is invalid
            let image = image(PathBuf::from(notification.app_icon.as_ref()));
            Some(image)
        } else {
            None
        }
    }
    fn render_notification_box(notification: &Notification) -> Element<Message> {
        let end = Color::new(0.05, 0.05, 0.05, 1.0);
        let start = Color::new(0.20, 0.20, 0.20, 1.0);
        let angle = Radians(0.0);
        let mut row = Row::new();
        row = row.push_maybe(
            Self::get_image(notification)
                .map(Container::new)
                .map(|c| c.max_width(150)),
        );

        let text_column = column![
            text!("{}", notification.summary.as_ref()).font(Font {
                weight: font::Weight::Bold,
                ..Font::default()
            }),
            text!("{}", notification.body.as_ref()).size(12)
        ]
        .align_x(Horizontal::Center);
        row = row.push(text_column);

        let text = rich_text!(notification.app_name.as_ref())
            .font(Font {
                weight: font::Weight::Light,
                ..Font::default()
            })
            .align_x(Horizontal::Center);

        let column = column![row, text];

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
