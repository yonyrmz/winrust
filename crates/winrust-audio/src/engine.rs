//! The playback engine: a `cpal` output stream fed by a decoder thread.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use parking_lot::Mutex;
use winrust_core::{PlaybackState, PlayerEvent, Track};
use winrust_dsp::Equalizer;

use crate::decoder::{DecodeError, DecoderStream};
use crate::resampler::SampleRateConverter;
use crate::ring_buffer::RingBuffer;
use crate::source::MediaSource;

/// Builds a `cpal` output stream for the given sample type.
///
/// The macro is used instead of a generic function because the sample
/// conversion traits are only implemented for the concrete sample types.
macro_rules! build_stream {
    ($device:expr, $config:expr, $shared:expr, $sample:ty) => {{
        let callback_shared: Arc<Shared> = Arc::clone(&$shared);

        $device
            .build_output_stream(
                $config,
                move |output: &mut [$sample], _: &cpal::OutputCallbackInfo| {
                    let paused = callback_shared.paused.load(Ordering::Relaxed);
                    if paused {
                        for value in output.iter_mut() {
                            *value = cpal::Sample::EQUILIBRIUM;
                        }
                        return;
                    }

                    let mut staging: [f32; 4096] = [0.0; 4096];
                    let count = output.len().min(staging.len());
                    let read = callback_shared.ring.pop(&mut staging[..count]);
                    for (index, value) in output.iter_mut().enumerate() {
                        let sample = if index < read { staging[index] } else { 0.0 };
                        *value = cpal::Sample::from_sample(sample);
                    }

                    // Feed the visualizer tap with a mono downmix.
                    if read > 0 {
                        let channels = callback_shared.output_channels.max(1);
                        let mut tap = callback_shared.tap.lock();
                        for frame in staging[..read].chunks(channels) {
                            if let Some(sample) = frame.first() {
                                if tap.len() >= TAP_CAPACITY {
                                    tap.pop_front();
                                }
                                tap.push_back(*sample);
                            }
                        }
                    }

                    callback_shared
                        .consumed_frames
                        .fetch_add(read as u64, Ordering::Relaxed);

                    let params = *callback_shared.params.lock();
                    let gain = if params.muted { 0.0 } else { params.volume };
                    if (gain - 1.0).abs() > f32::EPSILON {
                        for value in output.iter_mut() {
                            let scaled = cpal::Sample::to_sample::<f32>(*value) * gain;
                            *value = cpal::Sample::from_sample(scaled);
                        }
                    }
                },
                move |err| {
                    tracing::error!("audio stream error: {err}");
                },
                None,
            )
            .map_err(|err| AudioError::Stream(err.to_string()))?
    }};
}

/// Errors produced by the audio engine.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// No output device was available.
    #[error("no audio output device available")]
    NoDevice,
    /// The output device rejected the requested configuration.
    #[error("could not configure the audio output: {0}")]
    Configuration(String),
    /// The output stream could not be created.
    #[error("could not create the audio output stream: {0}")]
    Stream(String),
    /// The media could not be decoded.
    #[error(transparent)]
    Decode(#[from] DecodeError),
}

/// Parameters that the UI can change at any time.
#[derive(Debug, Clone, Copy)]
struct Params {
    volume: f32,
    muted: bool,
}

/// Live playback status maintained by the engine.
#[derive(Debug, Default)]
struct Status {
    state: PlaybackState,
    duration: f64,
    track: Option<Track>,
}

/// State shared between the UI thread, the engine thread and the audio callback.
struct Shared {
    ring: RingBuffer,
    params: Mutex<Params>,
    status: Mutex<Status>,
    paused: AtomicBool,
    /// Frames consumed by the output callback, used for the position readout.
    consumed_frames: AtomicU64,
    output_rate: u32,
    output_channels: usize,
    /// The equalizer applied to the decoded signal.
    equalizer: Mutex<Equalizer>,
    /// The stereo balance in the `-1.0..=1.0` range.
    balance: Mutex<f32>,
    /// A short tap of the most recent samples, used by the visualizer.
    tap: Mutex<VecDeque<f32>>,
}

/// The maximum number of samples kept in the visualizer tap.
const TAP_CAPACITY: usize = 8 * 1024;

impl Shared {
    fn position_seconds(&self) -> f64 {
        let rate = self.output_rate.max(1) as f64;
        let channels = self.output_channels.max(1) as f64;
        self.consumed_frames.load(Ordering::Relaxed) as f64 / (rate * channels)
    }

