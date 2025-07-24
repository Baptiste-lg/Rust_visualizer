// src/config.rs

use bevy::prelude::*;

// A resource to store user-configurable settings.
#[derive(Resource)]
pub struct VisualsConfig {
    // Multiplier for the intensity of the bass reaction.
    pub bass_sensitivity: f32,
    // Other settings like colors, animation type, etc., could be added here.
}

impl Default for VisualsConfig {
    fn default() -> Self {
        Self {
            // A sensible default value.
            bass_sensitivity: 4.0,
        }
    }
}