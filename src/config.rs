// src/config.rs

use bevy::prelude::*;

// Une ressource pour stocker les paramètres configurables par l'utilisateur.
// Nous la dérivons de `Resource` pour que Bevy puisse la gérer.
#[derive(Resource)]
pub struct VisualsConfig {
    // Multiplicateur pour l'intensité de la réaction des basses.
    // Une valeur de 1.0 est la normale, 2.0 double l'effet, etc.
    pub bass_sensitivity: f32,
    // On pourrait ajouter d'autres paramètres ici plus tard,
    // comme les couleurs, le type d'animation, etc.
}

// Nous fournissons une valeur par défaut pour cette configuration.
impl Default for VisualsConfig {
    fn default() -> Self {
        Self {
            bass_sensitivity: 4.0, // La valeur de base que tu utilisais déjà
        }
    }
}