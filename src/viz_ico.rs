use crate::{AppState, config::VisualsConfig, camera::MainCamera2D}; // Ajout import camera
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle},
    window::PrimaryWindow,
};

pub struct VizIcoPlugin;

impl Plugin for VizIcoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<IcoMaterial>::default())
            .add_systems(OnEnter(AppState::VisualizationIco), setup_ico_scene)
            .add_systems(
                Update,
                update_ico_material.run_if(in_state(AppState::VisualizationIco)),
            )
            .add_systems(OnExit(AppState::VisualizationIco), despawn_scene);
    }
}

#[derive(Component)]
struct IcoScene;

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
#[repr(C)]
pub struct IcoMaterial {
    #[uniform(0)]
    pub color: Vec4, // r, g, b, a
    #[uniform(0)]
    pub resolution_mouse: Vec4, // x=width, y=height, z=mouseX, w=mouseY
    #[uniform(0)]
    pub time_params: Vec4, // x=time, y=speed, z=ZOOM (camera scale), w=unused
}

impl Material2d for IcoMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/ico_shader.wgsl".into()
    }
}

fn setup_ico_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<IcoMaterial>>,
    config: Res<VisualsConfig>,
) {
    let quad_handle = meshes.add(Rectangle::new(1.0, 1.0));

    let material_handle = materials.add(IcoMaterial {
        color: Vec4::from(config.ico_color.as_linear_rgba_f32()),
        resolution_mouse: Vec4::new(800.0, 600.0, 0.0, 0.0),
        time_params: Vec4::new(0.0, config.ico_speed, 1.0, 0.0),
    });

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: quad_handle.into(),
            material: material_handle,
            // Le quad couvre tout l'écran
            transform: Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(10_000.0)),
            ..default()
        },
        IcoScene,
    ));
}

fn update_ico_material(
    time: Res<Time>,
    config: Res<VisualsConfig>,
    mut materials: ResMut<Assets<IcoMaterial>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<&OrthographicProjection, With<MainCamera2D>>, // On récupère la caméra
) {
    let Ok(window) = q_window.get_single() else {
        return;
    };

    // CORRECTION MAJEURE : Utilisation de la résolution PHYSIQUE
    // mesh.position dans le shader correspond aux pixels physiques.
    // Si on envoie la taille logique (width/height), le shader décale tout sur les écrans HDPI.
    let width = window.resolution.physical_width() as f32;
    let height = window.resolution.physical_height() as f32;

    // On récupère la souris (optionnel pour d'autres effets, mais plus pour le zoom)
    let mouse = window.cursor_position().unwrap_or(Vec2::ZERO);

    // Récupération du zoom (scale) de la caméra contrôlé par la molette
    let zoom_level = if let Ok(projection) = q_camera.get_single() {
        projection.scale
    } else {
        1.0
    };

    for (_, material) in materials.iter_mut() {
        material.color = Vec4::from(config.ico_color.as_linear_rgba_f32());

        material.resolution_mouse = Vec4::new(
            width,
            height,
            mouse.x,
            // On envoie la coordonnée Y inversée au cas où, mais moins critique ici
            height - mouse.y,
        );

        material.time_params.x = time.elapsed_seconds();
        material.time_params.y = config.ico_speed;
        // On injecte le zoom caméra dans le paramètre Z
        material.time_params.z = zoom_level;
    }
}

fn despawn_scene(mut commands: Commands, scene_query: Query<Entity, With<IcoScene>>) {
    if let Ok(entity) = scene_query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}