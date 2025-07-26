// src/viz_3d.rs

use bevy::prelude::*;
use crate::{AppState, audio::AudioAnalysis, config::VisualsConfig, VisualizationEnabled};

pub struct Viz3DPlugin;

// MODIFIÉ : L'état suit maintenant aussi la taille des colonnes
#[derive(Resource, Default)]
struct VoxelGridState {
    num_bands: usize,
    base_color: Color,
    column_size: usize,
}

// AJOUTÉ : Une nouvelle ressource pour stocker les matériaux de chaque colonne
#[derive(Resource, Default)]
struct ColumnMaterials(Vec<Handle<StandardMaterial>>);

impl Plugin for Viz3DPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<VoxelGridState>()
            // AJOUTÉ : On initialise la nouvelle ressource
            .init_resource::<ColumnMaterials>()
            .add_systems(Update, (
                manage_voxel_grid,
                // MODIFIÉ : La mise à jour des visuels est maintenant séparée en deux étapes
                update_column_materials.after(manage_voxel_grid),
                update_cube_transforms.after(update_column_materials)
            )
                .run_if(in_state(AppState::Visualization3D))
                .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0)
            )
            .add_systems(OnExit(AppState::Visualization3D), (despawn_visuals, |mut state: ResMut<VoxelGridState>| *state = VoxelGridState::default()));
    }
}

#[derive(Component)]
struct VisualizerCube {
    initial_position: Vec3,
    frequency_band: usize,
}

fn manage_voxel_grid(
    mut commands: Commands,
    config: Res<VisualsConfig>,
    mut grid_state: ResMut<VoxelGridState>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    cube_query: Query<Entity, With<VisualizerCube>>,
) {
    // MODIFIÉ : On vérifie aussi si la taille des colonnes a changé
    if config.num_bands != grid_state.num_bands
        || config.viz3d_base_color != grid_state.base_color
        || config.viz3d_column_size != grid_state.column_size
    {
        info!("3D visual config changed. Rebuilding voxel grid...");
        despawn_visuals(commands.reborrow(), cube_query);
        spawn_visuals(commands.reborrow(), meshes, materials, &config);
        grid_state.num_bands = config.num_bands;
        grid_state.base_color = config.viz3d_base_color;
        grid_state.column_size = config.viz3d_column_size;
    }
}

fn despawn_visuals(
    mut commands: Commands,
    cube_query: Query<Entity, With<VisualizerCube>>
) {
    for entity in &cube_query {
        commands.entity(entity).despawn_recursive();
    }
    // AJOUTÉ : On vide aussi notre ressource de matériaux
    commands.insert_resource(ColumnMaterials::default());
}

// MODIFIÉ : La fonction de création a été adaptée pour les colonnes et l'optimisation
fn spawn_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: &VisualsConfig,
) {
    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let cube_spacing = 1.5;
    let num_bands = config.num_bands;
    let column_size = config.viz3d_column_size;

    let mut column_materials_vec = Vec::with_capacity(num_bands);

    for x in 0..num_bands {
        let x_pos = (x as f32 - num_bands as f32 / 2.0) * cube_spacing;

        // On crée UN SEUL matériau par colonne
        let material = materials.add(StandardMaterial {
            base_color: config.viz3d_base_color,
            emissive: Color::BLACK,
            metallic: 1.0,
            perceptual_roughness: 0.1,
            ..default()
        });
        column_materials_vec.push(material.clone());

        for z in 0..column_size {
            let z_pos = (z as f32 - column_size as f32 / 2.0) * cube_spacing;
            let initial_pos = Vec3::new(x_pos, 0.0, z_pos);

            commands.spawn((
                PbrBundle {
                    mesh: cube_mesh.clone(),
                    // On clone la "poignée" (Handle) du matériau, pas le matériau lui-même. C'est très léger.
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
    // On stocke les poignées des matériaux dans notre ressource
    commands.insert_resource(ColumnMaterials(column_materials_vec));
}

// AJOUTÉ : Une nouvelle fonction pour mettre à jour les matériaux (très rapide)
fn update_column_materials(
    audio_analysis: Res<AudioAnalysis>,
    config: Res<VisualsConfig>,
    column_materials: Res<ColumnMaterials>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if audio_analysis.frequency_bins.len() != config.num_bands {
        return;
    }

    let afr = &audio_analysis.frequency_bins;

    for band_index in 0..config.num_bands {
        if let (Some(material_handle), Some(amplitude)) = (column_materials.0.get(band_index), afr.get(band_index)) {
            if let Some(material) = materials.get_mut(material_handle) {
                let scale_y = 1.0 + amplitude * config.bass_sensitivity;
                material.emissive = if config.bloom_enabled {
                    let glow_intensity = (scale_y - 1.0).max(0.0);
                    config.bloom_color * glow_intensity * 2.0
                } else {
                    Color::BLACK
                };
            }
        }
    }
}

// AJOUTÉ : Une fonction dédiée pour mettre à jour la position et la taille des cubes
fn update_cube_transforms(
    audio_analysis: Res<AudioAnalysis>,
    config: Res<VisualsConfig>,
    mut query: Query<(&mut Transform, &VisualizerCube)>,
) {
    if audio_analysis.frequency_bins.len() != config.num_bands {
        return;
    }

    let smoothing_factor = 0.2;

    for (mut transform, cube) in &mut query {
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
        }
    }
}