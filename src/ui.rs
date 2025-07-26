// src/ui.rs

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiSet};
use bevy_egui::egui::color_picker;
use crate::audio::{AudioSource, SelectedAudioSource, SelectedMic};
use crate::config::VisualsConfig;
// MODIFIED: Added ActiveVisualization to the imports
use crate::{AppState, VisualizationEnabled, ActiveVisualization};
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
                    .run_if(in_state(AppState::Visualization2D)
                        .or_else(in_state(AppState::Visualization3D))
                        .or_else(in_state(AppState::VisualizationOrb)))
            );
    }
}

// MODIFIED: Simplified menu actions
#[derive(Component)]
enum MenuButtonAction {
    Start,
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
        // MODIFIED: Changed to a single "Start" button
        create_menu_button(parent, "Start Visualization", MenuButtonAction::Start);
        create_menu_button(parent, "Select Microphone", MenuButtonAction::ToMicSelection);
    });
}

fn menu_button_interaction(
    mut button_query: Query<(&Interaction, &MenuButtonAction), (Changed<Interaction>, With<Button>)>,
    mut next_app_state: ResMut<NextState<AppState>>,
    // ADDED: Get the active visualization resource
    active_viz: Res<ActiveVisualization>,
) {
    for (interaction, action) in &mut button_query {
        if *interaction == Interaction::Pressed {
            match action {
                MenuButtonAction::Start => {
                    // MODIFIED: Go to the last active visualization state
                    next_app_state.set(active_viz.0.clone());
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

fn visualizer_ui_system(
    mut contexts: EguiContexts,
    mut config: ResMut<VisualsConfig>,
    mut selected_source: ResMut<SelectedAudioSource>,
    mut viz_enabled: ResMut<VisualizationEnabled>,
    app_state: Res<State<AppState>>,
    // ADDED: Get the NextState to allow changing visualizers
    mut next_app_state: ResMut<NextState<AppState>>,
    // ADDED: Get the active visualization to update it
    mut active_viz: ResMut<ActiveVisualization>,
) {
    egui::Window::new("Controls").show(contexts.ctx_mut(), |ui| {
        ui.label("Number of Bands");
        ui.add(egui::Slider::new(&mut config.num_bands, 4..=32).logarithmic(true));

        ui.label("Amplitude Sensitivity");
        ui.add(egui::Slider::new(&mut config.bass_sensitivity, 0.0..=20.0));

        if *app_state.get() == AppState::Visualization2D {
            ui.separator();
            ui.label("Inactive Color");
            let mut color_temp_inactive = [config.viz2d_inactive_color.r(), config.viz2d_inactive_color.g(), config.viz2d_inactive_color.b()];
            if color_picker::color_edit_button_rgb(ui, &mut color_temp_inactive).changed() {
                config.viz2d_inactive_color = Color::rgb(color_temp_inactive[0], color_temp_inactive[1], color_temp_inactive[2]);
            }

            ui.label("Active Color");
            let mut color_temp_active = [config.viz2d_active_color.r(), config.viz2d_active_color.g(), config.viz2d_active_color.b()];
            if color_picker::color_edit_button_rgb(ui, &mut color_temp_active).changed() {
                config.viz2d_active_color = Color::rgb(color_temp_active[0], color_temp_active[1], color_temp_active[2]);
            }
        }

        if *app_state.get() == AppState::Visualization3D {
            ui.separator();
            ui.checkbox(&mut config.spread_enabled, "Enable Cube Spread");

            ui.label("Base Color");
            let mut color_temp_base = [config.viz3d_base_color.r(), config.viz3d_base_color.g(), config.viz3d_base_color.b()];
            if color_picker::color_edit_button_rgb(ui, &mut color_temp_base).changed() {
                config.viz3d_base_color = Color::rgb(color_temp_base[0], color_temp_base[1], color_temp_base[2]);
            }

            ui.separator();
            ui.checkbox(&mut config.bloom_enabled, "Enable Bloom");
            if config.bloom_enabled {
                ui.label("Bloom Color");
                let mut color_temp_bloom = [config.bloom_color.r(), config.bloom_color.g(), config.bloom_color.b()];
                if color_picker::color_edit_button_rgb(ui, &mut color_temp_bloom).changed() {
                    config.bloom_color = Color::rgb(color_temp_bloom[0], color_temp_bloom[1], color_temp_bloom[2]);
                }

                ui.label("Intensity");
                ui.add(egui::Slider::new(&mut config.bloom_intensity, 0.0..=1.0));

                ui.label("Threshold");
                ui.add(egui::Slider::new(&mut config.bloom_threshold, 0.0..=2.0));
            }
        }

        if *app_state.get() == AppState::VisualizationOrb {
            ui.separator();
            ui.label("Orb controls would go here.");
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

        if ui.button("Choose AudioFile").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("audio", &["mp3", "wav"])
                .pick_file()
            {
                selected_source.0 = AudioSource::File(path);
            }
        }
    });

    // ADDED: This is the new window for selecting a visualizer.
    egui::Window::new("Visualizers").show(contexts.ctx_mut(), |ui| {
        ui.label("Select a visualizer:");
        ui.separator();

        if ui.button("2D Bars").clicked() {
            next_app_state.set(AppState::Visualization2D);
            active_viz.0 = AppState::Visualization2D;
        }
        if ui.button("3D Cubes").clicked() {
            next_app_state.set(AppState::Visualization3D);
            active_viz.0 = AppState::Visualization3D;
        }
        if ui.button("3D Orb").clicked() {
            next_app_state.set(AppState::VisualizationOrb);
            active_viz.0 = AppState::VisualizationOrb;
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