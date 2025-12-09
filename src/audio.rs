// src/audio.rs

use crate::{AppState, VisualizationEnabled, config::VisualsConfig};
use bevy::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rodio::{Decoder, Sink, source::Source};
use spectrum_analyzer::{
    FrequencyLimit, samples_fft_to_spectrum, scaling::divide_by_N_sqrt, windows::hann_window,
};
use std::collections::VecDeque;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

// --- Symphonia Helper ---

/// Uses the Symphonia library to reliably get the duration of an audio file.
/// This method is particularly accurate for formats with variable bitrates (VBR) like some MP3s.
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

/// A "tee" adapter for audio data. While the audio plays through the `rodio` sink,
/// this struct sends a copy of each raw audio sample to a channel for real-time analysis.
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
        // Send a copy of the sample for analysis, ignoring any errors if the receiver is dropped.
        self.sender.send(sample).ok();
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

/// The main Bevy plugin for handling all audio-related logic,
/// including playback, microphone input, and analysis.
pub struct AudioPlugin;

#[derive(Resource)]
pub struct AnalysisTimer(pub Timer);

/// A channel sender for passing raw audio samples to the analysis system.
#[derive(Resource, Clone)]
pub struct AnalysisAudioSender(pub Sender<f32>);
/// A channel receiver for raw audio samples.
pub struct AnalysisAudioReceiver(pub Receiver<f32>);

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        let (mic_tx, mic_rx) = std::sync::mpsc::channel::<Vec<f32>>();
        let (analysis_tx, analysis_rx) = std::sync::mpsc::channel::<f32>();

        app.insert_resource(AnalysisTimer(Timer::new(
            Duration::from_secs_f32(1.0 / 60.0), // Aim for 60 analysis updates per second.
            TimerMode::Repeating,
        )))
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
                .run_if(
                    in_state(AppState::Visualization2D)
                        .or_else(in_state(AppState::Visualization3D))
                        .or_else(in_state(AppState::VisualizationOrb))
                        .or_else(in_state(AppState::VisualizationDisc)),
                ),
        );
    }
}

/// Represents the current playback status of a file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PlaybackStatus {
    #[default]
    Paused,
    Playing,
}

/// A resource to hold all information related to audio playback control.
#[derive(Resource, Debug, Default)]
pub struct PlaybackInfo {
    pub status: PlaybackStatus,
    pub speed: f32,
    pub position: Duration,
    pub duration: Duration,
    /// A signal from the UI to seek to a new position (in seconds).
    pub seek_to: Option<f32>,
    /// Internal state for accurately tracking playback time.
    pub(crate) last_update: Option<Instant>,
    /// The playback position at the moment of `last_update`. Used as a stable base for calculations.
    pub(crate) position_at_last_update: Duration,
}

impl PlaybackInfo {
    /// Resets the state, typically when no file is loaded or the source changes.
    pub fn reset(&mut self) {
        self.status = PlaybackStatus::Paused;
        self.speed = 1.0;
        self.position = Duration::ZERO;
        self.duration = Duration::ZERO;
        self.seek_to = None;
        self.last_update = None;
        self.position_at_last_update = Duration::ZERO;
    }
}

/// Defines the selected source for audio input.
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

/// A non-send resource holding the microphone input stream.
#[allow(dead_code)]
pub struct MicStream(pub Option<cpal::Stream>);

/// A buffer for incoming microphone audio data.
#[derive(Resource, Default)]
pub struct MicAudioBuffer(pub VecDeque<f32>);

/// A buffer for audio data from files, ready for analysis.
#[derive(Resource, Default, Clone)]
pub struct AudioSamples(pub VecDeque<f32>);

/// Holds metadata about the currently playing audio.
#[derive(Resource)]
pub struct AudioInfo {
    pub sample_rate: u32,
}

/// Stores the results of the audio analysis, to be used by the visualizations.
#[derive(Resource, Default)]
pub struct AudioAnalysis {
    pub frequency_bins: Vec<f32>,
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
    pub treble_average: f32,
    pub volume: f32,
    pub flux: f32,
    /// Holds the spectrum data from the previous frame to calculate spectral flux.
    pub previous_spectrum: Vec<(f32, f32)>,
}

