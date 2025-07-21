// src/main.rs

// --- Modules ---
mod viz_2d;
mod viz_3d;

// --- Imports ---
use bevy::prelude::*;
use rodio::{source::Source, Decoder, OutputStream, Sink};
use spectrum_analyzer::{samples_fft_to_spectrum, scaling::divide_by_N_sqrt, windows::hann_window, FrequencyLimit};
use std::{fs::File, io::BufReader, time::Instant};
use viz_2d::Viz2DPlugin;
use viz_3d::Viz3DPlugin;

// --- Imports for CPAL ---
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

// --- Enums and Structs ---
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
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

#[derive(Resource, Default, Clone)]
pub struct AudioSamples(pub Vec<f32>);

#[derive(Resource)]
pub struct AudioInfo {
    pub sample_rate: u32,
}

#[derive(Resource)]
pub struct PlaybackStartTime(pub Instant);

#[derive(Resource, Default)]
pub struct AudioAnalysis {
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
}

// REMOVED the MicStream struct. We don't need it.

// --- Main Application Logic ---
fn main() {
    let (stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_non_send_resource(stream)
        .insert_non_send_resource(sink)
        .init_resource::<AudioSamples>()
        .init_resource::<AudioAnalysis>()
        .init_state::<AppState>()

        .add_plugins(Viz2DPlugin)
        .add_plugins(Viz3DPlugin)

        // Add the microphone setup system to run at startup.
        .add_systems(Startup, setup_microphone)

        .add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
        .add_systems(Update, menu_button_interaction.run_if(in_state(AppState::MainMenu)))
        .add_systems(OnExit(AppState::MainMenu), cleanup_menu)

        .add_systems(OnEnter(AppState::Visualization2D), play_audio_file)
        .add_systems(OnEnter(AppState::Visualization3D), (play_audio_file, setup_3d_scene))
        .add_systems(Update, audio_analysis_system.run_if(in_state(AppState::Visualization2D).or_else(in_state(AppState::Visualization3D))))

        .run();
}


// --- Microphone System ---

/// Initializes the default microphone input stream using cpal.
fn setup_microphone() {
    let host = cpal::default_host();
    let device = host.default_input_device().expect("No audio input device found");
    info!("Audio input device: {}", device.name().unwrap_or_else(|_| "Unknown name".into()));

    let config = device.default_input_config().expect("Failed to get default input config");
    info!("Default input config: {:?}", config);

    // Create a channel to send audio data from the audio thread to our logging thread.
    let (tx, rx) = std::sync::mpsc::channel::<Vec<f32>>();

    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // Send the buffer data to the main thread via the channel.
            tx.send(data.to_vec()).unwrap();
        },
        |err| {
            error!("An error occurred on the audio stream: {}", err);
        },
        None
    ).expect("Failed to build input stream");

    stream.play().expect("Failed to play audio stream");

    // **THE FIX**: Leak the stream.
    // This detaches the stream from the current scope and keeps it alive for the
    // duration of the program. This is necessary because the stream is not `Send`.
    std::mem::forget(stream);

    // Spawn a separate thread to listen for and log the microphone data.
    std::thread::spawn(move || {
        info!("Microphone listening thread started.");
        loop {
            match rx.recv() {
                Ok(buffer) => {
                    let sum: f32 = buffer.iter().map(|&x| x.abs()).sum();
                    let avg_volume = sum / buffer.len() as f32;
                    info!("[Microphone] Buffer received. Length: {}, Avg volume: {:.4}", buffer.len(), avg_volume);
                }
                Err(e) => {
                    error!("Error receiving microphone data: {}", e);
                    break;
                }
            }
        }
    });

    info!("Microphone capture successfully initialized.");
}


// --- Core Systems ---

fn play_audio_file(
    mut commands: Commands,
    sink: NonSend<Sink>,
    mut audio_samples: ResMut<AudioSamples>,
) {
    let file = BufReader::new(File::open("assets/ShortClip.mp3").expect("Failed to open music file"));
    let source = Decoder::new(file).unwrap();

    let sample_rate = source.sample_rate();
    let channels = source.channels();
    commands.insert_resource(AudioInfo { sample_rate });

    let mut f32_source = source.convert_samples::<f32>();

    let samples: Vec<f32> = if channels == 2 {
        let mut mono_samples = Vec::new();
        while let (Some(left), Some(right)) = (f32_source.next(), f32_source.next()) {
            mono_samples.push((left + right) / 2.0);
        }
        mono_samples
    } else {
        f32_source.collect()
    };

    audio_samples.0 = samples;

    let new_source = rodio::buffer::SamplesBuffer::new(1, sample_rate, audio_samples.0.clone());
    sink.clear();
    sink.append(new_source);
    sink.play();

    commands.insert_resource(PlaybackStartTime(Instant::now()));
    info!("Audio loaded and playback started.");
}

fn audio_analysis_system(
    audio_samples: Res<AudioSamples>,
    audio_info: Option<Res<AudioInfo>>,
    start_time: Option<Res<PlaybackStartTime>>,
    mut audio_analysis: ResMut<AudioAnalysis>,
) {
    let (Some(start_time), Some(audio_info)) = (start_time, audio_info) else { return };
    if audio_samples.0.is_empty() { return };

    let elapsed = start_time.0.elapsed().as_secs_f32();
    let fft_size = 4096;
    let current_sample_index = (elapsed * audio_info.sample_rate as f32) as usize;

    if current_sample_index + fft_size > audio_samples.0.len() {
        *audio_analysis = AudioAnalysis::default();
        return;
    }

    let samples_slice = &audio_samples.0[current_sample_index..current_sample_index + fft_size];
    let hann_window = hann_window(samples_slice);

    let spectrum_result = samples_fft_to_spectrum(
        &hann_window,
        audio_info.sample_rate,
        FrequencyLimit::Range(20.0, 20000.0),
        Some(&divide_by_N_sqrt),
    );

    let spectrum = spectrum_result.expect("Failed to compute spectrum");

    let bass_limit = 250.0;
    let mid_limit = 4000.0;

    let mut bass_val = 0.0;
    let mut mid_val = 0.0;
    let mut treble_val = 0.0;

    for (freq, val) in spectrum.data() {
        if *freq < bass_limit.into() {
            bass_val += val.val();
        } else if *freq < mid_limit.into() {
            mid_val += val.val();
        } else {
            treble_val += val.val();
        }
    }

    let smoothing = 0.5;
    audio_analysis.bass = audio_analysis.bass * smoothing + (bass_val * 1.5) * (1.0 - smoothing);
    audio_analysis.mid = audio_analysis.mid * smoothing + (mid_val * 1.5) * (1.0 - smoothing);
    audio_analysis.treble = audio_analysis.treble * smoothing + (treble_val * 1.5) * (1.0 - smoothing);
}


// --- UI and Scene Systems ---

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

fn cleanup_menu(mut commands: Commands, ui_query: Query<Entity, With<MainMenuUI>>) {
    info!("Cleaning up main menu UI...");
    for entity in ui_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn setup_3d_scene(mut commands: Commands) {
    info!("Setting up 3D scene...");
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-12.0, 10.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 2000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
}