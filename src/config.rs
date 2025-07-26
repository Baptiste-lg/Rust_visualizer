// src/config.rs

use bevy::prelude::*;

#[derive(Resource)]
pub struct VisualsConfig {
    pub bass_sensitivity: f32,
    // ADDED: A toggle for the bloom effect
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
    pub bloom_threshold: f32,
}

impl Default for VisualsConfig {
    fn default() -> Self {
        Self {
            bass_sensitivity: 4.0,
            // ADDED: Default bloom to be enabled
            bloom_enabled: true,
            bloom_intensity: 0.15,
            bloom_threshold: 0.8,
        }
    }
}