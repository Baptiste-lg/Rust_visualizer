// src/audio.rs

use bevy::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rodio::{Decoder, Sink, source::Source};
use spectrum_analyzer::{samples_fft_to_spectrum, scaling::divide_by_N_sqrt, windows::hann_window, FrequencyLimit};
// MODIFIED: Removed unused `Cow` import
use std::io::Cursor;
use std::sync::mpsc::{Receiver, Sender};
use crate::{AppState, VisualizationEnabled};
use std::path::PathBuf;
use std::time::Duration;
use std::collections::VecDeque;

struct AudioDataTee<S> {
    source: S,
    sender: Sender<f32>,
}

impl<S> Iterator for AudioDataTee<S>
where
    S: Iterator<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.source.next()?;
        self.sender.send(sample).ok();
        Some(sample)
    }
}

impl<S> Source for AudioDataTee<S>
where
    S: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> { self.source.current_frame_len() }
    fn channels(&self) -> u16 { self.source.channels() }
    fn sample_rate(&self) -> u32 { self.source.sample_rate() }
    fn total_duration(&self) -> Option<Duration> { self.source.total_duration() }
}


pub struct AudioPlugin;

#[derive(Resource)]
pub struct AnalysisTimer(pub Timer);

#[derive(Resource, Clone)]
pub struct AnalysisAudioSender(pub Sender<f32>);
pub struct AnalysisAudioReceiver(pub Receiver<f32>);

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        let (mic_tx, mic_rx) = std::sync::mpsc::channel::<Vec<f32>>();
        let (analysis_tx, analysis_rx) = std::sync::mpsc::channel::<f32>();

        app
            .insert_resource(AnalysisTimer(Timer::new(Duration::from_secs_f32(1.0 / 60.0), TimerMode::Repeating)))
            .insert_resource(MicAudioSender(mic_tx))
            .insert_non_send_resource(MicAudioReceiver(mic_rx))
            .insert_resource(AnalysisAudioSender(analysis_tx))
            .insert_non_send_resource(AnalysisAudioReceiver(analysis_rx))
            .init_resource::<AudioSamples>()
            .init_resource::<AudioAnalysis>()
            .init_resource::<SelectedMic>()
            .init_resource::<MicAudioBuffer>()
            .add_systems(
                Update,
                (
                    read_mic_data_system,
                    read_analysis_data_system,
                    manage_audio_playback,
                    audio_analysis_system
                        .after(read_mic_data_system)
                        .after(read_analysis_data_system)
                        .after(manage_audio_playback)
                        .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0),
                )
                .run_if(in_state(AppState::Visualization2D).or_else(in_state(AppState::Visualization3D)))
            );
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioSource {
    File(PathBuf),
    Microphone,
    None,
}

impl Default for AudioSource {
    fn default() -> Self { AudioSource::None }
}

#[derive(Resource, Default)]
pub struct SelectedAudioSource(pub AudioSource);

#[derive(Resource, Default)]
pub struct SelectedMic(pub Option<String>);
#[derive(Resource, Clone)]
pub struct MicAudioSender(pub Sender<Vec<f32>>);
pub struct MicAudioReceiver(pub Receiver<Vec<f32>>);

// MODIFIED: Silenced benign warning. The stream must be held to keep it alive.
#[allow(dead_code)]
pub struct MicStream(pub Option<cpal::Stream>);

#[derive(Resource, Default)]
pub struct MicAudioBuffer(pub VecDeque<f32>);

#[derive(Resource, Default, Clone)]
pub struct AudioSamples(pub VecDeque<f32>);
#[derive(Resource)]
pub struct AudioInfo { pub sample_rate: u32 }
#[derive(Resource, Default)]
pub struct AudioAnalysis { pub bass: f32, pub mid: f32, pub treble: f32 }


fn manage_audio_playback(
    mut commands: Commands,
    selected_source: Res<SelectedAudioSource>,
    sink: NonSend<Sink>,
    mut mic_stream: NonSendMut<MicStream>,
    mic_sender: Res<MicAudioSender>,
    analysis_sender: Res<AnalysisAudioSender>,
    selected_mic: Res<SelectedMic>,
    mut audio_samples: ResMut<AudioSamples>,
){
    if !selected_source.is_changed() {
        return;
    }

    sink.stop();
    *mic_stream = MicStream(None);
    audio_samples.0.clear();

    match &selected_source.0 {
        AudioSource::File(path) => {
            info!("Streaming and analyzing file: {:?}", path);

            let file_bytes = std::fs::read(path).expect("Failed to read music file");
            let cursor = Cursor::new(file_bytes);
            let source = Decoder::new(cursor).unwrap();

            commands.insert_resource(AudioInfo { sample_rate: source.sample_rate() });

            let tee_source = AudioDataTee {
                source: source.convert_samples(),
                sender: analysis_sender.0.clone(),
            };

            sink.append(tee_source);
            sink.play();
        }
        AudioSource::Microphone => {
            info!("Starting microphone capture");
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
            *mic_stream = MicStream(Some(stream));
        }
        AudioSource::None => {
            info!("Stopping all audio");
        }
    }
}

pub fn read_analysis_data_system(receiver: Option<NonSend<AnalysisAudioReceiver>>, mut buffer: ResMut<AudioSamples>) {
    if let Some(receiver) = receiver {
        buffer.0.extend(receiver.0.try_iter());
    }
}

pub fn read_mic_data_system(receiver: Option<NonSend<MicAudioReceiver>>, mut buffer: ResMut<MicAudioBuffer>) {
    if let Some(receiver) = receiver {
        for new_data in receiver.0.try_iter() {
            buffer.0.extend(new_data);
        }
    }
}

pub fn audio_analysis_system(
    time: Res<Time>,
    mut analysis_timer: ResMut<AnalysisTimer>,
    mut audio_analysis: ResMut<AudioAnalysis>,
    audio_info: Option<Res<AudioInfo>>,
    audio_source: Res<SelectedAudioSource>,
    mut audio_samples: ResMut<AudioSamples>,
    mut mic_buffer: ResMut<MicAudioBuffer>,
) {
    analysis_timer.0.tick(time.delta());
    if !analysis_timer.0.just_finished() {
        return;
    }

    let Some(audio_info) = audio_info else { return };
    let fft_size = 4096;

    let analysis_buffer: Option<Vec<f32>> = match &audio_source.0 {
        AudioSource::File(_) => {
            if audio_samples.0.len() < fft_size {
                None
            } else {
                let buffer_len = audio_samples.0.len();
                let analysis_vec = audio_samples.0.iter().copied().take(fft_size).collect();
                let drain_amount = buffer_len.saturating_sub(fft_size / 2);
                audio_samples.0.drain(..drain_amount);
                Some(analysis_vec)
            }
        },
        AudioSource::Microphone => {
            if mic_buffer.0.len() < fft_size {
                None
            } else {
                let buffer_len = mic_buffer.0.len();
                let analysis_vec = mic_buffer.0.iter().copied().take(fft_size).collect();
                let drain_amount = buffer_len.saturating_sub(fft_size / 2);
                mic_buffer.0.drain(..drain_amount);
                Some(analysis_vec)
            }
        }
        AudioSource::None => None,
    };

    let Some(samples_slice) = analysis_buffer else { return };

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
}