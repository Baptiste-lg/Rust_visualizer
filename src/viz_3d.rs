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

    // MODIFIED: Created a unique material for each cube
    // This is necessary so we can change the emissive color of each one independently.
    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    let cube_spacing = 2.0;
    let grid_size = 8;

    for x in 0..grid_size {
        for z in 0..grid_size {
            let x_pos = (x as f32 - grid_size as f32 / 2.0) * cube_spacing;
            let z_pos = (z as f32 - grid_size as f32 / 2.0) * cube_spacing;

            // KEY CHANGE: Each cube now gets its own material instance.
            let material = materials.add(StandardMaterial {
                base_color: Color::rgb(0.8, 0.7, 0.6),
                // The emissive property makes the object glow. We start it at black.
                emissive: Color::BLACK,
                metallic: 1.0,
                perceptual_roughness: 0.1,
                ..default()
            });

            commands.spawn((
                PbrBundle {
                    mesh: cube_mesh.clone(),
                    material: material, // Use the unique material
                    transform: Transform::from_xyz(x_pos, 0.0, z_pos),
                    ..default()
                },
                VisualizerCube,
            ));
        }
    }
}

fn update_3d_visuals(
    audio_analysis: Res<AudioAnalysis>,
    config: Res<VisualsConfig>,
    // MODIFIED: We now query for the material handle and need mutable access to the materials asset store
    mut query: Query<(&mut Transform, &Handle<StandardMaterial>), With<VisualizerCube>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let bass_height = 1.0 + audio_analysis.bass * config.bass_sensitivity;
    let smoothing_factor = 0.1;

    // KEY CHANGE: Create an HDR color based on the bass.
    // Values greater than 1.0 will cause a strong bloom.
    let emissive_color = Color::rgb_u8(255, 60, 0); // An orange color
    let emissive_strength = audio_analysis.bass * 5.0; // Multiplier to make it really bright!

    for (mut transform, material_handle) in &mut query {
        let current_scale = transform.scale.y;
        let target_scale = bass_height;

        transform.scale.y = current_scale + (target_scale - current_scale) * smoothing_factor;

        // Get the actual material from the handle and update its emissive property
        if let Some(material) = materials.get_mut(material_handle) {
            material.emissive = emissive_color * emissive_strength;
        }
    }
}