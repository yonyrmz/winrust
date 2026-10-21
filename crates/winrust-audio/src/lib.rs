//! Audio engine for winrust.
//!
//! The crate wires together three pieces:
//!
//! * [`source`] describes where the audio bytes come from (local files or
//!   HTTP streams resolved through `yt-dlp`).
//! * [`decoder`] turns those bytes into PCM using `symphonia`.
//! * [`engine`] plays the PCM through `cpal`, resampling on the fly when the
//!   device rate differs from the media rate.

pub mod decoder;
pub mod engine;
pub mod resampler;
pub mod ring_buffer;
pub mod source;

pub use decoder::{DecodeError, DecoderStream, StreamFormat};
pub use engine::{AudioEngine, AudioError};
pub use resampler::SampleRateConverter;
pub use ring_buffer::RingBuffer;
pub use source::{MediaSource, SourceError};
