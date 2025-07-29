// src/audio.rs

use crate::{config::VisualsConfig, AppState, VisualizationEnabled};
use bevy::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rodio::{source::Source, Decoder, Sink};
use spectrum_analyzer::{
    samples_fft_to_spectrum, scaling::divide_by_N_sqrt, windows::hann_window, FrequencyLimit,
};
use std::collections::VecDeque;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

// --- Symphonia Helper ---

// This function uses the Symphonia library to reliably get the duration of an audio file.
// It's much more accurate for formats like VBR MP3 than other libraries.
fn get_duration_with_symphonia(path: &Path) -> Result<Duration, Box<dyn std::error::Error>> {
    let src = std::fs::File::open(path)?;
    let mss = symphonia::core::io::MediaSourceStream::new(Box::new(src), Default::default());

    let hint = symphonia::core::probe::Hint::new();
    let meta_opts: symphonia::core::meta::MetadataOptions = Default::default();
    let fmt_opts: symphonia::core::formats::FormatOptions = Default::default();

    let probed = symphonia::default::get_probe().format(&hint, mss, &fmt_opts, &meta_opts)?;
    let format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or("No supported audio track found")?;

    let time_base = track.codec_params.time_base.ok_or("Missing time base")?;
    let n_frames = track.codec_params.n_frames.ok_or("Missing frame count")?;

    let total_time = time_base.calc_time(n_frames);

    Ok(Duration::from_secs(total_time.seconds) + Duration::from_secs_f64(total_time.frac))
}

// --- Bevy Plugin and Components ---

// A "tee" adapter for audio data. While the audio plays, this struct
// sends a copy of each raw audio sample to a channel for real-time analysis.
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
        self.sender.send(sample).ok(); // Send a copy for analysis.
        Some(sample)
    }
}

impl<S> Source for AudioDataTee<S>
where
    S: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.source.current_frame_len()
    }
    fn channels(&self) -> u16 {
        self.source.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.source.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }
}

pub struct AudioPlugin;

#[derive(Resource)]
pub struct AnalysisTimer(pub Timer);

#[derive(Resource, Clone)]
pub struct AnalysisAudioSender(pub Sender<f32>);
pub struct AnalysisAudioReceiver(pub Receiver<f32>);

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        let (analysis_tx, analysis_rx) = std::sync::mpsc::channel::<f32>();

        app.insert_resource(AnalysisTimer(Timer::new(
            Duration::from_secs_f32(1.0 / 60.0),
            TimerMode::Repeating,
        )))
        .insert_resource(AnalysisAudioSender(analysis_tx))
        .insert_non_send_resource(AnalysisAudioReceiver(analysis_rx))
        .init_resource::<AudioSamples>()
        .init_resource::<AudioAnalysis>()
        .init_resource::<SelectedMic>()
        .add_systems(
            Update,
            (
                read_analysis_data_system,
                manage_audio_playback,
                apply_playback_changes.after(manage_audio_playback),
                update_playback_position.after(apply_playback_changes),
                audio_analysis_system
                    .after(read_analysis_data_system)
                    .after(manage_audio_playback)
                    .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0),
            )
                .run_if(
                    in_state(AppState::Visualization2D)
                        .or_else(in_state(AppState::Visualization3D))
                        .or_else(in_state(AppState::VisualizationOrb))
                        .or_else(in_state(AppState::VisualizationDisc)),
                ),
        );
    }
}

// Represents the current playback status of a file.
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
    pub seek_to: Option<f32>,
    // Internal state for tracking playback time accurately.
    pub(crate) last_update: Option<Instant>,
    pub(crate) position_at_last_update: Duration,
}

impl PlaybackInfo {
    // Resets the state, typically when no file is loaded.
    pub fn reset(&mut self) {
        *self = PlaybackInfo::default();
    }
}

// Defines the selected source for audio input.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub enum AudioSource {
    File(PathBuf),
    #[default]
    None,
}

#[derive(Resource, Default)]
pub struct SelectedAudioSource(pub AudioSource);

#[derive(Resource, Default)]
pub struct SelectedMic(pub Option<String>);

#[derive(Resource, Default, Clone)]
pub struct AudioSamples(pub VecDeque<f32>);

// Holds metadata about the currently playing audio.
#[derive(Resource)]
pub struct AudioInfo {
    pub sample_rate: u32,
}

// Stores the results of the audio analysis.
#[derive(Resource, Default)]
pub struct AudioAnalysis {
    pub frequency_bins: Vec<f32>,
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
    pub treble_average: f32,
    pub volume: f32,
    pub flux: f32,
}

