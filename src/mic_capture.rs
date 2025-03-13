use std::sync::{Arc, Mutex};

use crate::app::ChibiConfig;
use async_channel::Sender;
use cpal::{
    Device, SupportedStreamConfig,
    traits::{DeviceTrait, StreamTrait},
};

pub fn detect_input(
    config: ChibiConfig,
    input_device: Arc<Mutex<Device>>,
    input_config: Arc<Mutex<SupportedStreamConfig>>,
    sender: Sender<bool>,
) {
    let mic_threshold = config.microphone_threshold;

    let stream = input_device
        .lock()
        .unwrap()
        .build_input_stream(
            &input_config.lock().unwrap().clone().into(),
            move |data: &[f32], _| {
                let max_amplitude = data.iter().cloned().fold(0. / 0., f32::max);
                let mic_active = max_amplitude > mic_threshold;

                if sender.send_blocking(mic_active).is_err() {
                    eprintln!("Failed to send data to thread");
                }
            },
            |err| {
                eprintln!("Error occurred: {:?}", err);
            },
            None,
        )
        .expect("Failed to build input stream");

    stream.play().expect("Error playing stream");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

pub fn spawn_detection_thread(
    clone: Arc<Mutex<ChibiConfig>>,
    input_device: Arc<Mutex<Device>>,
    input_config: Arc<Mutex<SupportedStreamConfig>>,
    sender: Sender<bool>,
) {
    std::thread::spawn(move || {
        let config = clone.lock().unwrap().clone();
        detect_input(config, input_device, input_config, sender);
    });
}
