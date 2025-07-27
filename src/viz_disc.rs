// src/viz_disc.rs

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle},
    window::PrimaryWindow, // On a besoin de ça pour obtenir la taille de la fenêtre
};
use crate::{AppState, config::VisualsConfig};

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

// On ajoute un champ pour la résolution de la fenêtre
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
    resolution: Vec2, // Le nouveau champ
}


impl Material2d for DiscMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/disc_shader.wgsl".into()
    }
}

// On initialise la scène avec une résolution de base.
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
        resolution: Vec2::new(800.0, 600.0), // Une valeur par défaut, sera mise à jour immédiatement
    });

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: quad_handle.into(),
            material: material_handle,
            // On met le carré à une échelle immense pour qu'il couvre tout
            transform: Transform::from_scale(Vec3::splat(1_000_000.0)),
            ..default()
        },
        DiscScene,
    ));
}

// C'est ici qu'on met à jour toutes les données à chaque image.
fn update_disc_material(
    time: Res<Time>,
    config: Res<VisualsConfig>,
    mut materials: ResMut<Assets<DiscMaterial>>,
    // On récupère la fenêtre principale pour lire sa taille
    q_window: Query<&Window, With<PrimaryWindow>>,
) {
    // On s'assure que la fenêtre existe
    let Ok(window) = q_window.get_single() else { return };
    let window_resolution = Vec2::new(window.width(), window.height());

    for (_, material) in materials.iter_mut() {
        material.time = time.elapsed_seconds();
        material.color = config.disc_color;
        material.radius = config.disc_radius;
        material.line_thickness = config.disc_line_thickness;
        material.iterations = config.disc_iterations as f32;
        material.speed = config.disc_speed;
        material.center_radius_factor = config.disc_center_radius_factor;
        // On met à jour la résolution dans le matériau
        material.resolution = window_resolution;
    }
}

fn despawn_scene(mut commands: Commands, scene_query: Query<Entity, With<DiscScene>>) {
    if let Ok(entity) = scene_query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}