    fn reset_position(&self, seconds: f64) {
        let rate = self.output_rate.max(1) as f64;
        let channels = self.output_channels.max(1) as f64;
        let frames = (seconds.max(0.0) * rate * channels) as u64;
        self.consumed_frames.store(frames, Ordering::Relaxed);
    }
}

/// Commands accepted by the engine thread.
#[allow(clippy::large_enum_variant)]
enum Command {
    Play {
        track: Track,
        source: MediaSource,
        start_at: f64,
    },
    Pause,
    Resume,
    Stop,
    Seek(f64),
    SetVolume(f32),
    SetMuted(bool),
    Shutdown,
    /// The decoder of a track finished.
    ///
    /// The generation identifies the decoder, so that a track that was
    /// replaced or seeked away does not advance the queue.
    Finished(u64),
}

/// The winrust audio engine.
///
/// The engine owns a `cpal` output stream. A background thread decodes the
/// media and pushes PCM frames into a ring buffer that the audio callback
/// drains.
pub struct AudioEngine {
    shared: Arc<Shared>,
    commands: Sender<Command>,
    stream: Option<cpal::Stream>,
    engine_thread: Option<JoinHandle<()>>,
    events: mpsc::Sender<PlayerEvent>,
}

impl AudioEngine {
    /// Creates the engine and starts the output stream.
    pub fn new(events: Sender<PlayerEvent>) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(AudioError::NoDevice)?;

        let supported = device
            .default_output_config()
            .map_err(|err| AudioError::Configuration(err.to_string()))?;

        let sample_rate = supported.sample_rate();
        let channels = supported.channels() as usize;

        let shared = Arc::new(Shared {
            ring: RingBuffer::new(sample_rate as usize * channels * 8),
            params: Mutex::new(Params {
                volume: 1.0,
                muted: false,
            }),
            status: Mutex::new(Status::default()),
            paused: AtomicBool::new(false),
            consumed_frames: AtomicU64::new(0),
            output_rate: sample_rate,
            output_channels: channels,
            equalizer: Mutex::new(Equalizer::new(sample_rate as f32)),
            balance: Mutex::new(0.0),
            tap: Mutex::new(VecDeque::with_capacity(TAP_CAPACITY)),
        });

        let (commands, command_rx) = mpsc::channel();

        let format = supported.sample_format();
        let config = supported.into();
        let stream = match format {
            SampleFormat::F32 => build_stream!(device, config, shared, f32),
            SampleFormat::I16 => build_stream!(device, config, shared, i16),
            SampleFormat::U16 => build_stream!(device, config, shared, u16),
            SampleFormat::I32 => build_stream!(device, config, shared, i32),
            other => {
                return Err(AudioError::Configuration(format!(
                    "unsupported sample format: {other}"
                )))
            }
        };
        stream
            .play()
            .map_err(|err| AudioError::Stream(err.to_string()))?;

        let engine_shared = Arc::clone(&shared);
        let engine_events = events.clone();
        let engine_tx = commands.clone();
        let engine_thread = std::thread::Builder::new()
            .name("winrust-engine".to_string())
            .spawn(move || engine_loop(command_rx, engine_tx, engine_shared, engine_events))
            .ok();

