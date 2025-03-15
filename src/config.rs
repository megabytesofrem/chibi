use std::sync::Arc;

use iced::widget::image::Handle;

#[derive(Clone)]
// Application configuration
pub struct ChibiConfig {
    images: Arc<Vec<Handle>>,

    // TODO: Implement rnnoise as an optional feature, although it will increase
    // latency potentially
    /// Microphone detection threshold (RMS amplitude)
    pub microphone_threshold: f32,

    /// Hysteresis factor for the microphone threshold. Acts as a deadband so the microphone
    /// stays active until the signal drops below a lower off threshold.
    pub hysteresis_factor: f32,
}

impl ChibiConfig {
    pub fn new(microphone_threshold: f32) -> Self {
        Self {
            microphone_threshold,
            images: Arc::new(vec![]),
            ..Default::default()
        }
    }

    pub fn set_images(&mut self, images: Vec<Handle>) {
        self.images = Arc::new(images);
    }

    pub fn get_image(&self, index: usize) -> Option<&Handle> {
        self.images.get(index)
    }
}

impl Default for ChibiConfig {
    fn default() -> Self {
        Self {
            images: Arc::new(vec![]),
            microphone_threshold: 0.12,
            hysteresis_factor: 0.30,
        }
    }
}
