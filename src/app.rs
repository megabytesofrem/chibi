//
// chibi: Indie PNG-tuber application made in Rust supporting all major platforms
// Licensed under the MPL-2.0 license
//

use std::path::Path;

use async_channel::Receiver;
use cpal::SupportedStreamConfig;
use cpal::traits::DeviceTrait;

use iced::Alignment;
use iced::Event;
use iced::alignment;
use iced::event;
use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::Container;
use iced::widget::Space;
use iced::widget::image::Handle;
use iced::widget::{button, column, combo_box, container, image, row, slider, text};
use iced::{Element, Length};

use crate::capture;
use crate::capture::InputDevice;
use crate::config::ChibiConfig;

const APP_VERSION: f32 = 1.1;

#[derive(Debug, Clone)]
pub enum View {
    Home,
    Settings,
    About,
}

#[derive(Debug, Clone)]
pub enum Message {
    MicActive(bool),
    ThresholdChanged(f32),
    HysteresisChanged(f32),
    InputChanged(InputDevice),
    SwitchView(View),
    AppEvent(iced::Event),
}

// Internal application state
pub struct ChibiApp {
    pub config: ChibiConfig,

    // Input device state
    pub available_input_devices: combo_box::State<InputDevice>,
    pub selected_input_device: Option<InputDevice>,
    pub selected_input_config: SupportedStreamConfig,

    // UI events
    mic_activated: bool,
    show_buttons: bool,
    show_modal: bool,
    chroma_key: bool,

    // Currently displayed image
    curr_view: View,
    curr_image: Option<Handle>,
    pub receiver: Option<Receiver<bool>>,
}

// App implementation

fn aligned_button<'a>(text: &'a str) -> button::Button<'a, Message> {
    button::Button::new(
        text::Text::new(text)
            .align_x(alignment::Horizontal::Center)
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding(5)
}

fn detailed_slider<'a, Message>(
    label: String,
    detail: String,
    range: std::ops::RangeInclusive<f32>,
    value: f32,
    message: impl Fn(f32) -> Message + 'a,
) -> Container<'a, Message>
where
    Message: Clone + 'a,
{
    container(column![
        text(label).size(14),
        text(detail)
            .size(12)
            .width(Length::Fill)
            .color([0.8, 0.8, 0.8]),
        container(slider(range, value, move |v| message(v)).step(0.01),),
    ])
}

impl Default for ChibiApp {
    fn default() -> Self {
        Self {
            config: ChibiConfig::default(),
            available_input_devices: combo_box::State::new(capture::get_input_devices()),
            selected_input_device: capture::get_default_device(),
            selected_input_config: capture::get_default_device()
                .unwrap()
                .raw_device
                .default_input_config()
                .unwrap(),
            mic_activated: false,
            show_buttons: true,
            show_modal: false,
            chroma_key: false,
            curr_view: View::Home,
            curr_image: None,
            receiver: None,
        }
    }
}

impl ChibiApp {
    fn view_home<'a>(&self) -> Element<Message> {
        let avatar_image = self
            .curr_image
            .clone()
            .unwrap_or(self.config.get_image(0).unwrap().clone());

        let buttons = if self.show_buttons {
            row![
                aligned_button("Settings").on_press(Message::SwitchView(View::Settings)),
                aligned_button("About").on_press(Message::SwitchView(View::About)),
            ]
            .spacing(5)
        } else {
            row![Space::new(Length::Fill, Length::Fill)]
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

        if self.chroma_key {
            container(layout)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgb(
                        1.0, 0.0, 1.0,
                    ))),
                    ..Default::default()
                })
                .padding(15)
                .into()
        } else {
            container(layout)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(15)
                .into()
        }
    }

    fn view_settings<'a>(&self) -> Element<Message> {
        // FIXME: Combobox shows up initially as "default" when nothing is selected

        let threshold_slider = detailed_slider(
            format!(
                "Microphone threshold: {:.2}",
                self.config.microphone_threshold
            )
            .into(),
            "Adjust the microphone detection threshold. \
            Too low a value may cause the microphone to activate too easily."
                .trim()
                .into(),
            0.0..=1.0,
            self.config.microphone_threshold,
            |value| Message::ThresholdChanged(value),
        );

        let hysteresis_slider = detailed_slider(
            format!("Hysteresis factor: {:.2}", self.config.hysteresis_factor).into(),
            "Adjust the Hysteresis factor. \
            Acts as a deadband so the microphone stays active until the signal drops \
            below a lower \"off\" threshold."
                .trim()
                .into(),
            0.0..=1.0,
            self.config.hysteresis_factor,
            |value| Message::HysteresisChanged(value),
        );

        let combo_input = column![
            text("Select an input device:").size(14),
            combo_box(
                &self.available_input_devices,
                "Input device",
                self.selected_input_device.as_ref(),
                Message::InputChanged,
            ),
            text("After selecting an input device, you will need to restart the application.")
                .color([0.8, 0.8, 0.8])
                .size(12)
        ];

        let ui_hints = column![
            text("Press 'ESC' to show/hide UI elements")
                .color([0.8, 0.8, 0.8])
                .size(12),
            text("Press 'c' to toggle chroma key")
                .color([0.8, 0.8, 0.8])
                .size(12),
        ];

        let layout = column![
            threshold_slider,
            hysteresis_slider,
            combo_input,
            Space::new(Length::Fill, Length::Fill),
            ui_hints,
            text(format!("Microphone activated: {}", self.mic_activated)).size(12),
            Space::new(Length::Fill, Length::Fill),
            aligned_button("Back").on_press(Message::SwitchView(View::Home))
        ]
        .spacing(10)
        .padding(15);

        container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_about<'a>(&self) -> Element<Message> {
        let labels = column![
            text(format!("Chibi {}", APP_VERSION)).size(24),
            text("Indie PNG-tuber application made in Rust supporting all major platforms")
                .size(12),
            text("The example assets used in this application are created by @chereverie").size(12),
            text("Licensed under the MPL-2.0 license").size(12),
        ]
        .align_x(alignment::Horizontal::Center)
        .width(Length::Fill)
        .spacing(10);

        let layout = column![
            labels,
            Space::new(Length::Fill, Length::Fill),
            aligned_button("Back").on_press(Message::SwitchView(View::Home))
        ]
        .spacing(10);

        container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(15)
            .into()
    }

    pub fn view(&self) -> Element<Message> {
        match self.curr_view {
            View::Home => self.view_home(),
            View::Settings => self.view_settings(),
            View::About => self.view_about(),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::MicActive(active) => {
                if active {
                    self.curr_image = Some(self.config.get_image(1).unwrap().clone());
                } else {
                    self.curr_image = Some(self.config.get_image(0).unwrap().clone());
                }

                self.mic_activated = active;
            }
            Message::ThresholdChanged(threshold) => {
                self.config.microphone_threshold = threshold;
            }
            Message::HysteresisChanged(hysteris) => {
                self.config.hysteresis_factor = hysteris;
            }
            Message::SwitchView(view) => {
                self.curr_view = view;
            }
            Message::InputChanged(device) => {
                self.selected_input_device = Some(device.clone());
                self.show_modal = true;
            }
            Message::AppEvent(event) => {
                if let Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) = event {
                    match key {
                        Key::Named(Named::Escape) => {
                            self.show_buttons = !self.show_buttons;
                        }
                        Key::Character(c) if c == "c" => {
                            self.chroma_key = !self.chroma_key;
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

                Handle::from_path(path)
            })
            .collect();

        self.config.set_images(images);
    }
}
