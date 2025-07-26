// src/viz_3d.rs

use bevy::prelude::*;
use crate::{AppState, audio::AudioAnalysis, config::VisualsConfig, VisualizationEnabled};

pub struct Viz3DPlugin;

/// A local resource to keep track of the state of our voxel grid.
/// This helps us know when we need to respawn the cubes.
#[derive(Resource, Default)]
struct VoxelGridState {
    num_bands: usize,
}

impl Plugin for Viz3DPlugin {
    fn build(&self, app: &mut App) {
        app
            // We introduce a local resource to manage the state.
            .init_resource::<VoxelGridState>()
            // We remove all previous cube-spawning logic and replace it with this single system.
            .add_systems(Update, (
                manage_voxel_grid,
                // We ensure the animation runs *after* the grid has been potentially updated.
                update_3d_visuals.after(manage_voxel_grid),
            )
                .run_if(in_state(AppState::Visualization3D))
                .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0)
            )
            // Add a system to clean up when we exit the 3D visualization state.
            .add_systems(OnExit(AppState::Visualization3D), (despawn_visuals, |mut state: ResMut<VoxelGridState>| *state = VoxelGridState::default()));
    }
}

#[derive(Component)]
struct VisualizerCube {
    initial_position: Vec3,
    frequency_band: usize,
}

/// This is our new "master" system for the grid.
/// It checks if the grid is out of sync with the config and rebuilds it if necessary.
fn manage_voxel_grid(
    mut commands: Commands,
    config: Res<VisualsConfig>,
    mut grid_state: ResMut<VoxelGridState>,
    // These are needed to spawn new cubes
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    // We need this to know which cubes to despawn
    cube_query: Query<Entity, With<VisualizerCube>>,
) {
    // If the number of bands in the config is different from our current grid state...
    if config.num_bands != grid_state.num_bands {
        // 1. Despawn all existing cubes.
        despawn_visuals(commands.reborrow(), cube_query);
        // 2. Spawn a new grid with the correct number of cubes.
        spawn_visuals(commands.reborrow(), meshes, materials, &config);
        // 3. Update our state to match the new grid.
        grid_state.num_bands = config.num_bands;
    }
}

/// A helper function to despawn all cubes.
fn despawn_visuals(
    mut commands: Commands,
    cube_query: Query<Entity, With<VisualizerCube>>
) {
    for entity in &cube_query {
        commands.entity(entity).despawn_recursive();
    }
}

/// A helper function to spawn a grid of cubes based on the current config.
fn spawn_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: &VisualsConfig,
) {
    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let cube_spacing = 1.5;
    let grid_size = config.num_bands;

    for x in 0..grid_size {
        for z in 0..grid_size {
            let x_pos = (x as f32 - grid_size as f32 / 2.0) * cube_spacing;
            let z_pos = (z as f32 - grid_size as f32 / 2.0) * cube_spacing;
            let initial_pos = Vec3::new(x_pos, 0.0, z_pos);

            let material = materials.add(StandardMaterial {
                base_color: Color::rgb(0.8, 0.7, 0.6),
                emissive: Color::BLACK,
                metallic: 1.0,
                perceptual_roughness: 0.1,
                ..default()
            });

            commands.spawn((
                PbrBundle {
                    mesh: cube_mesh.clone(),
                    material: material.clone(),
                    transform: Transform::from_translation(initial_pos),
                    ..default()
                },
                VisualizerCube {
                    initial_position: initial_pos,
                    frequency_band: x,
                },
            ));
        }
    }
}

/// The animation system remains largely the same.
fn update_3d_visuals(
    audio_analysis: Res<AudioAnalysis>,
    config: Res<VisualsConfig>,
    mut query: Query<(&mut Transform, &Handle<StandardMaterial>, &VisualizerCube)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if audio_analysis.frequency_bins.len() != config.num_bands {
        return;
    }

    let smoothing_factor = 0.2;

    for (mut transform, material_handle, cube) in &mut query {
        if let Some(band_amplitude) = audio_analysis.frequency_bins.get(cube.frequency_band) {
            let target_scale = 1.0 + band_amplitude * config.bass_sensitivity;
            transform.scale.y = transform.scale.y + (target_scale - transform.scale.y) * smoothing_factor;

            if config.spread_enabled {
                let spread_factor = 1.0 + (audio_analysis.treble_average * 0.1).min(1.5);
                transform.translation.x = cube.initial_position.x * spread_factor;
                transform.translation.z = cube.initial_position.z * spread_factor;
            } else {
                transform.translation.x = cube.initial_position.x;
                transform.translation.z = cube.initial_position.z;
            }

            let emissive_color = if config.bloom_enabled {
                let glow_intensity = (transform.scale.y - 1.0).max(0.0);
                config.bloom_color * glow_intensity * 2.0
            } else {
                Color::BLACK
            };

            if let Some(material) = materials.get_mut(material_handle) {
                material.emissive = emissive_color;
            }
        }
    }
}