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
            .add_systems(OnExit(AppState::VisualizationOrb), despawn_orb);
    }
}

#[derive(Component)]
struct Orb;

#[derive(Component)]
struct OrbSpike {
    band: usize,
    angle: f32,
}

fn setup_orb(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<VisualsConfig>,
) {
    // Main Orb
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Sphere::new(1.0).mesh().ico(5).unwrap()),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.8, 0.7, 0.6),
                ..default()
            }),
            transform: Transform::from_scale(Vec3::splat(5.0)),
            ..default()
        },
        Orb,
    ));

    // Spikes for each band
    let num_spikes = config.num_bands;
    for i in 0..num_spikes {
        let angle = i as f32 * (2.0 * std::f32::consts::PI / num_spikes as f32);
        let x = angle.cos();
        let y = angle.sin();

        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Sphere::new(1.0).mesh().ico(3).unwrap()),
                material: materials.add(StandardMaterial {
                    base_color: Color::rgb(1.0, 0.2, 0.2),
                    ..default()
                }),
                transform: Transform::from_translation(Vec3::new(x, y, 0.0) * 6.0),
                ..default()
            },
            OrbSpike { band: i, angle },
        ));
    }
}

fn update_orb(
    time: Res<Time>,
    audio_analysis: Res<AudioAnalysis>,
    config: Res<VisualsConfig>,
    mut orb_query: Query<&mut Transform, (With<Orb>, Without<OrbSpike>)>,
    mut spike_query: Query<(&mut Transform, &OrbSpike), With<OrbSpike>>,
) {
    // Rotate the main orb
    for mut transform in orb_query.iter_mut() {
        transform.rotate_y(time.delta_seconds() * 0.1);
        transform.rotate_x(time.delta_seconds() * 0.05);
    }

    if audio_analysis.frequency_bins.len() != config.num_bands {
        return;
    }

    // Update spikes based on audio
    for (mut transform, spike) in spike_query.iter_mut() {
        if let Some(amplitude) = audio_analysis.frequency_bins.get(spike.band) {
            let scale = 0.5 + amplitude * 2.0;
            transform.scale = Vec3::splat(scale.clamp(0.1, 5.0));

            let radius = 6.0 + amplitude * 1.5;
            let x = spike.angle.cos() * radius;
            let y = spike.angle.sin() * radius;
            transform.translation = Vec3::new(x, y, 0.0);
        }
    }
}

fn despawn_orb(
    mut commands: Commands,
    orb_query: Query<Entity, With<Orb>>,
    spike_query: Query<Entity, With<OrbSpike>>,
) {
    for entity in orb_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in spike_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}