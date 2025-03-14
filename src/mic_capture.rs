use std::sync::{Arc, Mutex};

use crate::app::ChibiConfig;
use async_channel::Sender;
use cpal::{
    Device, SupportedStreamConfig,
    traits::{DeviceTrait, StreamTrait},
};

/// Root mean square (RMS) amplitude of a signal
fn rms_amplitude(samples: &[f32]) -> f32 {
    let sum: f32 = samples.iter().map(|x| x * x).sum();
    (sum / samples.len() as f32).sqrt()
}

fn capture_input(
    config: ChibiConfig,
    input_device: Arc<Mutex<Device>>,
    input_config: Arc<Mutex<SupportedStreamConfig>>,
    buffer: Arc<Mutex<Vec<i16>>>,

    sender: Sender<bool>,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    // Future additions:
    // TODO: Amplify the signal when we receive it, before calculating RMS
    // TODO: DSP processing so the signal is as clean as possible

    let err_fn = |err| eprintln!("Error in audio stream: {}", err);

    let mut mic_active = false;

    input_device.lock().unwrap().build_input_stream(
        &input_config.lock().unwrap().clone().into(),
        move |data: &[f32], _| {
            // Compute RMS amplitude
            let rms = rms_amplitude(data);

            let rms_threshold_on = config.microphone_threshold;
            let rms_threshold_off = rms_threshold_on * config.hysteris_factor; // Hysteresis, aka "deadband"

            if mic_active {
                if rms < rms_threshold_off {
                    mic_active = false;
                }
            } else {
                if rms >= rms_threshold_on {
                    mic_active = true;
                }
            }

            sender.send_blocking(mic_active).unwrap();

            // Only process audio if the microphone is active
            if !mic_active {
                return;
            }

            let samples: Vec<i16> = data
                .iter()
                .map(|&sample| {
                    let clamped = sample.max(-1.0).min(1.0);
                    (clamped * 32767.0) as i16
                })
                .collect();

            // Append samples to the shared buffer
            let mut buf = buffer.lock().unwrap();
            buf.extend_from_slice(&samples);
        },
        err_fn,
        None,
    )
}

pub fn spawn_detection_thread(
    clone: Arc<Mutex<ChibiConfig>>,
    input_device: Arc<Mutex<Device>>,
    input_config: Arc<Mutex<SupportedStreamConfig>>,
    sender: Sender<bool>,
) {
    let config = clone.lock().unwrap().clone();
    let buffer = Arc::new(Mutex::new(Vec::<i16>::new()));

    std::thread::spawn(move || {
        let stream = capture_input(config, input_device, input_config, buffer.clone(), sender)
            .expect("Failed to capture input stream");

        stream.play().expect("Failed to play stream");

        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    });
}
