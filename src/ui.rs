// src/ui.rs

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiSet};
use crate::audio::{AudioSource, SelectedAudioSource, SelectedMic};
use crate::config::VisualsConfig;
use crate::{AppState, VisualizationEnabled};
use cpal::traits::{DeviceTrait, HostTrait};
use bevy::window::PrimaryWindow;


pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
            .add_systems(Update, menu_button_interaction.run_if(in_state(AppState::MainMenu)))
            .add_systems(OnExit(AppState::MainMenu), cleanup_menu)
            .add_systems(OnEnter(AppState::MicSelection), setup_mic_selection_menu)
            .add_systems(Update, mic_selection_interaction.run_if(in_state(AppState::MicSelection)))
            .add_systems(OnExit(AppState::MicSelection), cleanup_menu)
            .add_systems(
                Update,
                visualizer_ui_system
                    .after(EguiSet::InitContexts)
                    .run_if(|q: Query<Entity, With<PrimaryWindow>>| !q.is_empty())
                    .run_if(in_state(AppState::Visualization2D).or_else(in_state(AppState::Visualization3D)))
            );
    }
}

#[derive(Component)]
enum MenuButtonAction {
    Start3D,
    Start2D,
    ToMicSelection,
}
#[derive(Component)]
struct MainMenuUI;
#[derive(Component)]
struct MicDeviceButton(String);


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
        create_menu_button(parent, "Start 3D", MenuButtonAction::Start3D);
        create_menu_button(parent, "Start 2D", MenuButtonAction::Start2D);
        create_menu_button(parent, "Select Microphone", MenuButtonAction::ToMicSelection);
    });
}

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
                MenuButtonAction::ToMicSelection => {
                    next_app_state.set(AppState::MicSelection);
                }
            }
        }
    }
}

fn setup_mic_selection_menu(mut commands: Commands) {
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

    let host = cpal::default_host();
    if let Ok(devices) = host.input_devices() {
        root.with_children(|parent| {
            for device in devices {
                if let Ok(name) = device.name() {
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

fn mic_selection_interaction(
    mut button_query: Query<(&Interaction, &MicDeviceButton)>,
    mut selected_mic: ResMut<SelectedMic>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    for (interaction, button) in &mut button_query {
        if *interaction == Interaction::Pressed {
            selected_mic.0 = Some(button.0.clone());
            next_app_state.set(AppState::MainMenu);
        }
    }
}

// MODIFIED: Added a checkbox and conditional UI for bloom controls.
fn visualizer_ui_system(
    mut contexts: EguiContexts,
    mut config: ResMut<VisualsConfig>,
    mut selected_source: ResMut<SelectedAudioSource>,
    mut viz_enabled: ResMut<VisualizationEnabled>,
) {
    egui::Window::new("Controls").show(contexts.ctx_mut(), |ui| {
        ui.label("Bass Sensitivity");
        ui.add(egui::Slider::new(&mut config.bass_sensitivity, 0.0..=20.0));

        ui.separator();

        // Checkbox to enable/disable bloom
        ui.checkbox(&mut config.bloom_enabled, "Enable Bloom");

        // Only show bloom sliders if bloom is enabled
        if config.bloom_enabled {
            ui.label("Intensity");
            ui.add(egui::Slider::new(&mut config.bloom_intensity, 0.0..=1.0));
            ui.label("Threshold");
            ui.add(egui::Slider::new(&mut config.bloom_threshold, 0.0..=2.0));
        }

        ui.separator();
        let button_text = if viz_enabled.0 { "Deactivate Visualizer" } else { "Activate Visualizer" };
        if ui.button(button_text).clicked() {
            viz_enabled.0 = !viz_enabled.0;
        }
    });

    egui::Window::new("Audio Source").show(contexts.ctx_mut(), |ui| {
        ui.label(format!("Current Source: {:?}", selected_source.0));
        ui.separator();

        if ui.button("Use Microphone").clicked() {
            selected_source.0 = AudioSource::Microphone;
        }

        if ui.button("Choose Audio File").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("audio", &["mp3", "wav"])
                .pick_file()
            {
                selected_source.0 = AudioSource::File(path);
            }
        }
    });
}


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

fn cleanup_menu(mut commands: Commands, ui_query: Query<Entity, With<MainMenuUI>>) {
    for entity in ui_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}