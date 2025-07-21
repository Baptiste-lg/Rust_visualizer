// src/main.rs

// --- Modules ---
mod viz_2d;

// --- Imports ---
use bevy::prelude::*;
use rodio::{Decoder, OutputStream, Sink, source::SamplesConverter}; // Added for audio playback
use spectrum_analyzer::{samples_fft_to_spectrum, scaling::divide_by_N_sqrt, windows::hann_window, FrequencyLimit}; // Added for FFT
use std::{fs::File, io::BufReader, time::Instant};
use viz_2d::Viz2DPlugin;

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

/// Resource to hold the raw audio samples of the entire song
#[derive(Resource, Default)]
pub struct AudioSamples(pub Vec<f32>);

/// Resource to hold the start time of playback
#[derive(Resource)]
pub struct PlaybackStartTime(pub Instant);

/// Resource to hold the real-time audio analysis results
#[derive(Resource, Default)]
pub struct AudioAnalysis {
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
}

// --- Main Application Logic ---
fn main() {
    // Set up audio resources for playback
    let (stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    App::new()
        .add_plugins(DefaultPlugins)

        // Add audio playback resources
        .insert_non_send_resource(stream)
        .insert_non_send_resource(sink)

        // Add our custom resources and state machine
        .init_resource::<AudioSamples>()
        .init_resource::<AudioAnalysis>()
        .init_state::<AppState>()

        // Add the plugin for our 2D visualization
        .add_plugins(Viz2DPlugin)

        // Systems for MainMenu state
        .add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
        .add_systems(Update, menu_button_interaction.run_if(in_state(AppState::MainMenu)))
        .add_systems(OnExit(AppState::MainMenu), cleanup_menu)

        // System to start audio playback
        .add_systems(OnEnter(AppState::Visualization2D), play_audio_file)
        .add_systems(OnEnter(AppState::Visualization3D), play_audio_file)

        // Real audio analysis system
        .add_systems(Update, audio_analysis_system.run_if(in_state(AppState::Visualization2D).or_else(in_state(AppState::Visualization3D))))

        // Placeholder for the 3D scene
        .add_systems(OnEnter(AppState::Visualization3D), setup_3d_scene)
        .run();
}


// --- Core Systems ---

/// Loads the entire audio file into memory and starts playback.
fn play_audio_file(
    mut commands: Commands,
    sink: NonSend<Sink>,
    mut audio_samples: ResMut<AudioSamples>,
) {
    // --- Load the audio file ---
    let file = BufReader::new(File::open("assets/ShortClip.mp3").expect("Failed to open music file"));
    // Decode the audio file into a stream of samples
    let source = Decoder::new(file).unwrap();

    // We need to convert the audio to a single channel (mono) and a consistent sample rate
    // Bevy's default sample rate is 44100. Let's use that.
    let converter = SamplesConverter::new(source, 1, 44100);
    // Collect all samples into a Vec<f32> in memory. This is our raw data.
    audio_samples.0 = converter.collect();

    // --- Play the audio ---
    // Create a new audio source from our collected samples that can be played by the sink
    let new_source = rodio::buffer::SamplesBuffer::new(1, 44100, audio_samples.0.clone());
    sink.clear(); // Remove any previous audio
    sink.append(new_source);
    sink.play();

    // Store the exact start time of playback
    commands.insert_resource(PlaybackStartTime(Instant::now()));

    info!("Audio loaded and playback started.");
}


/// This system performs the actual FFT analysis on the audio data.
fn audio_analysis_system(
    audio_samples: Res<AudioSamples>,
    start_time: Option<Res<PlaybackStartTime>>,
    mut audio_analysis: ResMut<AudioAnalysis>,
) {
    // Only run if the audio has started playing
    let Some(start_time) = start_time else { return };
    if audio_samples.0.is_empty() { return };

    // --- Get current slice of audio data ---
    let elapsed = start_time.0.elapsed().as_secs_f32();
    let sample_rate = 44100.0;
    // The number of samples to analyze at once. Must be a power of 2.
    let fft_size = 4096;

    // Calculate the current position in the audio data
    let current_sample_index = (elapsed * sample_rate) as usize;

    // Ensure we don't go past the end of the song
    if current_sample_index + fft_size > audio_samples.0.len() {
        // Reset analysis when the song ends
        *audio_analysis = AudioAnalysis::default();
        return;
    }

    // Get the slice of samples for the FFT
    let samples_slice = &audio_samples.0[current_sample_index..current_sample_index + fft_size];

    // --- Perform FFT analysis ---
    // Apply a window function to the samples to improve FFT accuracy [cite: 60]
    let hann_window = hann_window(samples_slice);

    // Perform the FFT
    let spectrum = samples_fft_to_spectrum(
        &hann_window,
        sample_rate as u32,
        // Define frequency limits for the analysis
        FrequencyLimit::Range(20.0, 20000.0),
        // Use a logarithmic scaling for better visualization
        Some(&divide_by_N_sqrt),
    );

    // --- Map FFT results to frequency bands [cite: 61] ---
    let bass_limit = 250.0;
    let mid_limit = 4000.0;
    // let treble_limit = 20000.0; // This is the max

    let mut bass_val = 0.0;
    let mut mid_val = 0.0;
    let mut treble_val = 0.0;

    for (freq, val) in spectrum.data() {
        if *freq < bass_limit {
            bass_val += val.val();
        } else if *freq < mid_limit {
            mid_val += val.val();
        } else {
            treble_val += val.val();
        }
    }

    // Update the resource with the new values. We add a small multiplier to make the effect more visible.
    audio_analysis.bass = bass_val * 1.5;
    audio_analysis.mid = mid_val * 1.5;
    audio_analysis.treble = treble_val * 1.5;
}


// --- UI and Scene Systems (Complete but unchanged from before) ---

#[derive(Component)]
enum MenuButtonAction {
    Start3D,
    Start2D,
}
#[derive(Component)]
struct MainMenuUI;

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