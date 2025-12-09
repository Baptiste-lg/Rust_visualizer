// src/camera.rs

use crate::{AppState, config::VisualsConfig};
use bevy::{
    core_pipeline::bloom::BloomSettings,
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_egui::EguiContexts;

pub struct CameraPlugin;

// A marker component for the main 3D camera.
#[derive(Component)]
pub struct MainCamera3D;

// A marker component for the main 2D camera.
#[derive(Component)]
pub struct MainCamera2D;

// A component that enables pan-and-orbit camera controls.
#[derive(Component)]
pub struct PanOrbitController {
    pub focus: Vec3,
    pub radius: f32,
    pub enabled: bool,
}

impl Default for PanOrbitController {
    fn default() -> Self {
        PanOrbitController {
            focus: Vec3::ZERO,
            radius: 15.0,
            enabled: true,
        }
    }
}

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            // Systems for the 3D camera
            .add_systems(OnEnter(AppState::Visualization3D), setup_3d_camera)
            .add_systems(OnEnter(AppState::VisualizationOrb), setup_3d_camera)
            .add_systems(OnExit(AppState::Visualization3D), despawn_3d_camera)
            .add_systems(OnExit(AppState::VisualizationOrb), despawn_3d_camera)
            .add_systems(
                Update,
                (pan_orbit_camera, update_bloom_settings).run_if(
                    in_state(AppState::Visualization3D)
                        .or_else(in_state(AppState::VisualizationOrb)),
                ),
            )
            // Systems for the 2D camera
            .add_systems(OnEnter(AppState::Visualization2D), setup_2d_camera)
            .add_systems(OnEnter(AppState::VisualizationDisc), setup_2d_camera)
            .add_systems(OnExit(AppState::Visualization2D), despawn_2d_camera)
            .add_systems(OnExit(AppState::VisualizationDisc), despawn_2d_camera)
            .add_systems(
                Update,
                control_2d_camera.run_if(
                    in_state(AppState::Visualization2D)
                        .or_else(in_state(AppState::VisualizationDisc)),
                ),
            );
    }
}

// Spawns the 3D camera and a point light when entering a 3D visualization state.
fn setup_3d_camera(mut commands: Commands) {
    let initial_transform = Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y);

    commands.spawn((
        Camera3dBundle {
            transform: initial_transform,
            camera: Camera {
                hdr: true, // Enable High Dynamic Range for bloom effects.
                ..default()
            },
            ..default()
        },
        BloomSettings::default(),
        PanOrbitController::default(),
        MainCamera3D,
    ));

    // Add a default light to the scene.
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

// Despawns the 3D camera and light when exiting a 3D visualization state.
fn despawn_3d_camera(
    mut commands: Commands,
    camera_query: Query<Entity, With<MainCamera3D>>,
    light_query: Query<Entity, With<PointLight>>,
) {
    if let Ok(entity) = camera_query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
    if let Ok(entity) = light_query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}

// Spawns the 2D camera when entering a 2D visualization state.
fn setup_2d_camera(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), MainCamera2D));
}

// Despawns the 2D camera when exiting a 2D visualization state.
fn despawn_2d_camera(mut commands: Commands, camera_query: Query<Entity, With<MainCamera2D>>) {
    if let Ok(entity) = camera_query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}

// Updates the camera's bloom settings based on the configuration.
// This will add or remove the `BloomSettings` component as needed.
fn update_bloom_settings(
    config: Res<VisualsConfig>,
    mut camera_query: Query<(Entity, Option<&mut BloomSettings>), With<MainCamera3D>>,
    mut commands: Commands,
) {
    if let Ok((camera_entity, bloom_settings)) = camera_query.get_single_mut() {
        if config.bloom_enabled {
            match bloom_settings {
                Some(mut settings) => {
                    // Update existing bloom settings.
                    settings.intensity = config.bloom_intensity;
                    settings.prefilter_settings.threshold = config.bloom_threshold;
                }
                None => {
                    // Add bloom settings if they don't exist.
                    commands.entity(camera_entity).insert(BloomSettings {
                        intensity: config.bloom_intensity,
                        prefilter_settings: bevy::core_pipeline::bloom::BloomPrefilterSettings {
                            threshold: config.bloom_threshold,
                            ..default()
                        },
                        ..default()
                    });
                }
            }
        } else {
            // Remove bloom settings if disabled.
            if bloom_settings.is_some() {
                commands.entity(camera_entity).remove::<BloomSettings>();
            }
        }
    }
}

// Controls the zoom level of the 2D camera using the mouse wheel.
fn control_2d_camera(
    mut ev_scroll: EventReader<MouseWheel>,
    mut camera_query: Query<&mut OrthographicProjection, With<MainCamera2D>>,
    mut contexts: EguiContexts,
) {
    // Ignore scroll events if the mouse is over a UI element.
    if contexts.ctx_mut().is_pointer_over_area() || contexts.ctx_mut().wants_pointer_input() {
        ev_scroll.clear();
        return;
    }

    if let Ok(mut projection) = camera_query.get_single_mut() {
        for ev in ev_scroll.read() {
            projection.scale -= ev.y * 0.1;
        }
        // Prevent zooming in too far.
        projection.scale = projection.scale.max(0.1);
    }
}

// Implements the pan-and-orbit controls for the 3D camera.
fn pan_orbit_camera(
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut ev_motion: EventReader<MouseMotion>,
    mut ev_scroll: EventReader<MouseWheel>,
    input_mouse: Res<ButtonInput<MouseButton>>,
    mut query: Query<(&mut PanOrbitController, &mut Transform), With<MainCamera3D>>,
    mut contexts: EguiContexts,
) {
    let ctx = contexts.ctx_mut();
    // Ignore mouse events if the mouse is over a UI element.
    if ctx.is_pointer_over_area() || ctx.wants_pointer_input() {
        ev_motion.clear();
        ev_scroll.clear();
        return;
    }

    let Ok(window) = primary_window.get_single() else {
        return;
    };

    if let Ok((mut pan_orbit, mut transform)) = query.get_single_mut() {
        if !pan_orbit.enabled {
            return;
        }

        // Handle rotation with the left mouse button.
        if input_mouse.pressed(MouseButton::Left) {
            let mut rotation = Vec2::ZERO;
            for ev in ev_motion.read() {
                rotation += ev.delta;
            }

            if rotation.length_squared() > 0.0 {
                let window_size = Vec2::new(window.width(), window.height());
                let delta_x = rotation.x / window_size.x * std::f32::consts::PI * 2.0;
                let delta_y = rotation.y / window_size.y * std::f32::consts::PI;
                let yaw = Quat::from_rotation_y(-delta_x);
                let pitch = Quat::from_rotation_x(-delta_y);
                transform.rotation = yaw * transform.rotation * pitch;
            }
        }

        // Handle zoom with the mouse wheel.
        let mut scroll = 0.0;
        for ev in ev_scroll.read() {
            scroll += ev.y;
        }
        if scroll.abs() > 0.0 {
            pan_orbit.radius = (pan_orbit.radius - scroll * pan_orbit.radius * 0.1).max(5.0);
        }

        // Update the camera's position based on the new rotation and radius.
        let rot_matrix = Mat3::from_quat(transform.rotation);
        transform.translation =
            pan_orbit.focus + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, pan_orbit.radius));
    }
}
