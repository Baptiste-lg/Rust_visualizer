// src/viz_3d.rs

use bevy::prelude::*;
use crate::{AppState, AudioAnalysis}; // Import shared data from main.rs

// The plugin for our 3D visualization
pub struct Viz3DPlugin;

impl Plugin for Viz3DPlugin {
    fn build(&self, app: &mut App) {
        app
            // Add our systems to run only in the 3D visualization state
            .add_systems(OnEnter(AppState::Visualization3D), setup_3d_visuals)
            .add_systems(Update, update_3d_visuals.run_if(in_state(AppState::Visualization3D)));
    }
}

// Marker component for the cubes in our 3D scene
#[derive(Component)]
struct VisualizerCube;

/// This system runs once when we enter the 3D visualization state
fn setup_3d_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Setting up 3D visuals...");

    // Create a handle for the cube mesh and material
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
                // PbrBundle is the standard bundle for 3D objects with physically-based rendering
                PbrBundle {
                    mesh: cube_mesh.clone(),
                    material: cube_material.clone(),
                    transform: Transform::from_xyz(x_pos, 0.0, z_pos),
                    ..default()
                },
                VisualizerCube, // Tag our cube
            ));
        }
    }
}

/// This system runs every frame in the 3D visualization state
fn update_3d_visuals(
    audio_analysis: Res<AudioAnalysis>,
    mut query: Query<&mut Transform, With<VisualizerCube>>,
) {
    // We'll make the cubes' height react to the bass
    // We add a small base value (1.0) and multiply the bass to make the effect more dramatic
    let bass_height = 1.0 + audio_analysis.bass * 4.0;

    // We use lerp (linear interpolation) to smooth the movement of the cubes
    // This prevents them from flickering too aggressively
    let smoothing_factor = 0.1;

    for mut transform in &mut query {
        // Get the current scale and the target scale
        let current_scale = transform.scale.y;
        let target_scale = bass_height;

        // Calculate the new smoothed scale
        transform.scale.y = current_scale + (target_scale - current_scale) * smoothing_factor;
    }
}