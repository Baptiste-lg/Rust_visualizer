use bevy::prelude::*;
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;

// --- Enums for State and UI ---

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum AppState {
    #[default]
    MainMenu,
    Visualization3D,
    Visualization2D,
}

#[derive(Component)]
enum MenuButtonAction {
    Start3D,
    Start2D,
}

#[derive(Component)]
struct MainMenuUI;


// --- Main Application Logic ---

fn main() {
    // --- CORRECTED: Create the audio resources BEFORE building the App ---
    // 1. Get the stream and its handle.
    let (stream, stream_handle) = OutputStream::try_default().unwrap();

    // 2. Use the handle to create the sink.
    let sink = Sink::try_new(&stream_handle).unwrap();

    App::new()
        .add_plugins(DefaultPlugins)

        // 3. Move the stream and sink into the app as NonSend resources.
        // The stream object must be kept alive, so we insert it here.
        // There is no need to clone it.
        .insert_non_send_resource(stream)
        .insert_non_send_resource(sink)

        .init_state::<AppState>()

        // Systems for MainMenu state
        .add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
        .add_systems(Update, menu_button_interaction.run_if(in_state(AppState::MainMenu)))
        .add_systems(OnExit(AppState::MainMenu), cleanup_menu)

        // Systems for visualization states
        .add_systems(OnEnter(AppState::Visualization3D), (setup_3d_scene, play_audio_file))
        .add_systems(OnEnter(AppState::Visualization2D), (setup_2d_scene, play_audio_file))

        .run();
}


// --- Audio System ---

/// Accesses the audio Sink and plays a hardcoded file.
fn play_audio_file(
    // CORRECTED: Get non-mutable access to the Sink. This resolves the warning.
    sink: NonSend<Sink>
) {
    // Load a sound from a file.
    let file = BufReader::new(File::open("assets/ShortClip.mp3").expect("Failed to open music file"));
    let source = Decoder::new(file).unwrap();

    // Use the sink to play the sound.
    sink.clear();
    sink.append(source);
    sink.play();

    info!("Audio playback started.");
}


// --- UI and Scene Systems ---

/// System that runs once when entering the MainMenu state to build the UI
fn setup_main_menu(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), MainMenuUI));
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(20.0),
                    ..default()
                },
                ..default()
            },
            MainMenuUI,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(250.0),
                            height: Val::Px(65.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::rgb(0.15, 0.15, 0.15).into(),
                        ..default()
                    },
                    MenuButtonAction::Start3D,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Start 3D Visualization",
                        TextStyle { font_size: 24.0, color: Color::WHITE, ..default() },
                    ));
                });
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(250.0),
                            height: Val::Px(65.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::rgb(0.15, 0.15, 0.15).into(),
                        ..default()
                    },
                    MenuButtonAction::Start2D,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Start 2D Visualization",
                        TextStyle { font_size: 24.0, color: Color::WHITE, ..default() },
                    ));
                });
        });
}

/// System to handle button clicks and change the application state
fn menu_button_interaction(
    mut button_query: Query<(&Interaction, &MenuButtonAction), (Changed<Interaction>, With<Button>)>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    for (interaction, action) in &mut button_query {
        if *interaction == Interaction::Pressed {
            match action {
                MenuButtonAction::Start3D => {
                    next_app_state.set(AppState::Visualization3D);
                }
                MenuButtonAction::Start2D => {
                    next_app_state.set(AppState::Visualization2D);
                }
            }
        }
    }
}

/// System that runs once when exiting the MainMenu state to despawn UI
fn cleanup_menu(mut commands: Commands, ui_query: Query<Entity, With<MainMenuUI>>) {
    info!("Cleaning up main menu UI...");
    for entity in ui_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

/// System to set up the 3D scene
fn setup_3d_scene(mut commands: Commands) {
    info!("Setting up 3D scene...");
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
}

/// System to set up the 2D scene
fn setup_2d_scene(mut commands: Commands) {
    info!("Setting up 2D scene...");
    commands.spawn(Camera2dBundle::default());
}