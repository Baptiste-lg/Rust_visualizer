// src/config.rs

use bevy::prelude::*;

// A resource to store user-configurable settings.
#[derive(Resource)]
pub struct VisualsConfig {
    // Multiplier for the intensity of the bass reaction.
    pub bass_sensitivity: f32,
    // ADDED: Bloom effect settings
    pub bloom_intensity: f32,
    pub bloom_threshold: f32,
}

impl Default for VisualsConfig {
    fn default() -> Self {
        Self {
            // A sensible default value.
            bass_sensitivity: 4.0,
            // ADDED: Sensible defaults for bloom
            bloom_intensity: 0.15,
            bloom_threshold: 0.8,
        }
    }
}