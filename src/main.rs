// src/main.rs

// --- Module declarations ---
mod audio;
mod ui;
mod viz_2d;
mod viz_3d;
mod config;

// --- Plugin Imports ---
use crate::config::VisualsConfig;
use audio::{AudioPlugin, SelectedAudioSource, MicStream};
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use rodio::{OutputStream, Sink};
use ui::UiPlugin;
use viz_2d::Viz2DPlugin;
use viz_3d::Viz3DPlugin;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,
    MicSelection,
    Visualization3D,
    Visualization2D,
}

// NEW: A resource to control the visualizer's state.
#[derive(Resource, Debug)]
pub struct VisualizationEnabled(pub bool);

impl Default for VisualizationEnabled {
    fn default() -> Self {
        Self(true) // Enabled by default
    }
}


fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_non_send_resource(OutputStream::try_default().unwrap())
        .insert_non_send_resource(Sink::try_new(&OutputStream::try_default().unwrap().1).unwrap())
        .insert_non_send_resource(MicStream(None))
        .init_resource::<VisualsConfig>()
        .init_resource::<SelectedAudioSource>()
        .init_resource::<VisualizationEnabled>() // NEW: Initialize the resource
        .init_state::<AppState>()
        .add_plugins((
            EguiPlugin,
            AudioPlugin,
            UiPlugin,
            Viz2DPlugin,
            Viz3DPlugin,
            ScenePlugin,
        ))
        .run();
}

pub struct ScenePlugin;
impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::Visualization3D),
            setup_3d_scene
        );
    }
}

fn setup_3d_scene(mut commands: Commands) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-12.0, 10.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 2000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
}