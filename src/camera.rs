// src/camera.rs

use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*, window::PrimaryWindow
};
use crate::AppState;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::Visualization3D), setup_3d_scene_and_camera)
            .add_systems(Update, pan_orbit_camera.run_if(in_state(AppState::Visualization3D)));
    }
}

#[derive(Component)]
pub struct PanOrbitCamera {
    pub focus: Vec3,
    pub radius: f32,
    pub upside_down: bool,
}

impl Default for PanOrbitCamera {
    fn default() -> Self {
        PanOrbitCamera {
            focus: Vec3::ZERO,
            radius: 15.0,
            upside_down: false,
        }
    }
}

fn setup_3d_scene_and_camera(mut commands: Commands) {
    let initial_transform = Transform::from_xyz(0.0, 0.0, 15.0)
        .looking_at(Vec3::ZERO, Vec3::Y);

    commands.spawn((
        Camera3dBundle {
            transform: initial_transform,
            ..default()
        },
        PanOrbitCamera::default(),
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


fn pan_orbit_camera(
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut ev_motion: EventReader<MouseMotion>,
    mut ev_scroll: EventReader<MouseWheel>,
    input_mouse: Res<ButtonInput<MouseButton>>,
    mut query: Query<(&mut PanOrbitCamera, &mut Transform)>,
) {
    let Ok(window) = primary_window.get_single() else { return };

    for (mut pan_orbit, mut transform) in query.iter_mut() {
        // Orbit
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
                transform.rotation = yaw * transform.rotation; // Apply yaw first
                transform.rotation *= pitch; // Then apply pitch
            }
        }

        // Zoom
        let mut scroll = 0.0;
        for ev in ev_scroll.read() {
            scroll += ev.y;
        }
        if scroll.abs() > 0.0 {
            pan_orbit.radius -= scroll * pan_orbit.radius * 0.1;
            pan_orbit.radius = f32::max(pan_orbit.radius, 5.0);
        }

        // Update transform
        let rot_matrix = Mat3::from_quat(transform.rotation);
        transform.translation = pan_orbit.focus + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, pan_orbit.radius));
    }
}