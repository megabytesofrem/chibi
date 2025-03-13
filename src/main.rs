use app::{ChibiApp, ChibiConfig, Message};
use iced::{Task, Theme};
use std::sync::{Arc, Mutex};

pub mod app;
pub mod mic_capture;

fn main() -> iced::Result {
    // Create a channel to communicate with the detector thread
    let (sender, receiever) = async_channel::unbounded();
    let mut app = ChibiApp::new(ChibiConfig::default(), Some(receiever.clone()));

    // Load images from assets in the current directory
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let assets_dir = current_dir.join("assets");

    println!("Loading images from {:?}", assets_dir);

    // Load images from the assets directory
    app.load_images(&assets_dir);

    // Clone the input device and input config for the detector thread
    let input_device = Arc::new(Mutex::new(app.input_device.clone()));
    let input_config = Arc::new(Mutex::new(app.input_config.clone()));

    // Spawn the detector thread
    mic_capture::spawn_detection_thread(
        Arc::new(Mutex::new(app.config.clone())),
        input_device,
        input_config,
        sender,
    );

    // Capture the stream of messages from the detector thread and turn them into messages
    let stream_task = Task::stream(receiever).map(Message::MicActive);

    iced::application("anima", ChibiApp::update, ChibiApp::view)
        .theme(|_| Theme::GruvboxDark)
        .window(iced::window::Settings {
            size: (400.0, 400.0).into(),
            resizable: false,
            ..Default::default()
        })
        .subscription(ChibiApp::subscription)
        .run_with(|| (app, stream_task))
}
