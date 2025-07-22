// src/ui.rs

use bevy::prelude::*;
use crate::audio::{AudioSource, SelectedAudioSource, SelectedMic};
use crate::AppState;
use cpal::traits::{DeviceTrait, HostTrait};

/// The plugin that encapsulates all UI logic.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
            .add_systems(Update, menu_button_interaction.run_if(in_state(AppState::MainMenu)))
            .add_systems(OnExit(AppState::MainMenu), cleanup_menu)
            // Systems for the new microphone selection menu
            .add_systems(OnEnter(AppState::MicSelection), setup_mic_selection_menu)
            .add_systems(Update, mic_selection_interaction.run_if(in_state(AppState::MicSelection)))
            .add_systems(OnExit(AppState::MicSelection), cleanup_menu);
    }
}

// --- UI Components ---

#[derive(Component)]
enum MenuButtonAction {
    Start3DFile,
    Start2DFile,
    Start3DMic,
    Start2DMic,
    ToMicSelection,
}
#[derive(Component)]
struct MainMenuUI;
#[derive(Component)]
struct MicDeviceButton(String);


// --- UI Systems ---

/// Spawns the main menu buttons.
fn setup_main_menu(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), MainMenuUI));
    commands.spawn((NodeBundle {
            style: Style {
                width: Val::Percent(100.0), height: Val::Percent(100.0),
                align_items: AlignItems::Center, justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column, row_gap: Val::Px(15.0),
                ..default()
            },
            ..default()
        }, MainMenuUI)).with_children(|parent| {
        create_menu_button(parent, "Start 3D (File)", MenuButtonAction::Start3DFile);
        create_menu_button(parent, "Start 2D (File)", MenuButtonAction::Start2DFile);
        create_menu_button(parent, "Start 3D (Mic)", MenuButtonAction::Start3DMic);
        create_menu_button(parent, "Start 2D (Mic)", MenuButtonAction::Start2DMic);
        create_menu_button(parent, "Select Microphone", MenuButtonAction::ToMicSelection);
    });
}

/// Handles interactions with the main menu buttons.
fn menu_button_interaction(
    mut commands: Commands,
    mut button_query: Query<(&Interaction, &MenuButtonAction), (Changed<Interaction>, With<Button>)>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    for (interaction, action) in &mut button_query {
        if *interaction == Interaction::Pressed {
            match action {
                MenuButtonAction::Start3DFile => {
                    commands.insert_resource(SelectedAudioSource(AudioSource::File));
                    next_app_state.set(AppState::Visualization3D);
                }
                MenuButtonAction::Start2DFile => {
                    commands.insert_resource(SelectedAudioSource(AudioSource::File));
                    next_app_state.set(AppState::Visualization2D);
                }
                MenuButtonAction::Start3DMic => {
                    commands.insert_resource(SelectedAudioSource(AudioSource::Microphone));
                    next_app_state.set(AppState::Visualization3D);
                }
                MenuButtonAction::Start2DMic => {
                    commands.insert_resource(SelectedAudioSource(AudioSource::Microphone));
                    next_app_state.set(AppState::Visualization2D);
                }
                MenuButtonAction::ToMicSelection => {
                    next_app_state.set(AppState::MicSelection);
                }
            }
        }
    }
}

/// Spawns the microphone selection menu.
fn setup_mic_selection_menu(mut commands: Commands) {
    info!("Opening microphone selection menu...");
    commands.spawn((Camera2dBundle::default(), MainMenuUI));
    let mut root = commands.spawn((NodeBundle {
        style: Style {
            width: Val::Percent(100.0), height: Val::Percent(100.0),
            align_items: AlignItems::Center, justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0),
            ..default()
        },
        ..default()
    }, MainMenuUI));

    root.with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            "Select an Input Device",
            TextStyle { font_size: 32.0, color: Color::WHITE, ..default() },
        ));
    });

    // Step 1: Get the list of available input devices.
    let host = cpal::default_host();
    if let Ok(devices) = host.input_devices() {
        root.with_children(|parent| {
            // Step 2: Create a button for each device.
            for device in devices {
                if let Ok(name) = device.name() {
                    info!("Found device: {}", name);
                    parent.spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(500.0), height: Val::Px(50.0),
                                justify_content: JustifyContent::Center, align_items: AlignItems::Center,
                                margin: UiRect::top(Val::Px(5.0)),
                                ..default()
                            },
                            background_color: Color::rgb(0.2, 0.2, 0.2).into(),
                            ..default()
                        },
                        MicDeviceButton(name.clone()),
                    )).with_children(|btn| {
                        btn.spawn(TextBundle::from_section(name, TextStyle { font_size: 18.0, color: Color::WHITE, ..default() }));
                    });
                }
            }
        });
    } else {
        error!("Failed to get input devices");
    }
}

/// Handles interactions with the microphone selection buttons.
fn mic_selection_interaction(
    mut button_query: Query<(&Interaction, &MicDeviceButton)>,
    mut selected_mic: ResMut<SelectedMic>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    for (interaction, button) in &mut button_query {
        if *interaction == Interaction::Pressed {
            info!("Selected microphone: {}", &button.0);
            selected_mic.0 = Some(button.0.clone());
            next_app_state.set(AppState::MainMenu);
        }
    }
}

/// A generic helper function to create a menu button.
fn create_menu_button(parent: &mut ChildBuilder, text: &str, action: MenuButtonAction) {
    parent.spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(250.0), height: Val::Px(65.0),
                    justify_content: JustifyContent::Center, align_items: AlignItems::Center,
                    ..default()
                },
                background_color: Color::rgb(0.15, 0.15, 0.15).into(),
                ..default()
            },
            action,
        )).with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                text,
                TextStyle { font_size: 24.0, color: Color::WHITE, ..default() },
            ));
        });
}

/// A generic cleanup system for any menu UI.
fn cleanup_menu(mut commands: Commands, ui_query: Query<Entity, With<MainMenuUI>>) {
    info!("Cleaning up menu UI...");
    for entity in ui_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}