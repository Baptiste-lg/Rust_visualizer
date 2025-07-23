// src/viz_3d.rs

use bevy::prelude::*;
use crate::{AppState, audio::AudioAnalysis, config::VisualsConfig}; // Import new config
use crate::audio::audio_analysis_system;

pub struct Viz3DPlugin;

impl Plugin for Viz3DPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::Visualization3D), setup_3d_visuals)
            .add_systems(Update, update_3d_visuals
                .after(audio_analysis_system)
                .run_if(in_state(AppState::Visualization3D))
            );
    }
}

#[derive(Component)]
struct VisualizerCube;

fn setup_3d_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Setting up 3D visuals...");

    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let cube_material = materials.add(StandardMaterial {
        base_color: Color::rgb(0.8, 0.7, 0.6),
        metallic: 1.0,
        perceptual_roughness: 0.1,
        ..default()
    });

    let cube_spacing = 2.0;
    let grid_size = 8;

    for x in 0..grid_size {
        for z in 0..grid_size {
            let x_pos = (x as f32 - grid_size as f32 / 2.0) * cube_spacing;
            let z_pos = (z as f32 - grid_size as f32 / 2.0) * cube_spacing;

            commands.spawn((
                PbrBundle {
                    mesh: cube_mesh.clone(),
                    material: cube_material.clone(),
                    transform: Transform::from_xyz(x_pos, 0.0, z_pos),
                    ..default()
                },
                VisualizerCube,
            ));
        }
    }
}

// UPDATED: Now takes VisualsConfig as a parameter
fn update_3d_visuals(
    audio_analysis: Res<AudioAnalysis>,
    config: Res<VisualsConfig>, // <-- Read the config resource
    mut query: Query<&mut Transform, With<VisualizerCube>>,
) {
    // Use the sensitivity from the config resource
    let bass_height = 1.0 + audio_analysis.bass * config.bass_sensitivity;

    let smoothing_factor = 0.1;

    for mut transform in &mut query {
        let current_scale = transform.scale.y;
        let target_scale = bass_height;

        transform.scale.y = current_scale + (target_scale - current_scale) * smoothing_factor;
    }
}