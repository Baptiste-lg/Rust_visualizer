// src/audio.rs

use bevy::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rodio::{Decoder, Sink, source::Source};
use spectrum_analyzer::{samples_fft_to_spectrum, scaling::divide_by_N_sqrt, windows::hann_window, FrequencyLimit};
use std::io::Cursor;
use std::sync::mpsc::{Receiver, Sender};
use crate::{AppState, VisualizationEnabled, config::VisualsConfig};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::collections::VecDeque;

// A tee adapter for audio data, sending a copy to an analysis channel.
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
        self.sender.send(sample).ok(); // Send a copy for analysis
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
                    apply_playback_changes.after(manage_audio_playback),
                    update_playback_position.after(apply_playback_changes),
                    audio_analysis_system
                        .after(read_mic_data_system)
                        .after(read_analysis_data_system)
                        .after(manage_audio_playback)
                        .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0),
                )
                .run_if(in_state(AppState::Visualization2D)
                    .or_else(in_state(AppState::Visualization3D))
                    .or_else(in_state(AppState::VisualizationOrb))
                    .or_else(in_state(AppState::VisualizationDisc)))
            );
    }
}

// Represents the current playback status.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PlaybackStatus {
    #[default]
    Paused,
    Playing,
}

// A resource to hold all information related to audio playback control.
#[derive(Resource, Debug, Default)]
pub struct PlaybackInfo {
    pub status: PlaybackStatus,
    pub speed: f32,
    pub position: Duration,
    pub duration: Duration,
    pub seek_to: Option<f32>, // A signal from the UI to seek to a new position (in seconds)
    // Internal state for tracking playback time
    pub(crate) last_update: Option<Instant>,
}

impl PlaybackInfo {
    // Resets the state, typically when no file is loaded.
    pub fn reset(&mut self) {
        self.status = PlaybackStatus::Paused;
        self.speed = 1.0;
        self.position = Duration::ZERO;
        self.duration = Duration::ZERO;
        self.seek_to = None;
        self.last_update = None;
    }
}


#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub enum AudioSource {
    File(PathBuf),
    Microphone,
    #[default]
    None,
}

#[derive(Resource, Default)]
pub struct SelectedAudioSource(pub AudioSource);

#[derive(Resource, Default)]
pub struct SelectedMic(pub Option<String>);

#[derive(Resource, Clone)]
pub struct MicAudioSender(pub Sender<Vec<f32>>);
pub struct MicAudioReceiver(pub Receiver<Vec<f32>>);

#[allow(dead_code)]
pub struct MicStream(pub Option<cpal::Stream>);

#[derive(Resource, Default)]
pub struct MicAudioBuffer(pub VecDeque<f32>);

#[derive(Resource, Default, Clone)]
pub struct AudioSamples(pub VecDeque<f32>);
#[derive(Resource)]
pub struct AudioInfo { pub sample_rate: u32 }

#[derive(Resource, Default)]
pub struct AudioAnalysis {
    pub frequency_bins: Vec<f32>,
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
    pub treble_average: f32,
    pub previous_spectrum: Vec<(f32, f32)>,
    pub volume: f32,
    pub energy: f32,
    pub centroid: f32,
    pub flux: f32,
    pub rolloff: f32,
}

// This system is responsible for starting audio playback when the source changes.
pub fn manage_audio_playback(
    mut commands: Commands,
    selected_source: Res<SelectedAudioSource>,
    sink: NonSend<Sink>,
    mut mic_stream: NonSendMut<MicStream>,
    mic_sender: Res<MicAudioSender>,
    analysis_sender: Res<AnalysisAudioSender>,
    selected_mic: Res<SelectedMic>,
    mut audio_samples: ResMut<AudioSamples>,
    mut playback_info: ResMut<PlaybackInfo>,
){
    if !selected_source.is_changed() {
        return;
    }

    // Stop all current audio and reset state
    sink.stop();
    *mic_stream = MicStream(None);
    audio_samples.0.clear();
    playback_info.reset(); // Reset playback info on source change

    match &selected_source.0 {
        AudioSource::File(path) => {
            info!("Streaming and analyzing file: {:?}", path);

            let file_bytes = std::fs::read(path).expect("Failed to read music file");
            let cursor = Cursor::new(file_bytes);
            let source = Decoder::new(cursor).unwrap();

            commands.insert_resource(AudioInfo { sample_rate: source.sample_rate() });

            // Update playback info with the new file's duration
            playback_info.duration = source.total_duration().unwrap_or_default();
            playback_info.status = PlaybackStatus::Playing;
            playback_info.last_update = Some(Instant::now());

            let tee_source = AudioDataTee {
                source: source.convert_samples(),
                sender: analysis_sender.0.clone(),
            };

            sink.append(tee_source);
            // Don't auto-play here, let apply_playback_changes handle it
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
            // State is already reset above
        }
    }
}

