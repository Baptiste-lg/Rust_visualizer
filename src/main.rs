// src/main.rs

// --- Modules ---
mod viz_2d;
mod viz_3d;

// --- Imports ---
use bevy::prelude::*;
use rodio::{source::Source, Decoder, OutputStream, Sink};
use spectrum_analyzer::{samples_fft_to_spectrum, scaling::divide_by_N_sqrt, windows::hann_window, FrequencyLimit};
use std::{fs::File, io::BufReader, time::Instant, sync::mpsc::{Receiver, Sender}};
use viz_2d::Viz2DPlugin;
use viz_3d::Viz3DPlugin;

// --- Imports for CPAL ---
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

// --- Enums and Structs for Audio Source Selection ---
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AudioSource {
    #[default]
    File,
    Microphone,
}

#[derive(Resource, Default)]
pub struct SelectedAudioSource(pub AudioSource);

// THE FIX: Create separate resources for the Sender (thread-safe) and Receiver (not)
#[derive(Resource, Clone)]
struct MicAudioSender(Sender<Vec<f32>>);

struct MicAudioReceiver(Receiver<Vec<f32>>);

#[derive(Resource, Default)]
struct MicAudioBuffer(pub Vec<f32>);


// --- Bevy States and Components ---
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,
    Visualization3D,
    Visualization2D,
}

#[derive(Component)]
enum MenuButtonAction {
    Start3DFile,
    Start2DFile,
    Start3DMic,
    Start2DMic,
}

#[derive(Component)]
struct MainMenuUI;

// --- Core Audio Resources ---
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

// --- Main Application Logic ---
fn main() {
    // THE FIX: Create the channel here, in the main thread.
    let (mic_tx, mic_rx) = std::sync::mpsc::channel::<Vec<f32>>();

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_non_send_resource(OutputStream::try_default().unwrap())
        .insert_non_send_resource(Sink::try_new(&OutputStream::try_default().unwrap().1).unwrap())

        // THE FIX: Insert the channel ends into the app here.
        .insert_resource(MicAudioSender(mic_tx))
        .insert_non_send_resource(MicAudioReceiver(mic_rx))

        .init_resource::<AudioSamples>()
        .init_resource::<AudioAnalysis>()
        .init_resource::<SelectedAudioSource>()
        .init_resource::<MicAudioBuffer>()
        .init_state::<AppState>()

        .add_plugins(Viz2DPlugin)
        .add_plugins(Viz3DPlugin)

        .add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
        .add_systems(Update, menu_button_interaction.run_if(in_state(AppState::MainMenu)))
        .add_systems(OnExit(AppState::MainMenu), cleanup_menu)

        .add_systems(
            OnEnter(AppState::Visualization2D),
            (
                play_audio_file.run_if(|source: Res<SelectedAudioSource>| source.0 == AudioSource::File),
                setup_microphone.run_if(|source: Res<SelectedAudioSource>| source.0 == AudioSource::Microphone),
            )
        )
        .add_systems(
            OnEnter(AppState::Visualization3D),
            (
                (play_audio_file, setup_3d_scene).run_if(|source: Res<SelectedAudioSource>| source.0 == AudioSource::File),
                (setup_microphone, setup_3d_scene).run_if(|source: Res<SelectedAudioSource>| source.0 == AudioSource::Microphone),
            )
        )

        .add_systems(
            Update,
            (
                read_mic_data_system.run_if(|source: Res<SelectedAudioSource>| source.0 == AudioSource::Microphone),
                audio_analysis_system,
            )
            .run_if(in_state(AppState::Visualization2D).or_else(in_state(AppState::Visualization3D)))
        )
        .run();
}


// --- Audio Systems ---

/// This system now gets the Sender from a resource.
fn setup_microphone(
    mut commands: Commands,
    mic_sender: Res<MicAudioSender>, // THE FIX: Get the sender as a resource.
) {
    let host = cpal::default_host();
    let device = host.default_input_device().expect("No audio input device found");
    let config = device.default_input_config().expect("Failed to get default input config");

    info!("Initializing microphone: {} with config {:?}", device.name().unwrap(), config);

    commands.insert_resource(AudioInfo { sample_rate: config.sample_rate().0 });

    // THE FIX: Clone the sender for the audio thread.
    let tx = mic_sender.0.clone();
    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            tx.send(data.to_vec()).ok();
        },
        |err| error!("An error occurred on the audio stream: {}", err),
        None
    ).expect("Failed to build input stream");

    stream.play().expect("Failed to play audio stream");
    std::mem::forget(stream);

    // THE FIX: No longer need to insert anything here.
    info!("Microphone capture started.");
}