        Ok(Self {
            shared,
            commands,
            stream: Some(stream),
            engine_thread,
            events,
        })
    }

    /// Sample rate of the output device.
    pub fn output_sample_rate(&self) -> u32 {
        self.shared.output_rate
    }

    /// Number of channels of the output device.
    pub fn output_channels(&self) -> usize {
        self.shared.output_channels
    }

    /// Current playback position in seconds.
    pub fn position(&self) -> f64 {
        self.shared.position_seconds()
    }

    /// Duration of the current track in seconds.
    pub fn duration(&self) -> f64 {
        self.shared.status.lock().duration
    }

    /// Current playback state.
    pub fn state(&self) -> PlaybackState {
        self.shared.status.lock().state
    }

    /// A clone of the event sender, useful to forward player events.
    pub fn event_sender(&self) -> mpsc::Sender<PlayerEvent> {
        self.events.clone()
    }

    /// Starts playing the given track.
    pub fn play(&self, track: Track, source: MediaSource) {
        self.send(Command::Play {
            track,
            source,
            start_at: 0.0,
        });
    }

    /// Starts playing the given track from a given position.
    pub fn play_from(&self, track: Track, source: MediaSource, start_at: f64) {
        self.send(Command::Play {
            track,
            source,
            start_at,
        });
    }

    /// Pauses playback.
    pub fn pause(&self) {
        self.send(Command::Pause);
    }

    /// Resumes playback.
    pub fn resume(&self) {
        self.send(Command::Resume);
    }

    /// Stops playback and unloads the current track.
    pub fn stop(&self) {
        self.send(Command::Stop);
    }

    /// Seeks to the given position in seconds.
    pub fn seek(&self, seconds: f64) {
        self.send(Command::Seek(seconds));
    }

    /// Sets the master volume in the `0.0..=1.0` range.
    pub fn set_volume(&self, volume: f32) {
        {
            let mut params = self.shared.params.lock();
            params.volume = volume.clamp(0.0, 1.0);
        }
        self.send(Command::SetVolume(volume));
    }

    /// Sets the mute state.
    pub fn set_muted(&self, muted: bool) {
        {
            let mut params = self.shared.params.lock();
            params.muted = muted;
        }
        self.send(Command::SetMuted(muted));
    }

    /// Sets the gains of the ten equalizer bands, in dB.
    pub fn set_equalizer_bands(&self, gains: [f32; winrust_dsp::BAND_COUNT]) {
        self.shared.equalizer.lock().set_band_gains(gains);
    }

    /// Sets the preamp of the equalizer, in dB.
    pub fn set_equalizer_preamp(&self, gain_db: f32) {
        self.shared.equalizer.lock().set_preamp_db(gain_db);
    }

    /// Enables or bypasses the equalizer.
    pub fn set_equalizer_enabled(&self, enabled: bool) {
        self.shared.equalizer.lock().set_enabled(enabled);
    }

    /// Sets the stereo balance in the `-1.0..=1.0` range.
    pub fn set_balance(&self, balance: f32) {
        *self.shared.balance.lock() = balance.clamp(-1.0, 1.0);
    }

    /// Drains the visualizer tap, returning the most recent samples.
    pub fn drain_tap(&self) -> Vec<f32> {
        let mut tap = self.shared.tap.lock();
        tap.drain(..).collect()
    }

    /// Sends a command to the engine thread.
    fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.send(Command::Shutdown);
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
        }
        if let Some(handle) = self.engine_thread.take() {
            let _ = handle.join();
        }
    }
}

/// The engine thread loop.
fn engine_loop(
    commands: Receiver<Command>,
    command_tx: Sender<Command>,
    shared: Arc<Shared>,
    events: Sender<PlayerEvent>,
) {
    let mut decoder: Option<DecoderHandle> = None;
    let mut current: Option<(Track, MediaSource)> = None;
    let mut generation: u64 = 0;

    while let Ok(command) = commands.recv() {
        match command {
            Command::Play {
                track,
                source,
                start_at,
            } => {
                if let Some(handle) = decoder.take() {
                    handle.stop();
                }
                shared.ring.clear();
                shared.reset_position(start_at);
                {
                    let mut status = shared.status.lock();
                    status.state = PlaybackState::Playing;
                    status.track = Some(track.clone());
                    status.duration = 0.0;
                }
                shared.paused.store(false, Ordering::Relaxed);
                let _ = events.send(PlayerEvent::StateChanged(PlaybackState::Playing));
                let _ = events.send(PlayerEvent::TrackStarted(track.clone()));

                generation = generation.wrapping_add(1);
                let handle = spawn_decoder(
                    Arc::clone(&shared),
                    events.clone(),
                    command_tx.clone(),
                    generation,
                    source.clone(),
                    start_at,
                );
                decoder = Some(handle);
                current = Some((track, source));
            }
            Command::Pause => {
                shared.paused.store(true, Ordering::Relaxed);
                shared.status.lock().state = PlaybackState::Paused;
                let _ = events.send(PlayerEvent::StateChanged(PlaybackState::Paused));
            }
            Command::Resume => {
                shared.paused.store(false, Ordering::Relaxed);
                shared.status.lock().state = PlaybackState::Playing;
                let _ = events.send(PlayerEvent::StateChanged(PlaybackState::Playing));
            }
            Command::Stop => {
                if let Some(handle) = decoder.take() {
                    handle.stop();
                }
                current = None;
                shared.ring.clear();
                shared.reset_position(0.0);
                let mut status = shared.status.lock();
                status.state = PlaybackState::Stopped;
                status.track = None;
                status.duration = 0.0;
                let _ = events.send(PlayerEvent::StateChanged(PlaybackState::Stopped));
            }
            Command::Seek(seconds) => {
                let Some((_, source)) = current.clone() else {
                    continue;
                };
                if let Some(handle) = decoder.take() {
                    handle.stop();
                }
                shared.ring.clear();
                shared.reset_position(seconds);
                generation = generation.wrapping_add(1);
                decoder = Some(spawn_decoder(
                    Arc::clone(&shared),
                    events.clone(),
                    command_tx.clone(),
                    generation,
                    source,
                    seconds,
                ));
            }
            Command::SetVolume(volume) => {
                shared.params.lock().volume = volume.clamp(0.0, 1.0);
            }
            Command::SetMuted(muted) => {
                shared.params.lock().muted = muted;
            }
            Command::Finished(decoder_generation) => {
                // Only the decoder of the current track may advance the queue.
                if decoder_generation == generation {
                    let _ = events.send(PlayerEvent::TrackEnded);
                }
            }
            Command::Shutdown => {
                if let Some(handle) = decoder.take() {
                    handle.stop();
                }
                break;
            }
        }
    }

    if let Some(handle) = decoder.take() {
        handle.stop();
    }
}