// This system applies changes from the UI (play, pause, speed, seek) to the audio sink.
fn apply_playback_changes(
    mut playback_info: ResMut<PlaybackInfo>,
    sink: NonSend<Sink>,
    selected_source: Res<SelectedAudioSource>,
    analysis_sender: Res<AnalysisAudioSender>,
) {
    if !playback_info.is_changed() {
        return;
    }

    // Handle Play/Pause state changes
    match playback_info.status {
        PlaybackStatus::Playing => {
            if sink.is_paused() {
                sink.play();
                playback_info.last_update = Some(Instant::now());
            }
        }
        PlaybackStatus::Paused => {
            if !sink.is_paused() {
                sink.pause();
                // Update position before pausing
                if let Some(last_update) = playback_info.last_update.take() {
                    let elapsed = last_update.elapsed().as_secs_f32() * sink.speed();
                    playback_info.position += Duration::from_secs_f32(elapsed);
                }
            }
        }
    }

    // Handle speed changes
    if sink.speed() != playback_info.speed {
        // Update position before changing speed
        if !sink.is_paused() {
            if let Some(last_update) = playback_info.last_update.take() {
                let elapsed = last_update.elapsed().as_secs_f32() * sink.speed();
                playback_info.position += Duration::from_secs_f32(elapsed);
                playback_info.last_update = Some(Instant::now());
            }
        }
        sink.set_speed(playback_info.speed);
    }

    // Handle seeking
    if let Some(seek_pos_secs) = playback_info.seek_to.take() {
        if let AudioSource::File(path) = &selected_source.0 {
            info!("Seeking to {} seconds", seek_pos_secs);
            let seek_duration = Duration::from_secs_f32(seek_pos_secs);

            // Recreate the source to seek
            let file_bytes = std::fs::read(path).expect("Failed to read music file for seeking");
            let cursor = Cursor::new(file_bytes);
            let source = Decoder::new(cursor).unwrap();

            let new_source = source.skip_duration(seek_duration).convert_samples();

            let tee_source = AudioDataTee {
                source: new_source,
                sender: analysis_sender.0.clone(),
            };

            // Replace the sink's content
            sink.stop();
            sink.clear();
            sink.append(tee_source);

            // Update playback info
            playback_info.position = seek_duration;
            if playback_info.status == PlaybackStatus::Playing {
                sink.play();
                playback_info.last_update = Some(Instant::now());
            } else {
                sink.pause();
                playback_info.last_update = None;
            }
        }
    }
}

