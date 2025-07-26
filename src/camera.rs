// src/camera.rs

use bevy::{
    core_pipeline::{
        bloom::BloomSettings,
        experimental::taa::TemporalAntiAliasBundle
    },
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    window::PrimaryWindow
};
use bevy_egui::EguiContexts;
use crate::{AppState, config::VisualsConfig};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::Visualization3D), setup_3d_scene_and_camera)
            .add_systems(Update, (
                pan_orbit_camera,
                update_bloom_settings
            ).run_if(in_state(AppState::Visualization3D)));
    }
}

#[derive(Component)]
pub struct PanOrbitCamera {
    pub focus: Vec3,
    pub radius: f32,
}

impl Default for PanOrbitCamera {
    fn default() -> Self {
        PanOrbitCamera {
            focus: Vec3::ZERO,
            radius: 15.0,
        }
    }
}

fn setup_3d_scene_and_camera(mut commands: Commands) {
    let initial_transform = Transform::from_xyz(0.0, 0.0, 15.0)
        .looking_at(Vec3::ZERO, Vec3::Y);

    commands.spawn((
        Camera3dBundle {
            transform: initial_transform,
            // MODIFIED: We now use more advanced bloom settings for a better visual effect.
            camera: Camera {
                hdr: true, // 1. HDR is required for bloom.
                ..default()
            },
            ..default()
        },
        // 2. We initialize the bloom settings here.
        BloomSettings::default(),
        PanOrbitCamera::default(),
        TemporalAntiAliasBundle::default(),
    ));

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 2000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
}

// MODIFIED: This system now cleanly enables/disables bloom and applies settings from the UI.
fn update_bloom_settings(
    config: Res<VisualsConfig>,
    mut camera_query: Query<(Entity, Option<&mut BloomSettings>), With<Camera>>,
    mut commands: Commands,
) {
    let (camera_entity, bloom_settings) = camera_query.single_mut();

    if config.bloom_enabled {
        match bloom_settings {
            // If bloom is enabled and the component exists, we update it.
            Some(mut settings) => {
                settings.intensity = config.bloom_intensity;
                settings.low_frequency_boost = 0.7;
                settings.low_frequency_boost_curvature = 0.95;
                settings.high_pass_frequency = 0.85;
                settings.prefilter_settings.threshold = config.bloom_threshold;
                settings.prefilter_settings.threshold_softness = 0.6;
            }
            // If bloom is enabled but the component is missing, we add it.
            None => {
                commands.entity(camera_entity).insert(BloomSettings::default());
            }
        }
    } else {
        // If bloom is disabled and the component exists, we remove it.
        if bloom_settings.is_some() {
            commands.entity(camera_entity).remove::<BloomSettings>();
        }
    }
}


fn pan_orbit_camera(
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut ev_motion: EventReader<MouseMotion>,
    mut ev_scroll: EventReader<MouseWheel>,
    input_mouse: Res<ButtonInput<MouseButton>>,
    mut query: Query<(&mut PanOrbitCamera, &mut Transform)>,
    mut contexts: EguiContexts,
) {
    let ctx = contexts.ctx_mut();
    if ctx.is_pointer_over_area() || ctx.wants_pointer_input() {
        ev_motion.clear();
        ev_scroll.clear();
        return;
    }

    let Ok(window) = primary_window.get_single() else { return };

    for (mut pan_orbit, mut transform) in query.iter_mut() {
        if input_mouse.pressed(MouseButton::Left) {
            let mut rotation = Vec2::ZERO;
            for ev in ev_motion.read() {
                rotation += ev.delta;
            }

            if rotation.length_squared() > 0.0 {
                let window_size = Vec2::new(window.width() as f32, window.height() as f32);
                let delta_x = rotation.x / window_size.x * std::f32::consts::PI * 2.0;
                let delta_y = rotation.y / window_size.y * std::f32::consts::PI;

                let yaw = Quat::from_rotation_y(-delta_x);
                let pitch = Quat::from_rotation_x(-delta_y);
                transform.rotation = yaw * transform.rotation;
                transform.rotation *= pitch;
            }
        }

        let mut scroll = 0.0;
        for ev in ev_scroll.read() {
            scroll += ev.y;
        }
        if scroll.abs() > 0.0 {
            pan_orbit.radius -= scroll * pan_orbit.radius * 0.1;
            pan_orbit.radius = f32::max(pan_orbit.radius, 5.0);
        }

        let rot_matrix = Mat3::from_quat(transform.rotation);
        transform.translation = pan_orbit.focus + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, pan_orbit.radius));
    }
}