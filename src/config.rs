use core::fmt;
use std::fs;

use serde::ser::Error as SerdeError;
use serde::{Deserialize, Serialize};

// Application configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct ChibiConfig {
    // TODO: Implement rnnoise as an optional feature, although it will increase
    // latency potentially
    /// Microphone detection threshold (RMS amplitude)
    #[serde(serialize_with = "round_to_hundredths")]
    pub microphone_threshold: f32,

    /// Deadband that determines when the microphone stays active prior to a signal drop off
    #[serde(serialize_with = "round_to_hundredths")]
    pub deadband_factor: f32,

    /// Can appear more visually appealing, but less accurate
    pub flicker_input: bool,
}

impl ChibiConfig {
    pub fn new(microphone_threshold: f32) -> Self {
        Self {
            microphone_threshold,
            ..Default::default()
        }
    }

    pub fn load(&mut self) {
        // Create the config file if it doesn't exist
        if !fs::metadata("config.toml").is_ok() {
            println!("config.toml not found, creating a new one");
            fs::write("config.toml", toml::to_string(self).unwrap()).unwrap();
        }

        // Load the config file
        let config_file = fs::read_to_string("config.toml").ok();
        if config_file.is_none() {
            println!("Failed to read config.toml");
            return;
        }

        println!("Loaded config.toml successfully");
        let config: ChibiConfig = toml::from_str(config_file.as_deref().unwrap()).unwrap();
        self.microphone_threshold = config.microphone_threshold;
        self.deadband_factor = config.deadband_factor;
        self.flicker_input = config.flicker_input;
    }

    pub fn save(&self) {
        fs::write("config.toml", toml::to_string(self).unwrap()).expect("Failed to save config");
    }
}

impl Default for ChibiConfig {
    fn default() -> Self {
        Self {
            microphone_threshold: 0.12,
            deadband_factor: 0.30,
            flicker_input: false,
        }
    }
}

// Custom serializer to get serde to do what I want
fn round_to_hundredths<S>(x: &f32, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    // Hack to get serde to serialize with two decimal places

    let formatted = format!("{:.2}", x);
    let num: f64 = formatted.parse().map_err(S::Error::custom)?;
    s.serialize_f64(num)
}

#[macro_export]
macro_rules! lock_and_unlock {
    ($mutex:expr) => {
        $mutex.lock().unwrap()
    };
}
