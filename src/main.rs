// src/main.rs

// --- Module declarations ---
mod audio;
mod ui;
mod viz_2d;
mod viz_3d;
mod viz_orb;
mod config;
mod camera;

// --- Plugin Imports ---
use crate::config::VisualsConfig;
use crate::audio::{AudioPlugin, SelectedAudioSource, MicStream};
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use rodio::{OutputStream, Sink};
use ui::UiPlugin;
use viz_2d::Viz2DPlugin;
use viz_3d::Viz3DPlugin;
use viz_orb::VizOrbPlugin;
use camera::CameraPlugin;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,
    MicSelection,
    Visualization3D,
    Visualization2D,
    VisualizationOrb,
}

// ADDED: This resource will store the last active visualization state.
#[derive(Resource, Debug, Clone)]
pub struct ActiveVisualization(pub AppState);

impl Default for ActiveVisualization {
    fn default() -> Self {
        // The 3D cube visualizer will be the default when the app starts.
        Self(AppState::Visualization3D)
    }
}


#[derive(Resource, Debug)]
pub struct VisualizationEnabled(pub bool);

impl Default for VisualizationEnabled {
    fn default() -> Self {
        Self(true)
    }
}


fn main() {
    let mut app = App::new();

    let (stream, stream_handle) = OutputStream::try_default().unwrap();

    app
        .add_plugins(DefaultPlugins)
        .insert_non_send_resource(stream)
        .insert_non_send_resource(Sink::try_new(&stream_handle).unwrap())
        .insert_non_send_resource(MicStream(None))
        .init_resource::<VisualsConfig>()
        .init_resource::<SelectedAudioSource>()
        .init_resource::<VisualizationEnabled>()
        .init_resource::<ActiveVisualization>() // ADDED: Initialize the new resource
        .init_state::<AppState>()
        .add_plugins((
            EguiPlugin,
            AudioPlugin,
            UiPlugin,
            Viz2DPlugin,
            Viz3DPlugin,
            VizOrbPlugin,
            CameraPlugin,
        ))
        .run();
}