/// This system reads from the microphone channel and updates a resource.
fn read_mic_data_system(
    receiver: Option<NonSend<MicAudioReceiver>>,
    mut buffer: ResMut<MicAudioBuffer>
) {
    if let Some(receiver) = receiver {
        if let Some(last_data) = receiver.0.try_iter().last() {
            buffer.0 = last_data;
        }
    }
}

/// This system now only runs if the source is a file.
fn play_audio_file(
    mut commands: Commands,
    sink: NonSend<Sink>,
    mut audio_samples: ResMut<AudioSamples>,
) {
    let file = BufReader::new(File::open("assets/ShortClip.mp3").expect("Failed to open music file"));
    let source = Decoder::new(file).unwrap();

    let sample_rate = source.sample_rate();
    commands.insert_resource(AudioInfo { sample_rate });
    let samples: Vec<f32> = source.convert_samples::<f32>().collect();

    audio_samples.0 = samples;
    let new_source = rodio::buffer::SamplesBuffer::new(1, sample_rate, audio_samples.0.clone());
    sink.clear();
    sink.append(new_source);
    sink.play();

    commands.insert_resource(PlaybackStartTime(Instant::now()));
    info!("Audio file loaded and playback started.");
}

/// The analysis system now handles both audio sources.
fn audio_analysis_system(
    mut audio_analysis: ResMut<AudioAnalysis>,
    audio_info: Option<Res<AudioInfo>>,
    audio_source: Res<SelectedAudioSource>,
    audio_samples: Res<AudioSamples>,
    start_time: Option<Res<PlaybackStartTime>>,
    mic_buffer: Res<MicAudioBuffer>,
) {
    let Some(audio_info) = audio_info else { return };
    let fft_size = 4096;

    let samples_slice: &[f32] = match audio_source.0 {
        AudioSource::File => {
            let Some(start_time) = start_time else { return };
            if audio_samples.0.is_empty() { return; }

            let elapsed = start_time.0.elapsed().as_secs_f32();
            let current_sample_index = (elapsed * audio_info.sample_rate as f32) as usize;

            if current_sample_index + fft_size > audio_samples.0.len() {
                return;
            }
            &audio_samples.0[current_sample_index..current_sample_index + fft_size]
        },
        AudioSource::Microphone => {
            if mic_buffer.0.len() < fft_size {
                return;
            }
            &mic_buffer.0[mic_buffer.0.len() - fft_size..]
        }
    };

    let hann_window = hann_window(samples_slice);
    let spectrum = samples_fft_to_spectrum(
        &hann_window,
        audio_info.sample_rate,
        FrequencyLimit::Range(20.0, 20000.0),
        Some(&divide_by_N_sqrt),
    ).expect("Failed to compute spectrum");

    let bass_limit = 250.0;
    let mid_limit = 4000.0;
    let (mut bass_val, mut mid_val, mut treble_val) = (0.0, 0.0, 0.0);

    for (freq, val) in spectrum.data() {
        if *freq < bass_limit.into() { bass_val += val.val(); }
        else if *freq < mid_limit.into() { mid_val += val.val(); }
        else { treble_val += val.val(); }
    }

    let smoothing = 0.5;
    audio_analysis.bass = audio_analysis.bass * smoothing + (bass_val * 1.5) * (1.0 - smoothing);
    audio_analysis.mid = audio_analysis.mid * smoothing + (mid_val * 1.5) * (1.0 - smoothing);
    audio_analysis.treble = audio_analysis.treble * smoothing + (treble_val * 1.5) * (1.0 - smoothing);
}


// --- UI and Scene Systems (No changes below this line) ---

fn setup_main_menu(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), MainMenuUI));
    commands
        .spawn((NodeBundle {
            style: Style {
                width: Val::Percent(100.0), height: Val::Percent(100.0),
                align_items: AlignItems::Center, justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column, row_gap: Val::Px(20.0),
                ..default()
            },
            ..default()
        }, MainMenuUI))
        .with_children(|parent| {
            create_menu_button(parent, "Start 3D (File)", MenuButtonAction::Start3DFile);
            create_menu_button(parent, "Start 2D (File)", MenuButtonAction::Start2DFile);
            create_menu_button(parent, "Start 3D (Mic)", MenuButtonAction::Start3DMic);
            create_menu_button(parent, "Start 2D (Mic)", MenuButtonAction::Start2DMic);
        });
}

fn create_menu_button(parent: &mut ChildBuilder, text: &str, action: MenuButtonAction) {
    parent
        .spawn((
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
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                text,
                TextStyle { font_size: 24.0, color: Color::WHITE, ..default() },
            ));
        });
}

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