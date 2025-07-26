// src/config.rs

use bevy::prelude::*;

#[derive(Resource, Clone)]
pub struct VisualsConfig {
    // --- Paramètres Généraux ---
    pub bass_sensitivity: f32,
    pub num_bands: usize,

    // --- Paramètres du Bloom ---
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
    pub bloom_threshold: f32,
    pub bloom_color: Color,

    // --- Visualiseur 2D ---
    pub viz2d_inactive_color: Color,
    pub viz2d_active_color: Color,

    // --- Visualiseur 3D ---
    pub spread_enabled: bool,
    pub viz3d_base_color: Color,
    pub viz3d_column_size: usize,

    // --- Visualiseur Orbe ---
    pub orb_base_color: Color,
    pub orb_peak_color: Color,
    pub orb_noise_speed: f32,
    pub orb_noise_frequency: f32,
    pub orb_treble_influence: f32,
}

impl Default for VisualsConfig {
    fn default() -> Self {
        Self {
            // --- Généraux ---
            bass_sensitivity: 4.0,
            num_bands: 16,

            // --- Bloom ---
            bloom_enabled: true,
            bloom_intensity: 0.3,
            bloom_threshold: 0.8,
            bloom_color: Color::rgb(1.0, 0.2, 0.0),

            // --- 2D ---
            viz2d_inactive_color: Color::rgb(0.2, 0.2, 0.8),
            viz2d_active_color: Color::rgb(1.0, 0.3, 0.9),

            // --- 3D ---
            spread_enabled: true,
            viz3d_base_color: Color::rgb(0.8, 0.7, 0.6),
            viz3d_column_size: 8,

            // --- Orbe ---
            orb_base_color: Color::rgb(0.1, 0.1, 0.7),
            orb_peak_color: Color::rgb(1.0, 0.0, 1.0),
            orb_noise_speed: 1.0,
            orb_noise_frequency: 2.0,
            orb_treble_influence: 0.3,
        }
    }
}