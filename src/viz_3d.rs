// src/viz_3d.rs

use bevy::prelude::*;
use crate::{AppState, audio::AudioAnalysis, config::VisualsConfig, VisualizationEnabled};

pub struct Viz3DPlugin;

impl Plugin for Viz3DPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::Visualization3D), spawn_visuals)
            .add_systems(Update, (
                update_3d_visuals,
                // MODIFIED: Corrected the run_if condition. No extra parentheses needed.
                respawn_visuals_on_change.run_if(resource_changed::<VisualsConfig>),
            )
                .run_if(in_state(AppState::Visualization3D))
                .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0)
            );
    }
}

#[derive(Component)]
struct VisualizerCube {
    initial_position: Vec3,
    frequency_band: usize,
}

fn despawn_visuals(
    mut commands: Commands,
    cube_query: Query<Entity, With<VisualizerCube>>
) {
    for entity in &cube_query {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<VisualsConfig>,
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
                    material: material,
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

fn respawn_visuals_on_change(
    mut commands: Commands,
    cube_query: Query<Entity, With<VisualizerCube>>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    config: Res<VisualsConfig>,
) {
    // This system only runs when the config has changed, so we can directly respawn.
    despawn_visuals(commands.reborrow(), cube_query);
    spawn_visuals(commands, meshes, materials, config);
}


fn update_3d_visuals(
    audio_analysis: Res<AudioAnalysis>,
    config: Res<VisualsConfig>,
    mut query: Query<(&mut Transform, &Handle<StandardMaterial>, &VisualizerCube)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if audio_analysis.frequency_bins.len() != config.num_bands {
        return;
    }

    let spread_factor = 1.0 + (audio_analysis.treble_average * 0.1).min(1.5);
    let smoothing_factor = 0.2;

    for (mut transform, material_handle, cube) in &mut query {
        if let Some(band_amplitude) = audio_analysis.frequency_bins.get(cube.frequency_band) {
            // Using bass_sensitivity here as a general "amplitude sensitivity"
            let target_scale = 1.0 + band_amplitude * config.bass_sensitivity;
            transform.scale.y = transform.scale.y + (target_scale - transform.scale.y) * smoothing_factor;

            transform.translation.x = cube.initial_position.x * spread_factor;
            transform.translation.z = cube.initial_position.z * spread_factor;

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