/// Manages the audio source. When the `SelectedAudioSource` resource
/// changes, it stops the current audio, clears old state, and starts the new source.
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
) {
    if !selected_source.is_changed() {
        return;
    }

    // Stop all current audio and reset state before starting a new source.
    sink.stop();
    *mic_stream = MicStream(None);
    audio_samples.0.clear();
    playback_info.reset();

    match &selected_source.0 {
        AudioSource::File(path) => {
            info!("Audio source changed. Attempting to load file: {:?}", path);

            let duration = match get_duration_with_symphonia(path) {
                Ok(d) => {
                    info!("✅ Successfully read duration with Symphonia: {:?}", d);
                    d
                }
                Err(e) => {
                    error!(
                        "❌ Failed to get duration with Symphonia: {}. The progress bar will be incorrect.",
                        e
                    );
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
        AudioSource::Microphone => {
            info!("Starting microphone capture");
            let host = cpal::default_host();
            let device = selected_mic
                .0
                .as_ref()
                .and_then(|name| {
                    host.input_devices()
                        .ok()?
                        .find(|d| d.name().unwrap_or_default() == *name)
                })
                .unwrap_or_else(|| {
                    host.default_input_device()
                        .expect("No default audio input device found")
                });
            let config = device
                .default_input_config()
                .expect("Failed to get default input config");
            info!(
                "Initializing microphone: {} with config {:?}",
                device.name().unwrap(),
                config
            );
            commands.insert_resource(AudioInfo {
                sample_rate: config.sample_rate().0,
            });
            let tx = mic_sender.0.clone();
            let stream = device
                .build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        tx.send(data.to_vec()).ok();
                    },
                    |err| error!("An error occurred on the audio stream: {}", err),
                    None,
                )
                .expect("Failed to build input stream");
            stream.play().expect("Failed to play audio stream");
            *mic_stream = MicStream(Some(stream));
        }
        AudioSource::None => {
            info!("Stopping all audio");
            // State is already reset above.
        }
    }
}

/// Applies changes from the UI (play, pause, speed, seek) to the `rodio` audio sink.
fn apply_playback_changes(
    mut playback_info: ResMut<PlaybackInfo>,
    sink: NonSend<Sink>,
    selected_source: Res<SelectedAudioSource>,
    analysis_sender: Res<AnalysisAudioSender>,
) {
    if !playback_info.is_changed() {
        return;
    }

    // Handle Play/Pause state changes.
    match playback_info.status {
        PlaybackStatus::Playing => {
            if sink.is_paused() {
                sink.play();
                // When resuming, record the current time and position to serve as a new, stable
                // base for calculating the progress bar position. This prevents drift.
                playback_info.last_update = Some(Instant::now());
                playback_info.position_at_last_update = playback_info.position;
            }
        }
        PlaybackStatus::Paused => {
            if !sink.is_paused() {
                sink.pause();
                // When pausing, calculate and save the precise position up to this moment.
                if let Some(last_update) = playback_info.last_update.take() {
                    let elapsed = last_update.elapsed().as_secs_f32() * sink.speed();
                    playback_info.position =
                        playback_info.position_at_last_update + Duration::from_secs_f32(elapsed);
                }
                // Clear `last_update` as time is no longer elapsing.
                playback_info.last_update = None;
            }
        }
    }

    // Handle speed changes.
    if sink.speed() != playback_info.speed {
        // Before changing speed, update the position to the current moment to maintain accuracy.
        if !sink.is_paused() {
            if let Some(last_update) = playback_info.last_update.take() {
                let elapsed = last_update.elapsed().as_secs_f32() * sink.speed();
                playback_info.position =
                    playback_info.position_at_last_update + Duration::from_secs_f32(elapsed);
            }
            // After updating, set a new starting point for future calculations.
            playback_info.last_update = Some(Instant::now());
            playback_info.position_at_last_update = playback_info.position;
        }
        sink.set_speed(playback_info.speed);
    }

    // Handle seeking requested by the UI.
    if let Some(seek_pos_secs) = playback_info.seek_to.take() {
        if let AudioSource::File(path) = &selected_source.0 {
            info!("Seeking to {} seconds", seek_pos_secs);
            let seek_duration = Duration::from_secs_f32(seek_pos_secs);

            // To seek, the entire audio source must be recreated and replaced in the sink.
            let file_bytes = std::fs::read(path).expect("Failed to read music file for seeking");
            let cursor = Cursor::new(file_bytes);
            let source = Decoder::new(cursor).unwrap();

            // Create a new source that skips to the desired duration.
            let new_source = source.skip_duration(seek_duration).convert_samples();

            let tee_source = AudioDataTee {
                source: new_source,
                sender: analysis_sender.0.clone(),
            };

            // Replace the sink's content with the new, seeked source.
            sink.stop();
            sink.clear();
            sink.append(tee_source);

            // Update our internal tracking to reflect the new position.
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

/// Continuously updates the playback position for the UI progress bar.
fn update_playback_position(mut playback_info: ResMut<PlaybackInfo>, sink: NonSend<Sink>) {
    if playback_info.status == PlaybackStatus::Playing {
        // The new position is calculated from a stable starting point (`position_at_last_update`)
        // plus the time elapsed since then. This is more robust against floating-point drift
        // than simply adding delta time each frame.
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

/// Reads raw audio samples from the analysis channel into a buffer.
pub fn read_analysis_data_system(
    receiver: Option<NonSend<AnalysisAudioReceiver>>,
    mut buffer: ResMut<AudioSamples>,
) {
    if let Some(receiver) = receiver {
        buffer.0.extend(receiver.0.try_iter());
    }
}

/// Reads raw audio data from the microphone channel into a buffer.
pub fn read_mic_data_system(
    receiver: Option<NonSend<MicAudioReceiver>>,
    mut buffer: ResMut<MicAudioBuffer>,
) {
    if let Some(receiver) = receiver {
        for new_data in receiver.0.try_iter() {
            buffer.0.extend(new_data);
        }
    }
}

/// Performs the Fast Fourier Transform (FFT) on buffered audio samples
/// to get frequency data for the visualizations.
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

    // Determine the source of the audio samples (file or microphone).
    let analysis_buffer: Option<Vec<f32>> = match &audio_source.0 {
        AudioSource::File(_) => {
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
        }
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

    let Some(samples_slice) = analysis_buffer else {
        return;
    };

    // Apply a Hann window to the samples to reduce spectral leakage, which is an
    // artifact of FFT on non-periodic signals.
    let hann_window = hann_window(&samples_slice);

    // Compute the spectrum from the windowed samples.
    let spectrum = samples_fft_to_spectrum(
        &hann_window,
        audio_info.sample_rate,
        FrequencyLimit::Range(20.0, 20000.0), // Human hearing range
        Some(&divide_by_N_sqrt),              // Scaling function
    )
    .expect("Failed to compute spectrum");

    // Calculate the overall volume (RMS) of the current audio frame.
    let squared_sum = samples_slice.iter().map(|s| s * s).sum::<f32>();
    audio_analysis.volume = (squared_sum / samples_slice.len() as f32).sqrt();

    let spectrum_data: Vec<(f32, f32)> = spectrum
        .data()
        .iter()
        .map(|(f, v)| (f.val(), v.val()))
        .collect();

    // Calculate spectral flux: the rate of change in the spectrum's magnitude.
    // This can be used to detect transients or "beat" changes in the music.
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
    audio_analysis.bass = audio_analysis
        .frequency_bins
        .iter()
        .take(num_bands / 4)
        .sum();
    audio_analysis.mid = audio_analysis
        .frequency_bins
        .iter()
        .skip(num_bands / 4)
        .take(num_bands / 2)
        .sum();
    audio_analysis.treble = audio_analysis
        .frequency_bins
        .iter()
        .skip(3 * num_bands / 4)
        .sum();

    // Store the current spectrum for the next frame's flux calculation.
    audio_analysis.previous_spectrum = spectrum_data;
}
