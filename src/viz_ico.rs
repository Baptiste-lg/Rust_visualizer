use crate::{AppState, config::VisualsConfig};
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

// --- STRUCTURE BLINDÉE ---
// On utilise uniquement des Vec4.
// 1 Vec4 = 16 bytes. C'est l'alignement natif du GPU.
// Plus aucun risque d'erreur de padding.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
#[repr(C)]
pub struct IcoMaterial {
    #[uniform(0)]
    pub color: Vec4, // r, g, b, a
    #[uniform(0)]
    pub resolution_mouse: Vec4, // x=width, y=height, z=mouseX, w=mouseY
    #[uniform(0)]
    pub time_params: Vec4, // x=time, y=speed, z=unused, w=unused
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
        // On initialise avec une résolution par défaut pour éviter la division par 0 au start
        resolution_mouse: Vec4::new(800.0, 600.0, 0.0, 0.0),
        time_params: Vec4::new(0.0, config.ico_speed, 0.0, 0.0),
    });

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: quad_handle.into(),
            material: material_handle,
            // On s'assure que le quad est bien visible (Z=0.0) et couvre tout
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
) {
    let Ok(window) = q_window.get_single() else {
        return;
    };

    let width = window.width();
    let height = window.height();
    let mouse = window.cursor_position().unwrap_or(Vec2::ZERO);

    for (_, material) in materials.iter_mut() {
        material.color = Vec4::from(config.ico_color.as_linear_rgba_f32());

        // On compacte les données dans les Vec4
        material.resolution_mouse = Vec4::new(
            width,
            height,
            mouse.x,
            height - mouse.y, // Inversion Y pour Shadertoy
        );

        material.time_params.x = time.elapsed_seconds();
        material.time_params.y = config.ico_speed;
    }
}

fn despawn_scene(mut commands: Commands, scene_query: Query<Entity, With<IcoScene>>) {
    if let Ok(entity) = scene_query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}
