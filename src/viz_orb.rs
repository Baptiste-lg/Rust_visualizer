// src/viz_orb.rs

use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use crate::{AppState, audio::AudioAnalysis, config::VisualsConfig, VisualizationEnabled};

pub struct VizOrbPlugin;

#[derive(Resource, Default)]
struct OriginalOrbMesh {
    positions: Vec<[f32; 3]>,
}

#[derive(Resource, Default)]
struct OrbState {
    num_bands: usize,
    // ADDED: We also track the colors to know when to update the material.
    base_color: Color,
    peak_color: Color,
}

impl Plugin for VizOrbPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<OrbState>()
            .init_resource::<OriginalOrbMesh>()
            .add_systems(OnEnter(AppState::VisualizationOrb), setup_orb)
            .add_systems(Update, (
                manage_orb,
                update_orb.after(manage_orb),
            )
                .run_if(in_state(AppState::VisualizationOrb))
                .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0)
            )
            .add_systems(OnExit(AppState::VisualizationOrb), despawn_orb_visuals);
    }
}

#[derive(Component)]
struct OrbVisual;

#[derive(Component)]
struct OrbDeformer {
    band: usize,
    direction: Vec3,
}

fn manage_orb(
    mut commands: Commands,
    config: Res<VisualsConfig>,
    mut orb_state: ResMut<OrbState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut original_mesh: ResMut<OriginalOrbMesh>,
    query: Query<Entity, With<OrbVisual>>,
) {
    // MODIFIED: Rebuild the orb if the number of bands OR the colors change.
    if config.num_bands != orb_state.num_bands ||
       config.orb_base_color != orb_state.base_color ||
       config.orb_peak_color != orb_state.peak_color
    {
        info!("Orb config changed. Rebuilding...");
        for entity in &query {
            commands.entity(entity).despawn_recursive();
        }
        spawn_orb_entities(&mut commands, &mut meshes, &mut materials, &mut original_mesh, &config);
        orb_state.num_bands = config.num_bands;
        orb_state.base_color = config.orb_base_color;
        orb_state.peak_color = config.orb_peak_color;
    }
}

fn setup_orb(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut original_mesh: ResMut<OriginalOrbMesh>,
    config: Res<VisualsConfig>,
    mut orb_state: ResMut<OrbState>,
) {
    spawn_orb_entities(&mut commands, &mut meshes, &mut materials, &mut original_mesh, &config);
    orb_state.num_bands = config.num_bands;
    orb_state.base_color = config.orb_base_color;
    orb_state.peak_color = config.orb_peak_color;
}

fn spawn_orb_entities(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    original_mesh: &mut ResMut<OriginalOrbMesh>,
    config: &VisualsConfig,
) {
    let mut sphere = Sphere::new(1.0).mesh().ico(7).unwrap();

    if let Some(VertexAttributeValues::Float32x3(positions)) = sphere.attribute(Mesh::ATTRIBUTE_POSITION) {
        original_mesh.positions = positions.clone();
        // ADDED: Create a new vertex color attribute, initializing all vertices to the base color.
        let colors = vec![config.orb_base_color.as_rgba_f32(); positions.len()];
        sphere.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }

    let sphere_mesh = meshes.add(sphere);

    commands.spawn((
        PbrBundle {
            mesh: sphere_mesh,
            material: materials.add(StandardMaterial {
                // MODIFIED: Set the material's base color to white.
                // This tells the shader to use the vertex colors from the mesh directly.
                base_color: Color::WHITE,
                perceptual_roughness: 0.8,
                ..default()
            }),
            transform: Transform::from_scale(Vec3::splat(5.0)),
            ..default()
        },
        OrbVisual,
    ));

    let num_deformers = config.num_bands;
    for i in 0..num_deformers {
        let (x, y, z) = {
            let golden_ratio = (1.0 + 5.0f32.sqrt()) / 2.0;
            let i_f = i as f32;
            let n_f = num_deformers as f32;
            let y_coord = 1.0 - (2.0 * i_f) / (n_f - 1.0);
            let radius = (1.0 - y_coord * y_coord).sqrt();
            let theta = golden_ratio * i_f * std::f32::consts::TAU;
            (theta.cos() * radius, y_coord, theta.sin() * radius)
        };

        let direction = Vec3::new(x, y, z).normalize();

        commands.spawn((
            SpatialBundle::default(),
            OrbDeformer { band: i, direction },
            OrbVisual,
        ));
    }
}


fn update_orb(
    audio_analysis: Res<AudioAnalysis>,
    config: Res<VisualsConfig>,
    deformer_query: Query<&OrbDeformer>,
    mut meshes: ResMut<Assets<Mesh>>,
    sphere_query: Query<&Handle<Mesh>, With<OrbVisual>>,
    original_mesh: Res<OriginalOrbMesh>,
) {
    if audio_analysis.frequency_bins.len() != config.num_bands || original_mesh.positions.is_empty() {
        return;
    }

    let Ok(mesh_handle) = sphere_query.get_single() else { return };
    let Some(mesh) = meshes.get_mut(mesh_handle) else { return };

    // Get mutable access to both the position and color attributes of the mesh.
    let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) else { return };
    let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR) else { return };

    let mut offsets = vec![Vec3::ZERO; original_mesh.positions.len()];

    for deformer in &deformer_query {
        if let Some(amplitude) = audio_analysis.frequency_bins.get(deformer.band) {
            let stretch_strength = (amplitude * config.bass_sensitivity * 0.15).clamp(0.0, 1.5);
            if stretch_strength < 0.01 { continue; }

            for i in 0..original_mesh.positions.len() {
                let vertex_pos = Vec3::from_slice(&original_mesh.positions[i]).normalize();
                let alignment = vertex_pos.dot(deformer.direction).clamp(0.0, 1.0);
                offsets[i] += deformer.direction * stretch_strength * alignment.powf(2.5);
            }
        }
    }

    let smoothing_factor = 0.1;
    for i in 0..positions.len() {
        let base_pos = Vec3::from_slice(&original_mesh.positions[i]);
        let target_pos = base_pos + offsets[i];
        let current_pos = Vec3::from_slice(&positions[i]);
        positions[i] = current_pos.lerp(target_pos, smoothing_factor).to_array();

        // ADDED: This block handles the color blending.
        // The amount of displacement determines the color.
        let displacement = offsets[i].length();
        // Normalize the displacement to a 0.0-1.0 range to use for interpolation.
        let color_lerp_factor = (displacement / 0.75).clamp(0.0, 1.0);

        // Interpolate between the base and peak colors.
        let base_color = config.orb_base_color;
        let peak_color = config.orb_peak_color;
        let final_color = base_color.lerp(peak_color, color_lerp_factor);
        colors[i] = final_color.as_rgba_f32();
    }
}


fn despawn_orb_visuals(
    mut commands: Commands,
    query: Query<Entity, With<OrbVisual>>,
    mut orb_state: ResMut<OrbState>,
    mut original_mesh: ResMut<OriginalOrbMesh>
) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
    *orb_state = OrbState::default();
    *original_mesh = OriginalOrbMesh::default();
}