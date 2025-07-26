// src/camera.rs

use bevy::{
    // MODIFIED: Corrected the import path for post-processing components
    core_pipeline::{
        bloom::BloomSettings,
        tonemapping::Tonemapping,
        experimental::taa::TemporalAntiAliasBundle // Correct path for DebandDither/TAA
    },
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    window::PrimaryWindow
};
// MODIFIED: Removed the unused 'egui' import and added EguiContexts
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

// MODIFIED: The camera setup now includes the correct components for post-processing.
fn setup_3d_scene_and_camera(mut commands: Commands) {
    let initial_transform = Transform::from_xyz(0.0, 0.0, 15.0)
        .looking_at(Vec3::ZERO, Vec3::Y);

    commands.spawn((
        Camera3dBundle {
            transform: initial_transform,
            ..default()
        },
        PanOrbitCamera::default(),
        // --- Post-processing stack ---
        BloomSettings::default(),
        Tonemapping::TonyMcMapface,
        // Bevy 0.13 bundles DebandDither with Temporal Anti-Aliasing (TAA)
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

fn update_bloom_settings(
    config: Res<VisualsConfig>,
    mut camera_query: Query<&mut BloomSettings, With<Camera>>,
) {
    if config.is_changed() {
        for mut bloom in camera_query.iter_mut() {
            if config.bloom_enabled {
                bloom.intensity = config.bloom_intensity;
                bloom.prefilter_settings.threshold = config.bloom_threshold;
            } else {
                bloom.intensity = 0.0;
            }
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