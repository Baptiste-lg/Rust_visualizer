// src/viz_3d.rs

use bevy::prelude::*;
use crate::{AppState, audio::AudioAnalysis, config::VisualsConfig, VisualizationEnabled};
use crate::audio::audio_analysis_system;

pub struct Viz3DPlugin;

impl Plugin for Viz3DPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::Visualization3D), setup_3d_visuals)
            .add_systems(Update, update_3d_visuals
                .after(audio_analysis_system)
                .run_if(in_state(AppState::Visualization3D))
                .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0)
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

    let cube_spacing = 2.0;
    let grid_size = 8;

    for x in 0..grid_size {
        for z in 0..grid_size {
            let x_pos = (x as f32 - grid_size as f32 / 2.0) * cube_spacing;
            let z_pos = (z as f32 - grid_size as f32 / 2.0) * cube_spacing;

            let material = materials.add(StandardMaterial {
                base_color: Color::rgb(0.8, 0.7, 0.6),
                // MODIFIED: Set the initial emissive color to black.
                emissive: Color::BLACK,
                metallic: 1.0,
                perceptual_roughness: 0.1,
                ..default()
            });

            commands.spawn((
                PbrBundle {
                    mesh: cube_mesh.clone(),
                    material: material,
                    transform: Transform::from_xyz(x_pos, 0.0, z_pos),
                    ..default()
                },
                VisualizerCube,
            ));
        }
    }
}

// MODIFIED: This function now calculates a much stronger emissive color to make the bloom pop.
fn update_3d_visuals(
    audio_analysis: Res<AudioAnalysis>,
    config: Res<VisualsConfig>,
    mut query: Query<(&mut Transform, &Handle<StandardMaterial>), With<VisualizerCube>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let bass_height = 1.0 + audio_analysis.bass * config.bass_sensitivity;
    let smoothing_factor = 0.1;

    // We calculate the emissive color only if bloom is enabled.
    let emissive_color = if config.bloom_enabled {
        // Using a bright orange color as a base.
        let base_color = Color::rgb(2.0, 0.5, 0.0);
        // We multiply the color by the bass intensity to make it glow.
        // The value can go above 1.0, which is necessary for HDR.
        base_color * audio_analysis.bass * 10.0
    } else {
        // If bloom is disabled, the cubes emit no light.
        Color::BLACK
    };

    for (mut transform, material_handle) in &mut query {
        let current_scale = transform.scale.y;
        let target_scale = bass_height;

        transform.scale.y = current_scale + (target_scale - current_scale) * smoothing_factor;

        if let Some(material) = materials.get_mut(material_handle) {
            // We apply the new emissive color to the material of each cube.
            material.emissive = emissive_color;
        }
    }
}