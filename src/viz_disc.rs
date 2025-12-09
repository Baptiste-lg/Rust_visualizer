// src/viz_disc.rs

use crate::{AppState, audio::AudioAnalysis, camera::MainCamera2D, config::VisualsConfig};
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle},
    window::PrimaryWindow,
};

pub struct VizDiscPlugin;

impl Plugin for VizDiscPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<DiscMaterial>::default())
            .add_systems(OnEnter(AppState::VisualizationDisc), setup_disc_scene)
            .add_systems(
                Update,
                update_disc_material.run_if(in_state(AppState::VisualizationDisc)),
            )
            .add_systems(OnExit(AppState::VisualizationDisc), despawn_scene);
    }
}

// A marker component for the disc visualization scene.
#[derive(Component)]
struct DiscScene;

// The custom material for the 2D disc visualizer.
// The `#[uniform(0)]` attribute makes these fields available to the shader.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct DiscMaterial {
    #[uniform(0)]
    color: Color,
    #[uniform(0)]
    time: f32,
    #[uniform(0)]
    radius: f32,
    #[uniform(0)]
    line_thickness: f32,
    #[uniform(0)]
    iterations: f32,
    #[uniform(0)]
    speed: f32,
    #[uniform(0)]
    center_radius_factor: f32,
    #[uniform(0)]
    resolution: Vec2,
    #[uniform(0)]
    bass: f32,
    #[uniform(0)]
    flux: f32,
    #[uniform(0)]
    zoom: f32, // Passes the camera zoom level to the shader.
}

impl Material2d for DiscMaterial {
    // Specifies the fragment shader that will be used to render this material.
    fn fragment_shader() -> ShaderRef {
        "shaders/disc_shader.wgsl".into()
    }
}

// Sets up the scene for the disc visualizer. This involves creating a large quad
// that covers the entire screen and applying our custom `DiscMaterial` to it.
fn setup_disc_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<DiscMaterial>>,
    config: Res<VisualsConfig>,
) {
    let quad_handle = meshes.add(Rectangle::new(1.0, 1.0));

    // Create an instance of our custom material with default values.
    let material_handle = materials.add(DiscMaterial {
        color: config.disc_color,
        time: 0.0,
        radius: config.disc_radius,
        line_thickness: config.disc_line_thickness,
        iterations: config.disc_iterations as f32,
        speed: config.disc_speed,
        center_radius_factor: config.disc_center_radius_factor,
        resolution: Vec2::new(800.0, 600.0), // Initial resolution, will be updated.
        bass: 0.0,
        flux: 0.0,
        zoom: 1.0, // Initial zoom is 1.0 (no zoom).
    });

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: quad_handle.into(),
            material: material_handle,
            // Scale the quad to be enormous, ensuring it covers the screen.
            transform: Transform::from_scale(Vec3::splat(1_000_000.0)),
            ..default()
        },
        DiscScene,
    ));
}

// Updates the properties of the `DiscMaterial` every frame.
// This passes real-time data like audio analysis and time to the shader.
fn update_disc_material(
    time: Res<Time>,
    config: Res<VisualsConfig>,
    audio_analysis: Res<AudioAnalysis>,
    mut materials: ResMut<Assets<DiscMaterial>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    // Query for the 2D camera to get its projection scale (zoom level).
    q_camera: Query<&OrthographicProjection, With<MainCamera2D>>,
) {
    let Ok(window) = q_window.get_single() else {
        return;
    };
    let window_resolution = Vec2::new(window.width(), window.height());

    // Get the current zoom level from the camera's orthographic projection.
    let zoom_level = if let Ok(projection) = q_camera.get_single() {
        projection.scale
    } else {
        1.0 // Default to 1.0 if the camera isn't found.
    };

    // Iterate over all instances of our material and update their properties.
    for (_, material) in materials.iter_mut() {
        material.time = time.elapsed_seconds();
        material.color = config.disc_color;
        material.radius = config.disc_radius;
        material.line_thickness = config.disc_line_thickness;
        material.iterations = config.disc_iterations as f32;
        material.speed = config.disc_speed;
        material.center_radius_factor = config.disc_center_radius_factor;
        material.resolution = window_resolution;
        material.bass = audio_analysis.bass;
        material.flux = audio_analysis.flux;
        // Pass the updated zoom level to the material.
        material.zoom = zoom_level;
    }
}

// Despawns the disc scene when exiting the `VisualizationDisc` state.
fn despawn_scene(mut commands: Commands, scene_query: Query<Entity, With<DiscScene>>) {
    if let Ok(entity) = scene_query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}