// This system continuously updates the playback position for the UI progress bar.
fn update_playback_position(
    mut playback_info: ResMut<PlaybackInfo>,
    sink: NonSend<Sink>,
) {
    if playback_info.status == PlaybackStatus::Playing {
        if let Some(last_update) = playback_info.last_update {
            let elapsed_since_update = last_update.elapsed().as_secs_f32() * sink.speed();
            let new_pos = playback_info.position + Duration::from_secs_f32(elapsed_since_update);

            if new_pos > playback_info.duration && playback_info.duration != Duration::ZERO {
                // Reset to end when playback finishes
                playback_info.position = playback_info.duration;
                playback_info.status = PlaybackStatus::Paused;
                playback_info.last_update = None;
            } else {
                // This is a bit hacky. We don't want to constantly mark the resource as changed
                // just for the time update, as it would re-trigger apply_playback_changes.
                // The UI will read this value directly. So we update the inner value without
                // triggering bevy's change detection.
                let ptr = &mut *playback_info as *mut PlaybackInfo;
                unsafe { (*ptr).position = new_pos; }
            }
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
    config: Res<VisualsConfig>,
) {
    analysis_timer.0.tick(time.delta());
    if !analysis_timer.0.just_finished() {
        return;
    }

    let Some(audio_info) = audio_info else { return };
    let fft_size = 4096;

    let analysis_buffer: Option<Vec<f32>> = match &audio_source.0 {
        AudioSource::File(_) => {
            if audio_samples.0.len() < fft_size { None } else {
                let buffer_len = audio_samples.0.len();
                let analysis_vec = audio_samples.0.iter().copied().take(fft_size).collect();
                let drain_amount = buffer_len.saturating_sub(fft_size / 2);
                audio_samples.0.drain(..drain_amount);
                Some(analysis_vec)
            }
        },
        AudioSource::Microphone => {
            if mic_buffer.0.len() < fft_size { None } else {
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

    let squared_sum = samples_slice.iter().map(|s| s * s).sum::<f32>();
    audio_analysis.volume = (squared_sum / samples_slice.len() as f32).sqrt();
    audio_analysis.energy = squared_sum;

    let spectrum_data: Vec<(f32, f32)> = spectrum.data().iter().map(|(f, v)| (f.val(), v.val())).collect();
    let total_magnitude = spectrum_data.iter().map(|&(_, mag)| mag).sum::<f32>();

    if total_magnitude > 0.0 {
        let weighted_freq_sum = spectrum_data.iter().map(|&(freq, mag)| freq * mag).sum::<f32>();
        audio_analysis.centroid = weighted_freq_sum / total_magnitude;

        let rolloff_threshold = total_magnitude * 0.85;
        let mut cumulative_magnitude = 0.0;
        for &(freq, mag) in &spectrum_data {
            cumulative_magnitude += mag;
            if cumulative_magnitude >= rolloff_threshold {
                audio_analysis.rolloff = freq;
                break;
            }
        }

        if !audio_analysis.previous_spectrum.is_empty() && audio_analysis.previous_spectrum.len() == spectrum_data.len() {
            let sum_of_squared_diffs = spectrum_data.iter().zip(&audio_analysis.previous_spectrum)
                .map(|((_, cur_mag), (_, prev_mag))| (cur_mag - prev_mag).powi(2))
                .sum::<f32>();
            audio_analysis.flux = sum_of_squared_diffs.sqrt();
        } else {
            audio_analysis.flux = 0.0;
        }
    }
    audio_analysis.previous_spectrum = spectrum_data.clone();

    let num_bands = config.num_bands;
    let mut new_bins = vec![0.0; num_bands];
    let min_freq = 20.0f32;
    let max_freq = 20000.0f32;
    let band_limits: Vec<f32> = (0..num_bands).map(|i| {
        min_freq * (max_freq / min_freq).powf((i as f32 + 1.0) / num_bands as f32)
    }).collect();

    let mut current_band = 0;
    let mut treble_val = 0.0;

    for (freq, val) in spectrum.data() {
        if current_band < num_bands - 1 && freq.val() > band_limits[current_band] {
            current_band += 1;
        }
        new_bins[current_band] += val.val();

        if freq.val() > 4000.0 {
            treble_val += val.val();
        }
    }

    let smoothing = 0.5;
    if audio_analysis.frequency_bins.len() != num_bands {
        audio_analysis.frequency_bins.resize(num_bands, 0.0);
    }

    for i in 0..num_bands {
        audio_analysis.frequency_bins[i] = audio_analysis.frequency_bins[i] * smoothing + new_bins[i] * (1.0 - smoothing);
    }
    audio_analysis.treble_average = audio_analysis.treble_average * smoothing + treble_val * (1.0 - smoothing);

    audio_analysis.bass = audio_analysis.frequency_bins.iter().take(num_bands / 4).sum();
    audio_analysis.mid = audio_analysis.frequency_bins.iter().skip(num_bands / 4).take(num_bands / 2).sum();
    audio_analysis.treble = audio_analysis.frequency_bins.iter().skip(3 * num_bands / 4).sum();
}