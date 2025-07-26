// src/config.rs

use bevy::prelude::*;

#[derive(Resource)]
pub struct VisualsConfig {
    pub bass_sensitivity: f32,
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
    pub bloom_threshold: f32,
    pub bloom_color: Color,
    pub num_bands: usize,
}

impl Default for VisualsConfig {
    fn default() -> Self {
        Self {
            bass_sensitivity: 4.0,
            bloom_enabled: true,
            bloom_intensity: 0.3,
            bloom_threshold: 0.8,
            bloom_color: Color::rgb(1.0, 0.2, 0.0),
            num_bands: 5,
        }
    }
}