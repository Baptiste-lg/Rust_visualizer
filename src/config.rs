// src/config.rs

use bevy::prelude::*;

#[derive(Resource, Clone)]
pub struct VisualsConfig {
    pub bass_sensitivity: f32,
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
    pub bloom_threshold: f32,
    pub bloom_color: Color,
    pub num_bands: usize,
    pub spread_enabled: bool,
    pub viz2d_inactive_color: Color,
    pub viz2d_active_color: Color,
    pub viz3d_base_color: Color,
    pub orb_base_color: Color,
    pub orb_peak_color: Color,
}

impl Default for VisualsConfig {
    fn default() -> Self {
        Self {
            bass_sensitivity: 4.0,
            bloom_enabled: true,
            bloom_intensity: 0.3,
            bloom_threshold: 0.8,
            bloom_color: Color::rgb(1.0, 0.2, 0.0),
            num_bands: 6,
            spread_enabled: true,
            viz2d_inactive_color: Color::rgb(0.2, 0.2, 0.8), // Dark Blue
            viz2d_active_color: Color::rgb(1.0, 0.3, 0.9),   // Bright Pink
            viz3d_base_color: Color::rgb(0.8, 0.7, 0.6),     // Default Beige
            orb_base_color: Color::rgb(0.1, 0.1, 0.7),       // Deep Blue
            orb_peak_color: Color::rgb(1.0, 0.0, 1.0),       // Magenta
        }
    }
}