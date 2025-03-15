//
// chibi: Indie PNG-tuber application made in Rust supporting all major platforms
// Licensed under the MPL-2.0 license
//

use app::{ChibiApp, Message};
use config::ChibiConfig;

use iced::{Task, Theme};
use std::sync::{Arc, Mutex};

pub mod app;
pub mod capture;
pub mod config;

fn main() -> iced::Result {
    // Create a channel to communicate with the detector thread
    let (sender, receiever) = async_channel::unbounded();
    let mut app = ChibiApp::new(ChibiConfig::default(), Some(receiever.clone()));

    // Load images from assets in the current directory
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let assets_dir = current_dir.join("assets");
    app.load_images(&assets_dir);

    let input_device = Arc::new(Mutex::new(app.selected_input_device.clone().unwrap()));
    let input_config = Arc::new(Mutex::new(app.selected_input_config.clone()));

    // Spawn the detector thread
    capture::spawn_capture_thread(
        Arc::new(Mutex::new(app.config.clone())),
        Arc::new(Mutex::new(input_device.lock().unwrap().raw_device.clone())),
        input_config,
        sender,
    );

    // Capture the stream of messages from the detector thread and turn them into messages
    let stream_task = Task::stream(receiever).map(Message::MicActive);

    iced::application("chibi", ChibiApp::update, ChibiApp::view)
        .theme(move |_| Theme::TokyoNight)
        .window(iced::window::Settings {
            size: (400.0, 400.0).into(),
            resizable: false,
            ..Default::default()
        })
        .subscription(ChibiApp::subscription)
        .run_with(|| (app, stream_task))
}
