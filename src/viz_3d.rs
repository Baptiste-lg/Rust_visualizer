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

// MODIFIED: This component now stores the cube's original position and its associated frequency band.
#[derive(Component)]
struct VisualizerCube {
    initial_position: Vec3,
    frequency_band: usize,
}

// MODIFIED: We now assign a frequency band to each cube when creating the scene.
fn setup_3d_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Setting up 3D visuals...");

    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let cube_spacing = 2.0;
    let grid_size = 8; // Should match NUM_BANDS in audio.rs

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
                // Each cube in a row along the x-axis is linked to the same frequency band.
                VisualizerCube {
                    initial_position: initial_pos,
                    frequency_band: x,
                },
            ));
        }
    }
}

// MODIFIED: The entire animation logic is new, using multiple frequency bands.
fn update_3d_visuals(
    audio_analysis: Res<AudioAnalysis>,
    config: Res<VisualsConfig>,
    mut query: Query<(&mut Transform, &Handle<StandardMaterial>, &VisualizerCube)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if audio_analysis.frequency_bins.is_empty() {
        return;
    }

    // The treble will control how far the cubes spread out.
    let spread_factor = 1.0 + (audio_analysis.treble_average * 0.1).min(1.5);
    let smoothing_factor = 0.2;

    for (mut transform, material_handle, cube) in &mut query {
        // 1. Cube Height: Determined by the amplitude of its assigned frequency band.
        let band_amplitude = audio_analysis.frequency_bins[cube.frequency_band];
        let target_scale = 1.0 + band_amplitude * 4.0;
        transform.scale.y = transform.scale.y + (target_scale - transform.scale.y) * smoothing_factor;

        // 2. Cube Spacing: Cubes are pushed away from the center based on the treble.
        transform.translation.x = cube.initial_position.x * spread_factor;
        transform.translation.z = cube.initial_position.z * spread_factor;

        // 3. Cube Glow: Controlled by the cube's height and the bloom color from the config.
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