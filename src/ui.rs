// src/ui.rs

use crate::audio::{
    AudioAnalysis, AudioSource, PlaybackInfo, PlaybackStatus, SelectedAudioSource, SelectedMic,
};
use crate::config::VisualsConfig;
use crate::{ActiveVisualization, AppState, VisualizationEnabled};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::egui::color_picker;
use bevy_egui::{EguiContexts, EguiSet, egui};
use cpal::traits::{DeviceTrait, HostTrait};
use std::time::Duration;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
            .add_systems(
                Update,
                menu_button_interaction.run_if(in_state(AppState::MainMenu)),
            )
            .add_systems(OnExit(AppState::MainMenu), cleanup_menu)
            .add_systems(OnEnter(AppState::MicSelection), setup_mic_selection_menu)
            .add_systems(
                Update,
                mic_selection_interaction.run_if(in_state(AppState::MicSelection)),
            )
            .add_systems(OnExit(AppState::MicSelection), cleanup_menu)
            .add_systems(
                Update,
                (
                    visualizer_ui_system,
                    audio_details_panel_system,
                    playback_controls_system,
                )
                    .after(EguiSet::InitContexts)
                    .run_if(
                        in_state(AppState::Visualization2D)
                            .or_else(in_state(AppState::Visualization3D))
                            .or_else(in_state(AppState::VisualizationOrb))
                            .or_else(in_state(AppState::VisualizationDisc))
                            // AJOUT : On active l'UI pour le mode Ico
                            .or_else(in_state(AppState::VisualizationIco)),
                    ),
            );
    }
}

// A marker component for buttons in the main menu.
#[derive(Component)]
enum MenuButtonAction {
    Start,
    ToMicSelection,
}

// A marker component for all UI elements in the main menu.
#[derive(Component)]
struct MainMenuUI;
#[derive(Component)]
struct MicDeviceButton(String);

// Sets up the main menu UI when entering the `MainMenu` state.
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
                    row_gap: Val::Px(15.0),
                    ..default()
                },
                ..default()
            },
            MainMenuUI,
        ))
        .with_children(|parent| {
            create_menu_button(parent, "Start Visualization", MenuButtonAction::Start);
            create_menu_button(
                parent,
                "Select Microphone",
                MenuButtonAction::ToMicSelection,
            );
        });
}

