//
// chibi: Indie PNG-tuber application made in Rust supporting all major platforms
// Licensed under the MPL-2.0 license
//

use std::path::Path;
use std::sync::Arc;

use async_channel::Receiver;
use cpal::Device;
use cpal::SupportedStreamConfig;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;

use iced::Alignment;
use iced::Event;
use iced::alignment::Horizontal;
use iced::event;
use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::Container;
use iced::widget::Space;
use iced::widget::container;
use iced::widget::image::Handle as ImageHandle;
use iced::widget::slider;
use iced::widget::{column, image, text};
use iced::{Element, Length};

#[derive(Clone)]
// Application configuration
pub struct ChibiConfig {
    images: Arc<Vec<ImageHandle>>,
    pub microphone_threshold: f32,
}

#[derive(Debug, Clone)]
pub enum View {
    Home,
    Settings,
    About,
}

#[derive(Debug, Clone)]
pub enum Message {
    MicActive(bool),
    MicThresholdChanged(f32),
    SwitchView(View),
    AppEvent(iced::Event),
}

// Internal application state
pub struct ChibiApp {
    pub config: ChibiConfig,
    pub input_device: Device,
    pub input_config: SupportedStreamConfig,

    // UI events
    mic_activated: bool,
    show_buttons: bool,

    // Currently displayed image
    curr_view: View,
    curr_image: Option<ImageHandle>,
    receiver: Option<Receiver<bool>>,
}

impl ChibiConfig {
    pub fn new(microphone_threshold: f32) -> Self {
        Self {
            microphone_threshold,
            images: Arc::new(vec![]),
        }
    }
}

// App implementation

macro_rules! aligned_button {
    ($text:expr) => {
        iced::widget::button::Button::new(
            iced::widget::text::Text::new($text)
                .align_x(iced::alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .padding(5)
    };
}

impl Default for ChibiConfig {
    fn default() -> Self {
        Self {
            images: Arc::new(vec![]),
            microphone_threshold: 0.15,
        }
    }
}

impl Default for ChibiApp {
    fn default() -> Self {
        Self {
            config: ChibiConfig::default(),
            input_device: cpal::default_host()
                .default_input_device()
                .expect("No input device available"),
            input_config: cpal::default_host()
                .default_input_device()
                .expect("No input device available")
                .default_input_config()
                .expect("No default input config"),

            mic_activated: false,
            show_buttons: true,
            curr_view: View::Home,
            curr_image: None,
            receiver: None,
        }
    }
}

impl ChibiApp {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::MicActive(active) => {
                if active {
                    self.curr_image = Some(self.config.images[1].clone());
                } else {
                    self.curr_image = Some(self.config.images[0].clone());
                }

                self.mic_activated = active;
            }
            Message::MicThresholdChanged(threshold) => {
                self.config.microphone_threshold = threshold;
            }
            Message::SwitchView(view) => {
                self.curr_view = view;
            }
            Message::AppEvent(event) => {
                if let Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) = event {
                    match key {
                        Key::Named(Named::Escape) => {
                            self.show_buttons = !self.show_buttons;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        // Subscribe to application events
        event::listen().map(Message::AppEvent)
    }

    fn view_home(&self) -> Element<Message> {
        let avatar_image = self
            .curr_image
            .clone()
            .unwrap_or(self.config.images[0].clone());

        let buttons = if self.show_buttons {
            column![
                aligned_button!("Settings").on_press(Message::SwitchView(View::Settings)),
                aligned_button!("About").on_press(Message::SwitchView(View::About)),
            ]
            .spacing(10)
        } else {
            column![Space::new(Length::Fill, Length::Fill)]
        };

        let layout = column![
            column![
                image(avatar_image)
                    .width(Length::Fixed(300.0))
                    .height(Length::Fixed(300.0)),
                if self.show_buttons {
                    text(format!("Microphone activated: {}", self.mic_activated)).size(12)
                } else {
                    text("")
                }
            ]
            .align_x(Alignment::Center)
            .width(Length::Fill),
            Space::new(Length::Fill, Length::Fill),
            buttons
        ];

        Container::new(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(5)
            .into()
    }

    fn view_settings(&self) -> Element<Message> {
        let threshold = column![
            text(format!(
                "Microphone threshold (default 0.15): {0:.2}",
                self.config.microphone_threshold
            ))
            .size(14),
            text(
                "Adjust the microphone threshold to activate the mic. \
                Increase this if it picks up background noise."
            )
            .color([0.8, 0.8, 0.8])
            .size(12),
            container(
                slider(
                    0.0..=1.0,
                    self.config.microphone_threshold,
                    Message::MicThresholdChanged,
                )
                .step(0.01),
            ),
        ]
        .padding([10, 0]);

        let layout = column![
            threshold,
            text(format!("Microphone activated: {}", self.mic_activated)).size(12),
            Space::new(Length::Fill, Length::Fill),
            aligned_button!("Back").on_press(Message::SwitchView(View::Home))
        ]
        .spacing(10);

        Container::new(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .into()
    }

    fn view_about(&self) -> Element<Message> {
        let labels = column![
            text("Chibi").size(24),
            text("Indie PNG-tuber application made in Rust supporting all major platforms")
                .size(12),
            text("The example assets used in this application are created by @chereverie").size(12),
            text("Licensed under the MPL-2.0 license").size(12),
        ]
        .align_x(Horizontal::Center)
        .width(Length::Fill)
        .spacing(10);

        let layout = column![
            labels,
            Space::new(Length::Fill, Length::Fill),
            aligned_button!("Back").on_press(Message::SwitchView(View::Home))
        ]
        .spacing(10);

        Container::new(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .into()
    }

    pub fn view(&self) -> Element<Message> {
        match self.curr_view {
            View::Home => self.view_home(),
            View::Settings => self.view_settings(),
            View::About => self.view_about(),
        }
    }
}

impl ChibiApp {
    pub fn new(config: ChibiConfig, receiver: Option<Receiver<bool>>) -> Self {
        Self {
            config,
            receiver,
            ..Default::default()
        }
    }

    pub fn load_images(&mut self, path: &Path) {
        let images = std::fs::read_dir(path)
            .expect("Failed to read directory")
            .map(|entry| {
                let entry = entry.expect("Failed to read entry");
                let path = entry.path();

                ImageHandle::from_path(path)
            })
            .collect();

        self.config.images = Arc::new(images);
    }
}
