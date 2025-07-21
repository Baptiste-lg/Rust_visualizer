// English comments as requested

use bevy::prelude::*;
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;

// --- Enums for State and UI ---

/// Defines the different states of our application.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum AppState {
    #[default] // The default state is the MainMenu
    MainMenu,
    Visualization3D,
    Visualization2D,
}

/// Enum to identify the action of each menu button.
#[derive(Component)]
enum MenuButtonAction {
    Start3D,
    Start2D,
}

/// Marker component to tag entities belonging to the main menu screen
#[derive(Component)]
struct MainMenuUI;


// --- Main Application Logic ---

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Add the State machine
        .init_state::<AppState>()

        // Systems that run in the MainMenu state
        .add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
        .add_systems(Update, menu_button_interaction.run_if(in_state(AppState::MainMenu)))
        .add_systems(OnExit(AppState::MainMenu), cleanup_menu)

        // Systems that run when entering the visualization states
        // We add the audio setup here, so it runs after a mode is selected.
        .add_systems(OnEnter(AppState::Visualization3D), (setup_3d_scene, setup_audio_playback))
        .add_systems(OnEnter(AppState::Visualization2D), (setup_2d_scene, setup_audio_playback))

        .run();
}

// --- Audio System ---

/// Loads and plays a hardcoded audio file using rodio.
/// This system runs when entering a visualization state.
fn setup_audio_playback(mut commands: Commands) {
    // Get an output stream handle to the default physical sound device.
    // This must be done on the main thread.
    let (stream, stream_handle) = OutputStream::try_default().unwrap();

    // Create a Sink to play the sound. The sink is what actually plays the audio.
    let sink = Sink::try_new(&stream_handle).unwrap();

    // Load a sound from a file.
    // Make sure you have an `assets` folder with `music.mp3` in it.
    let file = BufReader::new(File::open("assets/music.mp3").expect("Failed to open music file"));
    let source = Decoder::new(file).unwrap();
    sink.append(source);
    sink.play();

    // The sink is detached, meaning it will play until it's done.
    sink.detach();

    // IMPORTANT: Insert the stream and sink as NonSend resources.
    // This tells Bevy to keep them on the main thread, avoiding the error.
    // We store them so they are not dropped, which would stop the audio.
    commands.insert_non_send_resource(stream);
    commands.insert_non_send_resource(sink);

    info!("Audio playback started.");
}


// --- UI and Scene Systems (Unchanged) ---

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