// Handles interactions with the main menu buttons.
#[allow(clippy::type_complexity)]
fn menu_button_interaction(
    mut button_query: Query<
        (&Interaction, &MenuButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_app_state: ResMut<NextState<AppState>>,
    active_viz: Res<ActiveVisualization>,
) {
    for (interaction, action) in &mut button_query {
        if *interaction == Interaction::Pressed {
            match action {
                MenuButtonAction::Start => {
                    // Transition to the last active visualization.
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
    let mut root = commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
            ..default()
        },
        MainMenuUI,
    ));

    root.with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            "Select an Input Device",
            TextStyle {
                font_size: 32.0,
                color: Color::WHITE,
                ..default()
            },
        ));
    });

    let host = cpal::default_host();
    if let Ok(devices) = host.input_devices() {
        root.with_children(|parent| {
            for device in devices {
                if let Ok(name) = device.name() {
                    parent
                        .spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(500.0),
                                    height: Val::Px(50.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    margin: UiRect::top(Val::Px(5.0)),
                                    ..default()
                                },
                                background_color: Color::rgb(0.2, 0.2, 0.2).into(),
                                ..default()
                            },
                            MicDeviceButton(name.clone()),
                        ))
                        .with_children(|btn| {
                            btn.spawn(TextBundle::from_section(
                                name,
                                TextStyle {
                                    font_size: 18.0,
                                    color: Color::WHITE,
                                    ..default()
                                },
                            ));
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

// This system draws the main control panel for the visualizer using egui.
#[allow(clippy::too_many_arguments)]
fn visualizer_ui_system(
    mut contexts: EguiContexts,
    mut config: ResMut<VisualsConfig>,
    mut selected_source: ResMut<SelectedAudioSource>,
    mut viz_enabled: ResMut<VisualizationEnabled>,
    app_state: Res<State<AppState>>,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut active_viz: ResMut<ActiveVisualization>,
    q_windows: Query<Entity, With<PrimaryWindow>>,
) {
    if q_windows.get_single().is_err() {
        return;
    }

    let current_state = app_state.get();
    let ctx = contexts.ctx_mut();

    egui::Window::new("Controls").show(ctx, |ui| {
        ui.heading("General");
        ui.checkbox(&mut config.details_panel_enabled, "Show Details Panel");
        ui.separator();

        ui.label("Number of Bands");
        ui.add(egui::Slider::new(&mut config.num_bands, 4..=32).logarithmic(true));

        ui.label("Amplitude Sensitivity");
        ui.add(egui::Slider::new(&mut config.bass_sensitivity, 0.0..=8.0));

        // --- Specific Controls for each visualization ---

        if *current_state == AppState::Visualization2D {
            ui.separator();
            ui.heading("2D Bar Controls");
        }

        if *current_state == AppState::VisualizationDisc {
            ui.separator();
            ui.heading("Disc Controls");

            ui.label("Disc Color");
            let mut color_temp_disc = [
                config.disc_color.r(),
                config.disc_color.g(),
                config.disc_color.b(),
            ];
            if color_picker::color_edit_button_rgb(ui, &mut color_temp_disc).changed() {
                config.disc_color =
                    Color::rgb(color_temp_disc[0], color_temp_disc[1], color_temp_disc[2]);
            }

            ui.label("Radius");
            ui.add(egui::Slider::new(&mut config.disc_radius, 0.1..=2.0));

            ui.label("Line Thickness");
            ui.add(egui::Slider::new(
                &mut config.disc_line_thickness,
                0.01..=0.5,
            ));

            ui.label("Iterations");
            ui.add(egui::Slider::new(&mut config.disc_iterations, 1..=50));

            ui.label("Speed");
            ui.add(egui::Slider::new(&mut config.disc_speed, -5.0..=5.0));

            ui.label("Center Radius Factor");
            ui.add(egui::Slider::new(
                &mut config.disc_center_radius_factor,
                -1.0..=2.0,
            ));
        }

        if *current_state == AppState::Visualization3D {
            ui.separator();
            ui.heading("3D Cube Controls");
            ui.checkbox(&mut config.spread_enabled, "Enable Spread Effect");
            ui.label("Column Size");
            ui.add(egui::Slider::new(&mut config.viz3d_column_size, 1..=16));
            ui.label("Cube Base Color");
            let mut color_temp = [
                config.viz3d_base_color.r(),
                config.viz3d_base_color.g(),
                config.viz3d_base_color.b(),
            ];
            if color_picker::color_edit_button_rgb(ui, &mut color_temp).changed() {
                config.viz3d_base_color = Color::rgb(color_temp[0], color_temp[1], color_temp[2]);
            }
        }

        if *current_state == AppState::VisualizationOrb {
            ui.separator();
            ui.heading("3D Orb Controls");
            ui.label("Base Color");
            let mut color_temp_base = [
                config.orb_base_color.r(),
                config.orb_base_color.g(),
                config.orb_base_color.b(),
            ];
            if color_picker::color_edit_button_rgb(ui, &mut color_temp_base).changed() {
                config.orb_base_color =
                    Color::rgb(color_temp_base[0], color_temp_base[1], color_temp_base[2]);
            }
            ui.label("Peak Color");
            let mut color_temp_peak = [
                config.orb_peak_color.r(),
                config.orb_peak_color.g(),
                config.orb_peak_color.b(),
            ];
            if color_picker::color_edit_button_rgb(ui, &mut color_temp_peak).changed() {
                config.orb_peak_color =
                    Color::rgb(color_temp_peak[0], color_temp_peak[1], color_temp_peak[2]);
            }
            ui.separator();
            ui.label("Noise Speed");
            ui.add(egui::Slider::new(&mut config.orb_noise_speed, 0.1..=5.0));
            ui.label("Noise Frequency");
            ui.add(egui::Slider::new(
                &mut config.orb_noise_frequency,
                0.5..=10.0,
            ));
            ui.label("Treble Influence");
            ui.add(egui::Slider::new(
                &mut config.orb_treble_influence,
                0.0..=1.0,
            ));
        }

        // AJOUT : Contrôles spécifiques pour le visualiseur Ico
        if *current_state == AppState::VisualizationIco {
            ui.separator();
            ui.heading("Icosahedron Controls");

            ui.label("Color Tint");
            let mut color_temp_ico = [
                config.ico_color.r(),
                config.ico_color.g(),
                config.ico_color.b(),
            ];
            if color_picker::color_edit_button_rgb(ui, &mut color_temp_ico).changed() {
                config.ico_color =
                    Color::rgb(color_temp_ico[0], color_temp_ico[1], color_temp_ico[2]);
            }

            ui.label("Rotation Speed");
            ui.add(egui::Slider::new(&mut config.ico_speed, -3.0..=3.0));
        }

        ui.separator();
        let button_text = if viz_enabled.0 {
            "Deactivate Visualizer"
        } else {
            "Activate Visualizer"
        };
        if ui.button(button_text).clicked() {
            viz_enabled.0 = !viz_enabled.0;
        }
    });

    // --- Bloom Settings Window ---
    if *current_state == AppState::Visualization3D || *current_state == AppState::VisualizationOrb {
        egui::Window::new("Bloom Settings").show(ctx, |ui| {
            ui.checkbox(&mut config.bloom_enabled, "Enable Bloom");
            if config.bloom_enabled {
                ui.label("Intensity");
                ui.add(egui::Slider::new(&mut config.bloom_intensity, 0.0..=1.0));
                ui.label("Threshold");
                ui.add(egui::Slider::new(&mut config.bloom_threshold, 0.0..=2.0));
                ui.label("Bloom Color");
                let mut color_temp_bloom = [
                    config.bloom_color.r(),
                    config.bloom_color.g(),
                    config.bloom_color.b(),
                ];
                if color_picker::color_edit_button_rgb(ui, &mut color_temp_bloom).changed() {
                    config.bloom_color = Color::rgb(
                        color_temp_bloom[0],
                        color_temp_bloom[1],
                        color_temp_bloom[2],
                    );
                }
            }
        });
    }

    // --- Audio Source Selection Window ---
    egui::Window::new("Audio Source").show(ctx, |ui| {
        ui.label(format!("Current Source: {:?}", selected_source.0));
        ui.separator();

        if ui.button("Use Microphone").clicked() {
            selected_source.0 = AudioSource::Microphone;
        }

        // --- CORRECTION: Allow collapsible if ---
        #[allow(clippy::collapsible_if)]
        if ui.button("Choose Audio File").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("audio", &["mp3", "wav"])
                .pick_file()
            {
                selected_source.0 = AudioSource::File(path);
            }
        }
    });

    // --- Visualizer Selection Window ---
    egui::Window::new("Visualizers").show(ctx, |ui| {
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
        if ui.button("2D Disc").clicked() {
            next_app_state.set(AppState::VisualizationDisc);
            active_viz.0 = AppState::VisualizationDisc;
        }
        // AJOUT : Bouton pour sélectionner le visualiseur Ico
        if ui.button("Ico Shader").clicked() {
            next_app_state.set(AppState::VisualizationIco);
            active_viz.0 = AppState::VisualizationIco;
        }
    });
}

// This system creates the UI window for controlling audio playback (play, pause, seek).
fn playback_controls_system(
    mut contexts: EguiContexts,
    mut playback_info: ResMut<PlaybackInfo>,
    selected_source: Res<SelectedAudioSource>,
) {
    // Only show the playback controls if a file is selected.
    if !matches!(selected_source.0, AudioSource::File(_)) {
        return;
    }

    egui::Window::new("Playback Controls").show(contexts.ctx_mut(), |ui| {
        // Disable controls if no track is loaded (duration is zero).
        ui.set_enabled(playback_info.duration > Duration::ZERO);

        // --- Play/Pause Button ---
        let button_text = if playback_info.status == PlaybackStatus::Playing {
            "⏸ Pause"
        } else {
            "▶ Play"
        };
        if ui.button(button_text).clicked() {
            playback_info.status = if playback_info.status == PlaybackStatus::Playing {
                PlaybackStatus::Paused
            } else {
                PlaybackStatus::Playing
            };
        }

        ui.separator();

        // --- Progress Bar / Seeker ---
        let total_secs = playback_info.duration.as_secs_f32();
        let mut current_pos_secs = playback_info.position.as_secs_f32();

        // Helper to format time for display (MM:SS).
        let format_time = |secs: f32| -> String {
            let total_seconds = secs.floor() as u64;
            let minutes = total_seconds / 60;
            let seconds = total_seconds % 60;
            format!("{:02}:{:02}", minutes, seconds)
        };

        let progress_label = format!(
            "{} / {}",
            format_time(current_pos_secs),
            format_time(total_secs)
        );

        ui.label("Progress");
        let progress_slider = egui::Slider::new(&mut current_pos_secs, 0.0..=total_secs)
            .show_value(false)
            .text(progress_label);

        // If the user interacts with the slider, send a seek request.
        if ui.add(progress_slider).changed() {
            playback_info.seek_to = Some(current_pos_secs);
        }

        ui.separator();

        // --- Speed Control ---
        ui.label("Playback Speed");
        ui.add(egui::Slider::new(&mut playback_info.speed, 0.25..=2.0).logarithmic(false));
    });
}

// This system displays a panel with detailed audio analysis metrics.
fn audio_details_panel_system(
    mut contexts: EguiContexts,
    config: Res<VisualsConfig>,
    audio_analysis: Res<AudioAnalysis>,
    q_windows: Query<Entity, With<PrimaryWindow>>,
) {
    if q_windows.get_single().is_err() {
        return;
    }

    if !config.details_panel_enabled {
        return;
    }

    egui::Window::new("Audio Analysis Details")
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 10.0))
        .show(contexts.ctx_mut(), |ui| {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            ui.heading("📊 Basic Metrics");
            ui.label(format!("Volume (RMS): {:.3}", audio_analysis.volume));
            ui.separator();
            ui.heading("🎚️ Frequency Bands");
            ui.label(format!("Bass:         {:.2}", audio_analysis.bass));
            ui.label(format!("Mid:          {:.2}", audio_analysis.mid));
            ui.label(format!("Treble:       {:.2}", audio_analysis.treble));
            ui.separator();
            ui.heading("Raw Frequency Bins");
            egui::ScrollArea::vertical()
                .max_height(100.0)
                .show(ui, |ui| {
                    for (i, bin) in audio_analysis.frequency_bins.iter().enumerate() {
                        ui.label(format!("Bin {:02}: {:.3}", i, bin));
                    }
                });
        });
}

// A helper function to create a styled button for the main menu.
fn create_menu_button(parent: &mut ChildBuilder, text: &str, action: MenuButtonAction) {
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
            action,
        ))
        .with_children(|p| {
            p.spawn(TextBundle::from_section(
                text,
                TextStyle {
                    font_size: 24.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
        });
}

// Despawns all UI elements tagged with `MainMenuUI` when exiting the state.
fn cleanup_menu(mut commands: Commands, ui_query: Query<Entity, With<MainMenuUI>>) {
    for entity in ui_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
