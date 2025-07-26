// src/viz_orb.rs

use bevy::{
    prelude::*,
    render::mesh::{Mesh, VertexAttributeValues},
};
use noise::{NoiseFn, Perlin};
use crate::{AppState, audio::AudioAnalysis, config::VisualsConfig, VisualizationEnabled};

pub struct VizOrbPlugin;

// Un tag pour tous les éléments visuels de l'orbe
#[derive(Component)]
struct OrbVisual;

// Un composant pour stocker l'état de notre orbe déformable
#[derive(Component)]
struct DeformableOrb {
    original_vertices: Vec<[f32; 3]>,
    noise: Perlin,
}

impl Plugin for VizOrbPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::VisualizationOrb), setup_orb)
            .add_systems(Update,
                deform_orb
                 .run_if(in_state(AppState::VisualizationOrb))
                 .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0)
            )
            .add_systems(OnExit(AppState::VisualizationOrb), despawn_orb_visuals);
    }
}

fn setup_orb(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<VisualsConfig>,
) {
    // 1. On crée le maillage de base
    let mut sphere_mesh = Sphere::new(3.0).mesh().ico(5).unwrap();

    // 2. ON LE DÉPLIE IMMÉDIATEMENT pour le rendre compatible avec compute_flat_normals
    sphere_mesh.duplicate_vertices();
    sphere_mesh.compute_flat_normals();

    // 3. On extrait les positions originales du maillage DÉPLIÉ
    let original_vertices = match sphere_mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(vertices)) => vertices.clone(),
        _ => Vec::new(),
    };

    // On crée l'entité de la sphère
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(sphere_mesh), // On ajoute le maillage déjà préparé
            material: materials.add(StandardMaterial {
                base_color: config.orb_base_color,
                perceptual_roughness: 0.8,
                metallic: 0.2,
                emissive: config.orb_base_color,
                ..default()
            }),
            ..default()
        },
        DeformableOrb {
            original_vertices,
            noise: Perlin::new(1),
        },
        OrbVisual,
    ));
}

fn deform_orb(
    time: Res<Time>,
    config: Res<VisualsConfig>,
    audio_analysis: Res<AudioAnalysis>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut query: Query<(&Handle<Mesh>, &Handle<StandardMaterial>, &DeformableOrb)>,
) {
    if audio_analysis.frequency_bins.is_empty() { return; }

    let total_bass_amplitude = audio_analysis.frequency_bins[0..config.num_bands/4]
        .iter().sum::<f32>() / (config.num_bands/4) as f32;

    for (mesh_handle, material_handle, orb) in &mut query {
        if let Some(mesh) = meshes.get_mut(mesh_handle) {
            let vertices = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION).unwrap();

            if let VertexAttributeValues::Float32x3(vertex_data) = vertices {
                if vertex_data.len() != orb.original_vertices.len() {
                    continue;
                }

                for i in 0..vertex_data.len() {
                    let original_pos = Vec3::from_array(orb.original_vertices[i]);
                    let normalized_pos = original_pos.normalize();

                    let time_val = time.elapsed_seconds() * config.orb_noise_speed;
                    let treble_factor = 1.0 + audio_analysis.treble_average * config.orb_treble_influence;
                    let noise_frequency = config.orb_noise_frequency * treble_factor;

                    let noise_input = (normalized_pos * noise_frequency) + time_val;
                    let noise_value = orb.noise.get([
                        noise_input.x as f64,
                        noise_input.y as f64,
                        noise_input.z as f64,
                    ]) as f32;

                    let displacement = noise_value * total_bass_amplitude * config.bass_sensitivity;
                    let new_pos = original_pos + normalized_pos * displacement;
                    vertex_data[i] = new_pos.into();
                }
            }

            // Le maillage étant déjà déplié, cet appel est maintenant SANS DANGER.
            mesh.compute_flat_normals();
        }

        if let Some(material) = materials.get_mut(material_handle) {
            let emissive_intensity = (total_bass_amplitude * 2.0).clamp(0.0, 5.0);
            material.emissive = config.orb_peak_color * emissive_intensity;
        }
    }
}

fn despawn_orb_visuals(
    mut commands: Commands,
    query: Query<Entity, With<OrbVisual>>,
) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}