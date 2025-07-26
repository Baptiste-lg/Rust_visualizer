// src/camera.rs

use bevy::{
    core_pipeline::{
        bloom::BloomSettings,
        experimental::taa::TemporalAntiAliasBundle
    },
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    render::camera::OrthographicProjection,
    window::PrimaryWindow
};
use bevy_egui::EguiContexts;
use crate::{AppState, config::VisualsConfig};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::Visualization3D), setup_3d_scene_and_camera)
            .add_systems(OnEnter(AppState::VisualizationOrb), setup_3d_scene_and_camera)
            .add_systems(Update, (
                pan_orbit_camera,
                update_bloom_settings
            ).run_if(in_state(AppState::Visualization3D).or_else(in_state(AppState::VisualizationOrb))))
            .add_systems(Update, control_2d_camera.run_if(in_state(AppState::Visualization2D)))
            // ADDED: Systems to despawn the 3D camera when leaving a 3D state
            .add_systems(OnExit(AppState::Visualization3D), despawn_3d_camera)
            .add_systems(OnExit(AppState::VisualizationOrb), despawn_3d_camera);
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

// ADDED: This component will tag the 3D camera so we can easily find and despawn it.
#[derive(Component)]
struct Camera3D;

fn setup_3d_scene_and_camera(mut commands: Commands) {
    let initial_transform = Transform::from_xyz(0.0, 0.0, 15.0)
        .looking_at(Vec3::ZERO, Vec3::Y);

    commands.spawn((
        Camera3dBundle {
            transform: initial_transform,
            camera: Camera {
                hdr: true,
                ..default()
            },
            ..default()
        },
        BloomSettings::default(),
        PanOrbitCamera::default(),
        TemporalAntiAliasBundle::default(),
        Camera3D, // ADDED: Tag the camera
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

// ADDED: This system despawns the 3D camera and light.
fn despawn_3d_camera(
    mut commands: Commands,
    camera_query: Query<Entity, With<Camera3D>>,
    light_query: Query<Entity, With<PointLight>>,
) {
    for entity in &camera_query {
        commands.entity(entity).despawn_recursive();
    }
    for entity in &light_query {
        commands.entity(entity).despawn_recursive();
    }
}

fn update_bloom_settings(
    config: Res<VisualsConfig>,
    mut camera_query: Query<(Entity, Option<&mut BloomSettings>), With<Camera3D>>,
    mut commands: Commands,
) {
    if let Ok((camera_entity, bloom_settings)) = camera_query.get_single_mut() {
        if config.bloom_enabled {
            match bloom_settings {
                Some(mut settings) => {
                    settings.intensity = config.bloom_intensity;
                    settings.low_frequency_boost = 0.7;
                    settings.low_frequency_boost_curvature = 0.95;
                    settings.high_pass_frequency = 0.85;
                    settings.prefilter_settings.threshold = config.bloom_threshold;
                    settings.prefilter_settings.threshold_softness = 0.6;
                }
                None => {
                    commands.entity(camera_entity).insert(BloomSettings::default());
                }
            }
        } else {
            if bloom_settings.is_some() {
                commands.entity(camera_entity).remove::<BloomSettings>();
            }
        }
    }
}

fn control_2d_camera(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut scroll_evr: EventReader<MouseWheel>,
    mut camera_query: Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
    mut contexts: EguiContexts,
) {
    if contexts.ctx_mut().is_pointer_over_area() || contexts.ctx_mut().wants_pointer_input() {
        scroll_evr.clear();
        return;
    }

    if let Ok((mut transform, mut projection)) = camera_query.get_single_mut() {
        let mut rotation_factor = 0.0;
        if keyboard_input.pressed(KeyCode::KeyE) {
            rotation_factor += 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            rotation_factor -= 1.0;
        }
        transform.rotate_z(rotation_factor * time.delta_seconds());

        for ev in scroll_evr.read() {
            projection.scale -= ev.y * 0.1;
        }
        projection.scale = projection.scale.max(0.1);
    }
}

fn pan_orbit_camera(
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut ev_motion: EventReader<MouseMotion>,
    mut ev_scroll: EventReader<MouseWheel>,
    input_mouse: Res<ButtonInput<MouseButton>>,
    mut query: Query<(&mut PanOrbitCamera, &mut Transform), With<Camera3D>>,
    mut contexts: EguiContexts,
) {
    let ctx = contexts.ctx_mut();
    if ctx.is_pointer_over_area() || ctx.wants_pointer_input() {
        ev_motion.clear();
        ev_scroll.clear();
        return;
    }

    let Ok(window) = primary_window.get_single() else { return };

    if let Ok((mut pan_orbit, mut transform)) = query.get_single_mut() {
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