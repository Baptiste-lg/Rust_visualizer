// src/viz_orb.rs

use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::math::primitives::Capsule3d;
use crate::{AppState, audio::AudioAnalysis, config::VisualsConfig, VisualizationEnabled};
use std::time::Duration;

pub struct VizOrbPlugin;

// Un tag pour tous les éléments visuels de l'orbe
#[derive(Component)]
struct OrbVisual;

// Un tag pour le projectile
#[derive(Component)]
struct OrbProjectile {
    direction: Vec3,
    lifespan: Timer,
}

// Ressource pour stocker les directions et les cooldowns de tir
#[derive(Resource)]
struct OrbProjetileInfo {
    directions: Vec<Vec3>,
    cooldowns: Vec<Timer>,
}

impl Plugin for VizOrbPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::VisualizationOrb), setup_orb)
            .add_systems(Update, (
                    spawn_orb_projectiles,
                    move_and_cull_projectiles
                ).run_if(in_state(AppState::VisualizationOrb))
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
    // Crée la sphère centrale
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

    // Pré-calcule les directions et initialise les timers de cooldown
    let num_bands = config.num_bands;
    let mut directions = Vec::with_capacity(num_bands);
    let mut cooldowns = Vec::with_capacity(num_bands);

    for i in 0..num_bands {
        let golden_ratio = (1.0 + 5.0f32.sqrt()) / 2.0;
        let i_f = i as f32;
        let n_f = num_bands as f32;
        let y_coord = 1.0 - (2.0 * i_f) / (n_f - 1.0);
        let radius = (1.0 - y_coord * y_coord).sqrt();
        let theta = golden_ratio * i_f * std::f32::consts::TAU;

        let direction = Vec3::new(theta.cos() * radius, y_coord, theta.sin() * radius).normalize();
        directions.push(direction);
        cooldowns.push(Timer::from_seconds(0.1, TimerMode::Once));
    }

    commands.insert_resource(OrbProjetileInfo { directions, cooldowns });
}

fn spawn_orb_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<VisualsConfig>,
    audio_analysis: Res<AudioAnalysis>,
    mut projectile_info: ResMut<OrbProjetileInfo>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if audio_analysis.frequency_bins.len() != config.num_bands {
        return;
    }

    // Si le nombre de bandes a changé, on doit recréer les directions/cooldowns
    if config.num_bands != projectile_info.directions.len() {
         let num_bands = config.num_bands;
        let mut directions = Vec::with_capacity(num_bands);
        let mut cooldowns = Vec::with_capacity(num_bands);

        for i in 0..num_bands {
            let golden_ratio = (1.0 + 5.0f32.sqrt()) / 2.0;
            let i_f = i as f32;
            let n_f = num_bands as f32;
            let y_coord = 1.0 - (2.0 * i_f) / (n_f - 1.0);
            let radius = (1.0 - y_coord * y_coord).sqrt();
            let theta = golden_ratio * i_f * std::f32::consts::TAU;

            let direction = Vec3::new(theta.cos() * radius, y_coord, theta.sin() * radius).normalize();
            directions.push(direction);
            cooldowns.push(Timer::from_seconds(0.1, TimerMode::Once));
        }
        projectile_info.directions = directions;
        projectile_info.cooldowns = cooldowns;
    }


    // On tick tous les cooldowns
    for timer in projectile_info.cooldowns.iter_mut() {
        timer.tick(time.delta());
    }

    // Crée une seule fois le mesh et le matériau pour les projectiles
    let mut arm_mesh: Mesh = Capsule3d::new(0.2, 1.5).into();
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
    let mesh_handle = meshes.add(arm_mesh);
    let material_handle = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: config.orb_peak_color * 2.0, // Pour le bloom
        ..default()
    });


    // On parcourt les bandes de fréquence
    for i in 0..config.num_bands {
        if let (Some(amplitude), Some(cooldown), Some(direction)) = (
            audio_analysis.frequency_bins.get(i),
            projectile_info.cooldowns.get_mut(i),
            projectile_info.directions.get(i),
        ) {
            // Si le son est assez fort et que le cooldown est terminé
            if *amplitude > config.orb_activation_threshold && cooldown.finished() {

                let rotation = Quat::from_rotation_arc(Vec3::Y, *direction);
                let transform = Transform::from_rotation(rotation)
                                    .with_translation(*direction * 2.0); // Commence à la surface de l'orbe

                commands.spawn((
                    PbrBundle {
                        mesh: mesh_handle.clone(),
                        material: material_handle.clone(),
                        transform,
                        ..default()
                    },
                    OrbProjectile {
                        direction: *direction,
                        lifespan: Timer::from_seconds(config.orb_arm_lifespan, TimerMode::Once),
                    },
                    OrbVisual,
                ));

                // On réinitialise le cooldown
                cooldown.reset();
            }
        }
    }
}

fn move_and_cull_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<VisualsConfig>,
    mut projectile_query: Query<(Entity, &mut Transform, &mut OrbProjectile)>,
) {
    for (entity, mut transform, mut projectile) in &mut projectile_query {
        // On tick le timer de durée de vie
        projectile.lifespan.tick(time.delta());

        // Si le timer est fini, on supprime le projectile
        if projectile.lifespan.finished() {
            commands.entity(entity).despawn();
        } else {
            // Sinon, on le déplace
            transform.translation += projectile.direction * config.orb_arm_speed * time.delta_seconds();
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
    // On retire la ressource pour éviter les soucis au prochain lancement
    commands.remove_resource::<OrbProjetileInfo>();
}