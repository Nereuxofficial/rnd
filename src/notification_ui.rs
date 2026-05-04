use crate::image::Image;
use crate::notification::Expiry;
use crate::notification::Notification;
use crate::notification_receiver::{
    NotificationMsg, NotificationReceiver, NotificationReceiverSignals,
};
use crate::BusSender;
use iced::alignment::Vertical;
use iced::border;
use iced::border::Radius;
use iced::futures::Stream;
use iced::futures::StreamExt;
use iced::widget::image;
use iced::widget::progress_bar;
use iced::widget::{column, container, text, Button, Container, Row};
use iced::Background;
use iced::Border;
use iced::Length;
use iced::Padding;
use iced::Point;
use iced::Size;
use iced::{event, font, ContentFit, Event, Font};
use iced::{gradient, window};
use iced::{Color, Element, Fill, Radians};
use iced_layershell::daemon;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings};
use iced_layershell::settings::{LayerShellSettings, StartMode};
use iced_layershell::to_layer_message;
use iced_runtime::core::alignment::Horizontal;
use iced_runtime::core::image::Handle;
use iced_runtime::futures::Subscription;
use iced_runtime::window::Action as WindowAction;
use iced_runtime::{Action, Task};
use std::collections::HashMap;
use std::path::PathBuf;
use std::string::ToString;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use tokio::sync::broadcast::Sender as BroadcastSender;
use tokio::time::Instant;
use tokio_stream::wrappers::BroadcastStream;

struct HashableSender(BroadcastSender<NotificationMsg>);

impl std::hash::Hash for HashableSender {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        1u64.hash(state);
    }
}
use tracing::info;
use zbus::object_server::InterfaceRef;
use zbus::zvariant::OwnedValue;

const HEIGHT: u32 = 150;
const TICK_LENGTH: u128 = 100;

pub fn spawn_popup(bus_sender: BusSender, reply_handle: InterfaceRef<NotificationReceiver>) {
    let bus_sender = Arc::new(Mutex::new(Some(bus_sender)));

    daemon(
        move || {
            let sender = bus_sender
                .lock()
                .unwrap()
                .take()
                .expect("boot called twice");
            (
                NotificationUi {
                    ids: HashMap::new(),
                    sender,
                    reply_handle: reply_handle.clone(),
                },
                Task::none(),
            )
        },
        "rnd",
        NotificationUi::update,
        NotificationUi::view,
    )
    .style(|_, _| iced::theme::Style {
        background_color: Color::TRANSPARENT,
        text_color: Color::WHITE,
    })
    .subscription(NotificationUi::subscription)
    .layer_settings(LayerShellSettings {
        start_mode: StartMode::Background,
        ..Default::default()
    })
    .run()
    .unwrap()
}

struct NotificationUi {
    ids: HashMap<window::Id, Notification>,
    sender: BusSender,
    reply_handle: InterfaceRef<NotificationReceiver>,
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
    ActionInvocation {
        id: window::Id,
        action: String,
    },
    CloseWindow(window::Id),
    TickElapsed,
}

impl NotificationUi {
    fn remove_id(&mut self, id: window::Id) {
        info!("Removing id: {}", id);
        self.ids.remove(&id);
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RemoveWindow(id) | Message::CloseWindow(id) => {
                self.remove_id(id);
                iced_runtime::task::effect(Action::Window(WindowAction::Close(id)))
            }
            Message::ActionInvocation { id, action } => {
                info!("Action invocation: {} on {}", action, id);
                let reply_handle = self.reply_handle.clone();
                Task::future(async move {
                    reply_handle
                        .action_invoked(id.to_string().parse().unwrap(), action.as_str())
                        .await
                        .expect("Failed to send action invocation");
                    Message::CloseWindow(id)
                })
            }
            Message::Notification(msg) => match msg {
                NotificationMsg::Notification(n) => {
                    info!("Received notification: {n:#?}");
                    self.ids.insert(n.id, n.clone());
                    Task::done(Message::NewLayerShell {
                        settings: NewLayerShellSettings {
                            size: Some((400, HEIGHT)),
                            anchor: Anchor::Top | Anchor::Right,
                            layer: Layer::Top,
                            margin: Some((
                                HEIGHT as i32 * self.ids.len() as i32 - HEIGHT as i32 + 50,
                                100,
                                100,
                                100,
                            )),
                            keyboard_interactivity: KeyboardInteractivity::None,
                            ..Default::default()
                        },
                        id: n.id,
                    })
                }
            },
            Message::TickElapsed => {
                let mut tasks: Vec<Task<Message>> = vec![];
                self.ids.retain(|id, n| match n.expire_timeout {
                    Expiry::Never => true,
                    Expiry::Miliseconds(ms) => {
                        if Instant::now()
                            .duration_since(n.start_time.into())
                            .as_millis()
                            > ms
                        {
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
        let notification_box = self
            .ids
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
            Subscription::run_with(
                HashableSender(self.sender.clone()),
                build_notification_stream,
            )
            .map(|result| result.unwrap())
            .map(Message::Notification),
        ])
    }
}

fn build_notification_stream(sender: &HashableSender) -> BroadcastStream<NotificationMsg> {
    BroadcastStream::new(sender.0.subscribe())
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
    ) -> Poll<Option<Self::Item>> {
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

    fn render_notification_box(notification: &'_ Notification) -> Element<'_, Message> {
        // TODO: Use accent color from image
        let accent_color = Color::from_rgb(0.80, 0.1, 0.1);

        let mut row = Row::new();
        if let Some(img) = Self::get_image(notification) {
            row = row.push(
                Container::new(img)
                    .max_width(100)
                    .padding(Padding::new(10.))
                    .style(move |_| {
                        iced::widget::container::Style::default()
                            .border(Border::default().rounded(15))
                    }),
            );
        }

        let mut text_column = column![
            text!("{}", notification.summary.as_ref()).font(Font {
                weight: font::Weight::Bold,
                ..Font::default()
            }),
            text!("{}", notification.body.as_ref())
                .size(12)
                .align_x(Horizontal::Center)
        ]
        .align_x(Horizontal::Left)
        .width(Fill)
        .spacing(20);

        let actions = notification
            .actions
            .iter()
            .map(|(name, text)| {
                Button::new(text!("{}", text)).on_press(Message::ActionInvocation {
                    id: notification.id,
                    action: text.to_string(),
                })
            })
            .map(Element::new);
        text_column = text_column.push(Row::from_iter(actions).spacing(10));

        row = row.push(text_column);
        let corner_radius = &10;
        let progress_bar = container(progress_bar(0.0..=0.0, 0.0).girth(Length::Fixed(4.)).style(
            move |_: &iced::Theme| {
                iced::widget::progress_bar::Style {
                    bar: Background::Color(accent_color),
                    background: Background::Color(accent_color),
                    border: Border::default()
                        .rounded(Radius::new(*corner_radius))
                        .color(accent_color),
                }
            },
        ));

        container(column![progress_bar, row])
            .style(move |_theme| {
                container::Style::from(Color::BLACK).border(
                    Border::default()
                        .color(accent_color)
                        .rounded(Radius::new(*corner_radius)),
                )
            })
            .width(Fill)
            .height(Fill)
            .into()
    }
}
