// src/audio.rs

use bevy::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rodio::{Decoder, Sink, source::Source};
use spectrum_analyzer::{samples_fft_to_spectrum, scaling::divide_by_N_sqrt, windows::hann_window, FrequencyLimit};
use std::borrow::Cow;
use std::fs::File;
use std::io::BufReader;
use std::sync::mpsc::{Receiver, Sender};
use crate::AppState;

/// The plugin that encapsulates all audio logic.
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        let (mic_tx, mic_rx) = std::sync::mpsc::channel::<Vec<f32>>();

        app
            .insert_resource(MicAudioSender(mic_tx))
            .insert_non_send_resource(MicAudioReceiver(mic_rx))
            .init_resource::<AudioSamples>()
            .init_resource::<AudioAnalysis>()
            .init_resource::<SelectedAudioSource>()
            .init_resource::<MicAudioBuffer>()
            .init_resource::<SelectedMic>()
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
                    play_audio_file.run_if(|source: Res<SelectedAudioSource>| source.0 == AudioSource::File),
                    setup_microphone.run_if(|source: Res<SelectedAudioSource>| source.0 == AudioSource::Microphone),
                )
            )
            .add_systems(
                Update,
                (
                    read_mic_data_system.run_if(|source: Res<SelectedAudioSource>| source.0 == AudioSource::Microphone),
                    audio_analysis_system,
                )
                .run_if(in_state(AppState::Visualization2D).or_else(in_state(AppState::Visualization3D)))
            );
    }
}


// --- Audio-related structs and enums ---

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AudioSource { #[default] File, Microphone }
#[derive(Resource, Default)]
pub struct SelectedAudioSource(pub AudioSource);
#[derive(Resource, Default)]
pub struct SelectedMic(pub Option<String>);
#[derive(Resource, Clone)]
pub struct MicAudioSender(pub Sender<Vec<f32>>);
pub struct MicAudioReceiver(pub Receiver<Vec<f32>>);
#[derive(Resource, Default)]
pub struct MicAudioBuffer(pub Vec<f32>);
#[derive(Resource, Default, Clone)]
pub struct AudioSamples(pub Vec<f32>);
#[derive(Resource)]
pub struct AudioInfo { pub sample_rate: u32 }
#[derive(Resource)]
pub struct PlaybackStartTime(pub std::time::Instant);
#[derive(Resource, Default)]
pub struct AudioAnalysis { pub bass: f32, pub mid: f32, pub treble: f32 }


// --- Audio Systems ---

fn setup_microphone(
    mut commands: Commands,
    mic_sender: Res<MicAudioSender>,
    selected_mic: Res<SelectedMic>,
) {
    let host = cpal::default_host();
    let device = selected_mic.0.as_ref().and_then(|name| {
        host.input_devices().ok()?.find(|d| d.name().unwrap_or_default() == *name)
    }).unwrap_or_else(|| {
        host.default_input_device().expect("No default audio input device found")
    });
    let config = device.default_input_config().expect("Failed to get default input config");
    info!("Initializing microphone: {} with config {:?}", device.name().unwrap(), config);
    commands.insert_resource(AudioInfo { sample_rate: config.sample_rate().0 });
    let tx = mic_sender.0.clone();
    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| { tx.send(data.to_vec()).ok(); },
        |err| error!("An error occurred on the audio stream: {}", err),
        None
    ).expect("Failed to build input stream");
    stream.play().expect("Failed to play audio stream");
    std::mem::forget(stream);
    info!("Microphone capture started.");
}

fn read_mic_data_system(receiver: Option<NonSend<MicAudioReceiver>>, mut buffer: ResMut<MicAudioBuffer>) {
    if let Some(receiver) = receiver {
        for new_data in receiver.0.try_iter() {
            buffer.0.extend(new_data);
        }
    }
}

fn play_audio_file(mut commands: Commands, sink: NonSend<Sink>, mut audio_samples: ResMut<AudioSamples>) {
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
    commands.insert_resource(PlaybackStartTime(std::time::Instant::now()));
    info!("Audio file loaded and playback started.");
}

fn audio_analysis_system(
    mut audio_analysis: ResMut<AudioAnalysis>,
    audio_info: Option<Res<AudioInfo>>,
    audio_source: Res<SelectedAudioSource>,
    audio_samples: Res<AudioSamples>,
    start_time: Option<Res<PlaybackStartTime>>,
    mut mic_buffer: ResMut<MicAudioBuffer>,
) {
    let Some(audio_info) = audio_info else { return };
    let fft_size = 4096;

    let samples_slice: Cow<[f32]> = match audio_source.0 {
        AudioSource::File => {
            if let Some(start_time) = start_time {
                if audio_samples.0.is_empty() { return; }
                let elapsed = start_time.0.elapsed().as_secs_f32();
                let current_sample_index = (elapsed * audio_info.sample_rate as f32) as usize;
                if current_sample_index + fft_size > audio_samples.0.len() { return; }
                Cow::from(&audio_samples.0[current_sample_index..current_sample_index + fft_size])
            } else {
                return;
            }
        },
        AudioSource::Microphone => {
            if mic_buffer.0.len() < fft_size {
                return;
            }
            let buffer_len = mic_buffer.0.len();
            let analysis_vec = mic_buffer.0[buffer_len - fft_size..].to_vec();
            let drain_amount = buffer_len - (fft_size / 2);
            mic_buffer.0.drain(0..drain_amount);
            Cow::from(analysis_vec)
        }
    };

    if samples_slice.is_empty() { return; }

    let hann_window = hann_window(&samples_slice);
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

    info!(
        "Audio Analysis -- Bass: {:.4}, Mid: {:.4}, Treble: {:.4}",
        audio_analysis.bass,
        audio_analysis.mid,
        audio_analysis.treble
    );
}