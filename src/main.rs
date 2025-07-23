// src/main.rs

// --- Module declarations ---
mod audio;
mod ui;
mod viz_2d;
mod viz_3d;
mod config;

// --- Plugin Imports ---
use crate::config::VisualsConfig;
use audio::AudioPlugin;
use bevy::prelude::*;
use rodio::{OutputStream, Sink};
use ui::UiPlugin;
use viz_2d::Viz2DPlugin;
use viz_3d::Viz3DPlugin;
use bevy_egui::EguiPlugin;

/// The global state of the application, shared between modules.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,
    MicSelection, // State for the new microphone selection menu
    Visualization3D,
    Visualization2D,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Rodio resources must be inserted here as they are not `Send`
        .insert_non_send_resource(OutputStream::try_default().unwrap())
        .insert_non_send_resource(Sink::try_new(&OutputStream::try_default().unwrap().1).unwrap())
        .init_resource::<VisualsConfig>()
        // Initialize the application state
        .init_state::<AppState>()
        // Add all of our plugins
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

/// A plugin to set up the basic 3D scene (camera and light).
pub struct ScenePlugin;
impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::Visualization3D),
            setup_3d_scene
        );
    }
}

/// This system spawns the 3D camera and a point light.
fn setup_3d_scene(mut commands: Commands) {
    info!("Setting up 3D scene...");
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