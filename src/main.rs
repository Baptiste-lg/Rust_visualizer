// src/main.rs

// --- Module declarations ---
mod audio;
mod camera;
mod config;
mod ui;
mod viz_2d;
mod viz_3d;
mod viz_disc;
mod viz_ico;
mod viz_orb; // <--- AJOUT : Déclaration du module

// --- Plugin Imports ---
use crate::audio::{AudioPlugin, MicStream, PlaybackInfo, SelectedAudioSource};
use crate::camera::CameraPlugin;
use crate::config::VisualsConfig;
use crate::ui::UiPlugin;
use crate::viz_2d::Viz2DPlugin;
use crate::viz_3d::Viz3DPlugin;
use crate::viz_disc::VizDiscPlugin;
use crate::viz_ico::VizIcoPlugin;
use crate::viz_orb::VizOrbPlugin; // <--- AJOUT : Import du plugin

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use rodio::{OutputStream, Sink};

// Defines the different states of the application.
// This controls which screen or visualization is currently active.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,
    MicSelection,
    Visualization3D,
    Visualization2D,
    VisualizationOrb,
    VisualizationDisc,
    VisualizationIco, // <--- AJOUT : Nouvel état pour le shader Ico
}

// A resource to keep track of the last active visualization.
// This allows the app to return to the correct visualizer from the main menu.
#[derive(Resource, Debug, Clone)]
pub struct ActiveVisualization(pub AppState);

impl Default for ActiveVisualization {
    fn default() -> Self {
        Self(AppState::Visualization3D)
    }
}

// A resource to globally enable or disable the visualizer rendering.
#[derive(Resource, Debug)]
pub struct VisualizationEnabled(pub bool);

impl Default for VisualizationEnabled {
    fn default() -> Self {
        Self(true)
    }
}

// The main entry point of the application.
fn main() {
    let mut app = App::new();

    // Initialize the audio output stream and sink provided by the `rodio` library.
    // These are `NonSend` resources because they need to stay on the main thread.
    let (stream, stream_handle) = OutputStream::try_default().unwrap();

    app.add_plugins(DefaultPlugins)
        // Insert audio resources required for playback and analysis.
        .insert_non_send_resource(stream)
        .insert_non_send_resource(Sink::try_new(&stream_handle).unwrap())
        .insert_non_send_resource(MicStream(None))
        // Initialize all application resources and states.
        .init_resource::<VisualsConfig>()
        .init_resource::<SelectedAudioSource>()
        .init_resource::<VisualizationEnabled>()
        .init_resource::<ActiveVisualization>()
        .init_resource::<PlaybackInfo>()
        .init_state::<AppState>()
        // Add all the custom plugins that define the app's functionality.
        .add_plugins((
            EguiPlugin,
            AudioPlugin,
            UiPlugin,
            Viz2DPlugin,
            Viz3DPlugin,
            VizOrbPlugin,
            CameraPlugin,
            VizDiscPlugin,
            VizIcoPlugin, // <--- AJOUT : Activation du plugin
        ))
        .run();
}
