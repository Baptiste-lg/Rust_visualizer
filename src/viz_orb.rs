// src/viz_orb.rs

use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::math::primitives::Capsule3d;
use crate::{AppState, audio::AudioAnalysis, config::VisualsConfig, VisualizationEnabled};

pub struct VizOrbPlugin;

// This resource's only job is to track if the config has changed.
#[derive(Resource, Default)]
struct OrbState {
    num_bands: usize,
    base_color: Color,
    peak_color: Color,
}

impl Plugin for VizOrbPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<OrbState>()
            .add_systems(OnEnter(AppState::VisualizationOrb), setup_orb)
            .add_systems(Update, (
                manage_orb,
                update_orb_arms.after(manage_orb),
            )
                .run_if(in_state(AppState::VisualizationOrb))
                .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0)
            )
            .add_systems(OnExit(AppState::VisualizationOrb), despawn_orb_visuals);
    }
}

// A component to tag all entities related to the orb for easy cleanup
#[derive(Component)]
struct OrbVisual;

// A component for the extending arms
#[derive(Component)]
struct OrbArm {
    band: usize,
}

fn manage_orb(
    mut commands: Commands,
    config: Res<VisualsConfig>,
    mut orb_state: ResMut<OrbState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<Entity, With<OrbVisual>>,
) {
    if config.num_bands != orb_state.num_bands ||
       config.orb_base_color != orb_state.base_color ||
       config.orb_peak_color != orb_state.peak_color
    {
        info!("Orb config changed. Rebuilding...");
        for entity in &query {
            commands.entity(entity).despawn_recursive();
        }
        spawn_orb_entities(&mut commands, &mut meshes, &mut materials, &config);
        orb_state.num_bands = config.num_bands;
        orb_state.base_color = config.orb_base_color;
        orb_state.peak_color = config.orb_peak_color;
    }
}

fn setup_orb(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<VisualsConfig>,
    mut orb_state: ResMut<OrbState>,
) {
    spawn_orb_entities(&mut commands, &mut meshes, &mut materials, &config);
    orb_state.num_bands = config.num_bands;
    orb_state.base_color = config.orb_base_color;
    orb_state.peak_color = config.orb_peak_color;
}

// This function now creates a central sphere and separate, custom-colored arms.
fn spawn_orb_entities(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    config: &VisualsConfig,
) {
    // Create the material that will be shared by the arms.
    let arm_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.6,
        metallic: 0.2,
        ..default()
    });

    // Spawn the central, non-deforming sphere.
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Sphere::new(1.0).mesh().ico(5).unwrap()),
            material: materials.add(StandardMaterial {
                base_color: config.orb_base_color,
                perceptual_roughness: 0.6,
                metallic: 0.2,
                ..default()
            }),
            transform: Transform::from_scale(Vec3::splat(3.0)),
            ..default()
        },
        OrbVisual,
    ));

    let num_arms = config.num_bands;
    for i in 0..num_arms {
        let mut arm_mesh: Mesh = Capsule3d::new(0.5, 1.0).into();

        if let Some(VertexAttributeValues::Float32x3(positions)) = arm_mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            let mut colors = Vec::with_capacity(positions.len());
            let min_y = positions.iter().map(|p| p[1]).reduce(f32::min).unwrap_or(-1.0);
            let max_y = positions.iter().map(|p| p[1]).reduce(f32::max).unwrap_or(1.0);

            for pos in positions {
                let y_pos = pos[1];
                let lerp_factor = (y_pos - min_y) / (max_y - min_y);
                let color_lerp_factor = lerp_factor.powf(3.0);

                let base = config.orb_base_color;
                let peak = config.orb_peak_color;

                let r = base.r() + (peak.r() - base.r()) * color_lerp_factor;
                let g = base.g() + (peak.g() - base.g()) * color_lerp_factor;
                let b = base.b() + (peak.b() - base.b()) * color_lerp_factor;
                let a = base.a() + (peak.a() - base.a()) * color_lerp_factor;

                colors.push([r, g, b, a]);
            }
            arm_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        }

        let (x, y, z) = {
            let golden_ratio = (1.0 + 5.0f32.sqrt()) / 2.0;
            let i_f = i as f32;
            let n_f = num_arms as f32;
            let y_coord = 1.0 - (2.0 * i_f) / (n_f - 1.0);
            let radius = (1.0 - y_coord * y_coord).sqrt();
            let theta = golden_ratio * i_f * std::f32::consts::TAU;
            (theta.cos() * radius, y_coord, theta.sin() * radius)
        };

        let direction = Vec3::new(x, y, z).normalize();

        let rotation = Quat::from_rotation_arc(Vec3::Y, direction);
        let translation = direction * 1.0;
        let transform = Transform {
            translation,
            rotation,
            scale: Vec3::new(1.0, 0.01, 1.0),
        };

        commands.spawn((
            PbrBundle {
                mesh: meshes.add(arm_mesh),
                material: arm_material.clone(),
                transform,
                ..default()
            },
            OrbArm { band: i },
            OrbVisual,
        ));
    }
}

fn update_orb_arms(
    audio_analysis: Res<AudioAnalysis>,
    config: Res<VisualsConfig>,
    mut arm_query: Query<(&mut Transform, &OrbArm)>,
) {
    if audio_analysis.frequency_bins.len() != config.num_bands { return; }

    for (mut transform, arm) in arm_query.iter_mut() {
        if let Some(amplitude) = audio_analysis.frequency_bins.get(arm.band) {
            let target_scale_y = (amplitude.sqrt() * config.bass_sensitivity * 0.8).clamp(0.01, 4.0);

            let smoothing_factor = config.orb_smoothing;
            transform.scale.y = transform.scale.y + (target_scale_y - transform.scale.y) * smoothing_factor;
        }
    }
}


fn despawn_orb_visuals(
    mut commands: Commands,
    query: Query<Entity, With<OrbVisual>>,
    mut orb_state: ResMut<OrbState>,
) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
    *orb_state = OrbState::default();
}