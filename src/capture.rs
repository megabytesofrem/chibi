use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex},
};

use async_channel::Sender;
use cpal::{
    Device, SupportedStreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

use crate::config::ChibiConfig;

/// Abstraction over `cpal::Device` which includes a friendly name
#[derive(Clone)]
pub struct InputDevice {
    pub raw_device: cpal::Device,
    pub friendly_name: String,
}

impl fmt::Debug for InputDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({:?})", self.friendly_name, self.raw_device.name())
    }
}

impl fmt::Display for InputDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.friendly_name)
    }
}

impl InputDevice {
    pub fn new(raw_device: cpal::Device, friendly_name: String) -> Self {
        Self {
            raw_device,
            friendly_name,
        }
    }
}

/// Root mean square (RMS) amplitude of a signal
fn rms_amplitude(samples: &[f32]) -> f32 {
    let sum: f32 = samples.iter().map(|x| x * x).sum();
    (sum / samples.len() as f32).sqrt()
}

/// Wrapper over `cpal::default_input_device`
pub fn get_default_device() -> Option<InputDevice> {
    let host = cpal::default_host();
    let default_device = host
        .default_input_device()
        .expect("Failed to get default input device");

    let input_device;

    // Query it with ALSA hints on Linux
    #[cfg(target_os = "linux")]
    {
        let dev_name = default_device
            .clone()
            .name()
            .expect("Failed to get device name");

        input_device = Some(get_alsa_hint_for(&dev_name).map_or_else(
            || InputDevice::new(default_device.clone(), dev_name.clone()),
            |_| InputDevice::new(default_device.clone(), dev_name.clone()),
        ));
    }

    // On any other platform, just use the device name
    #[cfg(not(target_os = "linux"))]
    {
        let dev_name = default_device.name().expect("Failed to get device name");
        input_device = Some(InputDevice::new(default_device, dev_name));
    }

    input_device
}

#[cfg(target_os = "linux")]
fn get_alsa_hint_for(name: &str) -> Option<String> {
    let hints = get_alsa_hints();
    hints.get(name).cloned()
}

#[cfg(target_os = "linux")]
fn get_alsa_hints() -> HashMap<String, String> {
    use alsa::Direction;
    use alsa::device_name::HintIter;
    use std::ffi::CString;

    let mut hints = HashMap::new();

    let iface = CString::new("pcm").expect("CString::new failed");
    let hint_iter = HintIter::new(None, &iface).expect("Failed to get ALSA hints");

    for hint in hint_iter {
        let name = hint.name.expect("Failed to get hint name");
        let desc = hint.desc.expect("Failed to get hint description");

        if let Some(direction) = hint.direction {
            if direction != Direction::Capture {
                continue;
            }
        }

        hints.insert(name.to_string(), desc.to_string());
    }

    hints
}

/// Return a list of input devices tagged with their friendly names
///
/// On Linux, this will use the `alsa` crate to query hints
/// On any other platform, this will use the device name as returned by `cpal`
pub fn get_input_devices() -> Vec<InputDevice> {
    let input_devices: Vec<InputDevice>;

    let host = cpal::default_host();
    let devices: Vec<cpal::Device> = host
        .input_devices()
        .expect("No input devices found")
        .collect();

    // On Linux query ALSA hints for the device description and use that
    #[cfg(target_os = "linux")]
    {
        let hints = get_alsa_hints();
        input_devices = devices
            .into_iter()
            .map(|dev| {
                let dev_name = dev.name().unwrap_or_else(|_| "Unknown".into());

                InputDevice::new(
                    dev,
                    match dev_name.to_lowercase() {
                        s if s.contains("pipewire") => "Pipewire Media Server".to_string(),
                        s if s.contains("pulse") => "PulseAudio".to_string(),
                        _ => hints.get(&dev_name).cloned().unwrap_or(dev_name),
                    },
                )
            })
            .collect();
    }

    // On any other platform than Linux, just use the device name
    #[cfg(not(target_os = "linux"))]
    {
        input_devices = devices
            .iter()
            .map(|dev| {
                let dev_name = dev.name().expect("Failed to get device name");
                InputDevice::new(dev.clone(), dev_name)
            })
            .collect();
    }

    input_devices
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
            let rms_threshold_off = rms_threshold_on * config.hysteresis_factor; // Hysteresis, aka "deadband"

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

pub fn spawn_capture_thread(
    config: Arc<Mutex<ChibiConfig>>,
    input_device: Arc<Mutex<Device>>,
    input_config: Arc<Mutex<SupportedStreamConfig>>,
    sender: Sender<bool>,
) {
    let config = config.lock().unwrap().clone();
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
