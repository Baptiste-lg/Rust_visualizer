// src/viz_orb.rs

use bevy::prelude::*;
use crate::{AppState, audio::AudioAnalysis, config::VisualsConfig, VisualizationEnabled};

pub struct VizOrbPlugin;

impl Plugin for VizOrbPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::VisualizationOrb), setup_orb)
            .add_systems(Update, update_orb
                .run_if(in_state(AppState::VisualizationOrb))
                .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0)
            )
            .add_systems(OnExit(AppState::VisualizationOrb), despawn_orb_visuals);
    }
}

// Component to tag all entities belonging to the orb scene for easy cleanup
#[derive(Component)]
struct OrbVisual;

// Component for the parent entity that will rotate
#[derive(Component)]
struct OrbCenter;

// MODIFIED: This now represents a deformer that will push the orb's mesh outwards
#[derive(Component)]
struct OrbDeformer {
    band: usize,
    direction: Vec3,
}

fn setup_orb(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<VisualsConfig>,
) {
    // This is the parent entity that everything will rotate around.
    // It has no mesh itself.
    let parent = commands.spawn((
        SpatialBundle::default(),
        OrbCenter,
        OrbVisual,
    )).id();

    // MODIFIED: This is the visible sphere. It's a child of the rotating parent.
    // We will use a high-resolution IcoSphere for smoother deformation.
    let sphere_mesh = meshes.add(Sphere::new(1.0).mesh().ico(7).unwrap());

    commands.entity(parent).with_children(|p| {
        p.spawn(PbrBundle {
            mesh: sphere_mesh,
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.8, 0.7, 0.6),
                perceptual_roughness: 0.8,
                ..default()
            }),
            transform: Transform::from_scale(Vec3::splat(5.0)),
            ..default()
        });
    });

    // MODIFIED: These are no longer visible spikes. They are invisible deformers.
    // We create one deformer for each frequency band.
    let num_deformers = config.num_bands;
    for i in 0..num_deformers {
        // We use a mathematical formula to distribute points evenly on a sphere's surface.
        // This ensures the stretching effect is uniform around the orb.
        let (x, y, z) = {
            let golden_ratio = (1.0 + 5.0f32.sqrt()) / 2.0;
            let i_f = i as f32;
            let n_f = num_deformers as f32;
            let y_coord = 1.0 - (2.0 * i_f) / (n_f - 1.0);
            let radius = (1.0 - y_coord * y_coord).sqrt();
            let theta = golden_ratio * i_f * std::f32::consts::TAU; // Use TAU for full circle
            (theta.cos() * radius, y_coord, theta.sin() * radius)
        };

        // This is the direction the orb will stretch in for this frequency band.
        let direction = Vec3::new(x, y, z).normalize();

        // Spawn a deformer entity. It has no mesh or visuals.
        commands.spawn((
            SpatialBundle::default(),
            OrbDeformer { band: i, direction },
            OrbVisual,
        ));
    }
}

fn update_orb(
    time: Res<Time>,
    audio_analysis: Res<AudioAnalysis>,
    config: Res<VisualsConfig>,
    mut center_query: Query<&mut Transform, With<OrbCenter>>,
    // MODIFIED: We query for the OrbDeformers and the single sphere mesh handle
    deformer_query: Query<&OrbDeformer>,
    mut meshes: ResMut<Assets<Mesh>>,
    // We query for the mesh handle on the visible sphere
    sphere_query: Query<&Handle<Mesh>, (Without<OrbCenter>, Without<OrbDeformer>)>,
) {
    // Rotate the parent entity, which makes the whole orb spin
    if let Ok(mut transform) = center_query.get_single_mut() {
        transform.rotate_y(time.delta_seconds() * 0.1);
        transform.rotate_x(time.delta_seconds() * 0.05);
    }

    if audio_analysis.frequency_bins.len() != config.num_bands {
        return;
    }

    // Get the actual mesh asset so we can modify it
    let Ok(mesh_handle) = sphere_query.get_single() else { return };
    let Some(mesh) = meshes.get_mut(mesh_handle) else { return };
    let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) else { return };

    // This vector will store the total "push" for each vertex in the mesh
    let mut offsets = vec![Vec3::ZERO; positions.len()];

    // For each deformer, calculate how much it should push the mesh vertices
    for deformer in &deformer_query {
        if let Some(amplitude) = audio_analysis.frequency_bins.get(deformer.band) {
            // The strength of the stretch is based on the music's amplitude
            let stretch_strength = (amplitude * config.bass_sensitivity * 0.1).clamp(0.0, 1.0);

            if stretch_strength < 0.01 { continue; } // Skip if no sound

            // Go through every vertex in the sphere's mesh
            for i in 0..positions.len() {
                let vertex_pos = Vec3::from_slice(&positions[i]).normalize();

                // We calculate how "aligned" the vertex is with the deformer's direction.
                // A vertex directly in line with the deformer gets the full push.
                // A vertex on the side gets less push.
                let alignment = vertex_pos.dot(deformer.direction).clamp(0.0, 1.0);

                // The final offset is the deformer's direction multiplied by its strength and alignment.
                // We use a power of 4 to make the stretch more focused and "bumpy".
                offsets[i] += deformer.direction * stretch_strength * alignment.powf(4.0);
            }
        }
    }

    // Finally, apply the calculated offsets to each vertex position
    for i in 0..positions.len() {
        // We start with the original vertex position (normalized to form a perfect sphere)
        // and add the calculated offset to it.
        let base_pos = Vec3::from_slice(&positions[i]).normalize();
        let new_pos = base_pos + offsets[i];
        positions[i] = new_pos.to_array();
    }
}


fn despawn_orb_visuals(mut commands: Commands, query: Query<Entity, With<OrbVisual>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}