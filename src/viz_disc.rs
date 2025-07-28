// src/viz_disc.rs

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle},
    window::PrimaryWindow,
};
// AJOUTÉ : On importe la ressource d'analyse audio et le tag de la caméra 2D
use crate::{AppState, audio::AudioAnalysis, camera::MainCamera2D, config::VisualsConfig};

pub struct VizDiscPlugin;

impl Plugin for VizDiscPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(Material2dPlugin::<DiscMaterial>::default())
            .add_systems(OnEnter(AppState::VisualizationDisc), setup_disc_scene)
            .add_systems(Update, update_disc_material.run_if(in_state(AppState::VisualizationDisc)))
            .add_systems(OnExit(AppState::VisualizationDisc), despawn_scene);
    }
}

#[derive(Component)]
struct DiscScene;

// MODIFIÉ : On ajoute le champ 'zoom'.
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
    // AJOUTÉ : Le champ pour faire passer le zoom au shader.
    #[uniform(0)]
    zoom: f32,
}

impl Material2d for DiscMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/disc_shader.wgsl".into()
    }
}

// MODIFIÉ : On initialise la valeur du zoom.
fn setup_disc_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<DiscMaterial>>,
    config: Res<VisualsConfig>,
) {
    let quad_handle = meshes.add(Rectangle::new(1.0, 1.0));

    let material_handle = materials.add(DiscMaterial {
        color: config.disc_color,
        time: 0.0,
        radius: config.disc_radius,
        line_thickness: config.disc_line_thickness,
        iterations: config.disc_iterations as f32,
        speed: config.disc_speed,
        center_radius_factor: config.disc_center_radius_factor,
        resolution: Vec2::new(800.0, 600.0),
        bass: 0.0,
        flux: 0.0,
        // On initialise le zoom à 1.0 (pas de zoom).
        zoom: 1.0,
    });

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: quad_handle.into(),
            material: material_handle,
            transform: Transform::from_scale(Vec3::splat(1_000_000.0)),
            ..default()
        },
        DiscScene,
    ));
}

// MODIFIÉ : On récupère la valeur du zoom de la caméra et on la passe au matériau.
fn update_disc_material(
    time: Res<Time>,
    config: Res<VisualsConfig>,
    audio_analysis: Res<AudioAnalysis>,
    mut materials: ResMut<Assets<DiscMaterial>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    // AJOUTÉ : Une query pour trouver la caméra 2D et lire sa projection.
    q_camera: Query<&OrthographicProjection, With<MainCamera2D>>,
) {
    let Ok(window) = q_window.get_single() else { return };
    let window_resolution = Vec2::new(window.width(), window.height());

    // On récupère le niveau de zoom actuel de la caméra.
    let zoom_level = if let Ok(projection) = q_camera.get_single() {
        projection.scale
    } else {
        1.0 // Valeur par défaut si on ne trouve pas la caméra.
    };

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

        // AJOUTÉ : On met à jour le zoom dans le matériau.
        material.zoom = zoom_level;
    }
}

fn despawn_scene(mut commands: Commands, scene_query: Query<Entity, With<DiscScene>>) {
    if let Ok(entity) = scene_query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}