/// Handle to a running decoder thread.
struct DecoderHandle {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl DecoderHandle {
    fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Drop for DecoderHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Spawns the thread that decodes and resamples a media source.
fn spawn_decoder(
    shared: Arc<Shared>,
    events: Sender<PlayerEvent>,
    command_tx: Sender<Command>,
    generation: u64,
    source: MediaSource,
    start_at: f64,
) -> DecoderHandle {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::clone(&stop);

    let thread = std::thread::Builder::new()
        .name("winrust-decoder".to_string())
        .spawn(move || {
            if let Err(err) = decode_loop(&shared, &source, start_at, &stop_flag) {
                tracing::error!("decoder failed: {err}");
                let _ = events.send(PlayerEvent::Error(err.to_string()));
            }
            // The engine thread decides whether the track really ended.
            let _ = command_tx.send(Command::Finished(generation));
        })
        .ok();

    DecoderHandle { stop, thread }
}

/// The decoding loop.
fn decode_loop(
    shared: &Arc<Shared>,
    source: &MediaSource,
    start_at: f64,
    stop: &AtomicBool,
) -> Result<(), DecodeError> {
    let mut decoder = DecoderStream::open(source)?;

    let in_rate = decoder.stream_format.sample_rate;
    let channels = decoder.stream_format.channels as usize;
    let mut converter =
        SampleRateConverter::new(in_rate, shared.output_rate, channels).map_err(|err| {
            DecodeError::Symphonia(symphonia::core::errors::Error::IoError(
                std::io::Error::other(err),
            ))
        })?;

    if let Some(duration) = decoder.duration() {
        shared.status.lock().duration = duration;
    }

    // Skip ahead when the user seeks into the middle of a track.
    let skip_frames = (start_at * in_rate as f64) as u64;
    let mut skipped = 0u64;

    let mut buffer: Vec<f32> = Vec::with_capacity(64 * 1024);

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        let Some(samples) = decoder.decode_next()? else {
            break;
        };

        if samples.is_empty() {
            continue;
        }

        let mut decoded = samples.as_slice();

        if skipped < skip_frames {
            let skip_samples = ((skip_frames - skipped) as usize) * channels;
            if skip_samples < decoded.len() {
                skipped += (skip_samples / channels) as u64;
                decoded = &decoded[skip_samples..];
            } else {
                skipped += (decoded.len() / channels) as u64;
                continue;
            }
        }

        buffer.clear();
        converter.process(decoded, &mut buffer).map_err(|err| {
            DecodeError::Symphonia(symphonia::core::errors::Error::IoError(
                std::io::Error::other(err),
            ))
        })?;

        if !buffer.is_empty() {
            apply_dsp(shared, &mut buffer);
            shared.ring.push(&buffer);
        }
    }

    // Flush the tail of the resampler.
    buffer.clear();
    converter.flush(&mut buffer).map_err(|err| {
        DecodeError::Symphonia(symphonia::core::errors::Error::IoError(
            std::io::Error::other(err),
        ))
    })?;
    if !buffer.is_empty() {
        apply_dsp(shared, &mut buffer);
        shared.ring.push(&buffer);
    }

    Ok(())
}

/// Applies the equalizer and the stereo balance to a block of samples.
fn apply_dsp(shared: &Arc<Shared>, buffer: &mut [f32]) {
    let channels = shared.output_channels.max(1);
    if channels < 2 {
        return;
    }

    {
        let mut equalizer = shared.equalizer.lock();
        equalizer.process_interleaved(buffer);
    }

    let balance = *shared.balance.lock();
    if balance.abs() < f32::EPSILON {
        return;
    }

    let (left_gain, right_gain) = if balance < 0.0 {
        (1.0 + balance, 1.0)
    } else {
        (1.0, 1.0 - balance)
    };

    for frame in buffer.chunks_mut(channels) {
        if let Some(left) = frame.first_mut() {
            *left *= left_gain;
        }
        if let Some(right) = frame.get_mut(1) {
            *right *= right_gain;
        }
    }
}