// This system manages the audio source. When the `SelectedAudioSource` resource
// changes (e.g., user picks a new file), it stops the current audio,
// clears the old state, and starts playing the new source.
pub fn manage_audio_playback(
    mut commands: Commands,
    selected_source: Res<SelectedAudioSource>,
    sink: NonSend<Sink>,
    analysis_sender: Res<AnalysisAudioSender>,
    mut audio_samples: ResMut<AudioSamples>,
    mut playback_info: ResMut<PlaybackInfo>,
) {
    if !selected_source.is_changed() {
        return;
    }

    // Stop all current audio and reset state
    sink.stop();
    audio_samples.0.clear();
    playback_info.reset();

    match &selected_source.0 {
        AudioSource::File(path) => {
            info!(
                "Audio source changed. Attempting to load file: {:?}",
                path
            );

            // Use Symphonia to get the duration, as it's the most reliable method.
            let duration = match get_duration_with_symphonia(path) {
                Ok(d) => {
                    info!("✅ Successfully read duration with Symphonia: {:?}", d);
                    d
                }
                Err(e) => {
                    error!("❌ Failed to get duration with Symphonia: {}. The progress bar will be incorrect.", e);
                    Duration::ZERO
                }
            };

            let file_bytes = std::fs::read(path).expect("Failed to read music file for playback");
            let cursor = Cursor::new(file_bytes);
            let source = Decoder::new(cursor).unwrap();

            commands.insert_resource(AudioInfo {
                sample_rate: source.sample_rate(),
            });

            // Initialize playback info for the new track.
            playback_info.duration = duration;
            playback_info.status = PlaybackStatus::Playing;
            playback_info.last_update = Some(Instant::now());
            playback_info.position_at_last_update = Duration::ZERO; // Playback starts at 0.

            // Use the Tee adapter to send audio data to the analysis channel while playing.
            let tee_source = AudioDataTee {
                source: source.convert_samples(),
                sender: analysis_sender.0.clone(),
            };

            sink.append(tee_source);
        }
        AudioSource::None => {
            info!("Stopping all audio");
            // State is already reset above.
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
                // When resuming, save the current position as the new base for time calculation.
                playback_info.last_update = Some(Instant::now());
                playback_info.position_at_last_update = playback_info.position;
            }
        }
        PlaybackStatus::Paused => {
            if !sink.is_paused() {
                sink.pause();
                // When pausing, calculate and save the precise position.
                if let Some(last_update) = playback_info.last_update.take() {
                    let elapsed = last_update.elapsed().as_secs_f32() * sink.speed();
                    playback_info.position =
                        playback_info.position_at_last_update + Duration::from_secs_f32(elapsed);
                }
                // Time is no longer elapsing, so clear the update instant.
                playback_info.last_update = None;
            }
        }
    }

    // Handle speed changes
    if sink.speed() != playback_info.speed {
        // Update position before changing speed to maintain accuracy.
        if !sink.is_paused() {
            if let Some(last_update) = playback_info.last_update.take() {
                let elapsed = last_update.elapsed().as_secs_f32() * sink.speed();
                playback_info.position =
                    playback_info.position_at_last_update + Duration::from_secs_f32(elapsed);
            }
            // Set a new starting point for calculation.
            playback_info.last_update = Some(Instant::now());
            playback_info.position_at_last_update = playback_info.position;
        }
        sink.set_speed(playback_info.speed);
    }

    // Handle seeking
    if let Some(seek_pos_secs) = playback_info.seek_to.take() {
        if let AudioSource::File(path) = &selected_source.0 {
            info!("Seeking to {} seconds", seek_pos_secs);
            let seek_duration = Duration::from_secs_f32(seek_pos_secs);

            // Recreate the audio source to seek to the new position.
            let file_bytes = std::fs::read(path).expect("Failed to read music file for seeking");
            let cursor = Cursor::new(file_bytes);
            let source = Decoder::new(cursor).unwrap();

            let new_source = source.skip_duration(seek_duration).convert_samples();

            let tee_source = AudioDataTee {
                source: new_source,
                sender: analysis_sender.0.clone(),
            };

            // Replace the sink's content with the new, seeked source.
            sink.stop();
            sink.clear();
            sink.append(tee_source);

            // Update the position and the starting point for time calculation.
            playback_info.position = seek_duration;
            playback_info.position_at_last_update = seek_duration;

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
fn update_playback_position(mut playback_info: ResMut<PlaybackInfo>, sink: NonSend<Sink>) {
    if playback_info.status == PlaybackStatus::Playing {
        // The new position is calculated from a stable starting point to avoid drift.
        if let Some(last_update) = playback_info.last_update {
            let elapsed_since_update = last_update.elapsed().as_secs_f32() * sink.speed();
            let new_pos = playback_info.position_at_last_update
                + Duration::from_secs_f32(elapsed_since_update);

            if new_pos >= playback_info.duration && playback_info.duration != Duration::ZERO {
                // Playback has finished.
                playback_info.position = playback_info.duration;
                playback_info.status = PlaybackStatus::Paused;
                playback_info.last_update = None;
            } else {
                // Update the position for display purposes.
                playback_info.position = new_pos;
            }
        }
    }
}

// Reads raw audio samples from the analysis channel into a buffer.
pub fn read_analysis_data_system(
    receiver: Option<NonSend<AnalysisAudioReceiver>>,
    mut buffer: ResMut<AudioSamples>,
) {
    if let Some(receiver) = receiver {
        buffer.0.extend(receiver.0.try_iter());
    }
}

// Performs the Fast Fourier Transform (FFT) on the buffered audio samples
// to get frequency data for the visualizations.
pub fn audio_analysis_system(
    time: Res<Time>,
    mut analysis_timer: ResMut<AnalysisTimer>,
    mut audio_analysis: ResMut<AudioAnalysis>,
    audio_info: Option<Res<AudioInfo>>,
    audio_source: Res<SelectedAudioSource>,
    mut audio_samples: ResMut<AudioSamples>,
    config: Res<VisualsConfig>,
) {
    analysis_timer.0.tick(time.delta());
    if !analysis_timer.0.just_finished() {
        return;
    }

    let Some(audio_info) = audio_info else { return };
    let fft_size = 4096;

    // Determine the source of the audio samples.
    let analysis_buffer: Option<Vec<f32>> = if let AudioSource::File(_) = &audio_source.0 {
        if audio_samples.0.len() < fft_size {
            None
        } else {
            let buffer_len = audio_samples.0.len();
            let analysis_vec = audio_samples.0.iter().copied().take(fft_size).collect();
            // Drain half the FFT size to create overlapping windows, which smooths the analysis.
            let drain_amount = buffer_len.saturating_sub(fft_size / 2);
            audio_samples.0.drain(..drain_amount);
            Some(analysis_vec)
        }
    } else {
        None
    };

    let Some(samples_slice) = analysis_buffer else { return };

    // Apply a Hann window to the samples to reduce spectral leakage.
    let hann_window = hann_window(&samples_slice);

    // Compute the spectrum from the windowed samples.
    let spectrum = samples_fft_to_spectrum(
        &hann_window,
        audio_info.sample_rate,
        FrequencyLimit::Range(20.0, 20000.0),
        Some(&divide_by_N_sqrt),
    )
    .expect("Failed to compute spectrum");

    // Calculate the overall volume (RMS) of the current audio frame.
    let squared_sum = samples_slice.iter().map(|s| s * s).sum::<f32>();
    audio_analysis.volume = (squared_sum / samples_slice.len() as f32).sqrt();

    let spectrum_data: Vec<(f32, f32)> =
        spectrum.data().iter().map(|(f, v)| (f.val(), v.val())).collect();

    // Calculate spectral flux: the rate of change in the spectrum.
    // This can be used to detect transients or changes in the music.
    if !audio_analysis.previous_spectrum.is_empty()
        && audio_analysis.previous_spectrum.len() == spectrum_data.len()
    {
        let sum_of_squared_diffs = spectrum_data
            .iter()
            .zip(&audio_analysis.previous_spectrum)
            .map(|((_, cur_mag), (_, prev_mag))| (cur_mag - prev_mag).powi(2))
            .sum::<f32>();
        audio_analysis.flux = sum_of_squared_diffs.sqrt();
    } else {
        audio_analysis.flux = 0.0;
    }

    // Group the raw FFT results into a smaller number of frequency bands for visualization.
    let num_bands = config.num_bands;
    let mut new_bins = vec![0.0; num_bands];
    let min_freq = 20.0f32;
    let max_freq = 20000.0f32;
    // Use a logarithmic scale for frequency bands, which better matches human hearing.
    let band_limits: Vec<f32> = (0..num_bands)
        .map(|i| min_freq * (max_freq / min_freq).powf((i as f32 + 1.0) / num_bands as f32))
        .collect();

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

    // Apply smoothing to the frequency bins to make the visualization less jittery.
    let smoothing = 0.5;
    if audio_analysis.frequency_bins.len() != num_bands {
        audio_analysis.frequency_bins.resize(num_bands, 0.0);
    }

    for i in 0..num_bands {
        audio_analysis.frequency_bins[i] =
            audio_analysis.frequency_bins[i] * smoothing + new_bins[i] * (1.0 - smoothing);
    }
    audio_analysis.treble_average =
        audio_analysis.treble_average * smoothing + treble_val * (1.0 - smoothing);

    // Calculate simplified bass, mid, and treble values for easier use in visualizations.
    audio_analysis.bass = audio_analysis.frequency_bins.iter().take(num_bands / 4).sum();
    audio_analysis.mid = audio_analysis
        .frequency_bins
        .iter()
        .skip(num_bands / 4)
        .take(num_bands / 2)
        .sum();
    audio_analysis.treble = audio_analysis.frequency_bins.iter().skip(3 * num_bands / 4).sum();
}