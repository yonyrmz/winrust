//! Symphonia based decoding of the audio formats winrust supports.

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::{MediaSourceStream, ReadOnlySource};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::source::{MediaSource, SourceError};

/// Errors raised while decoding media.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    /// The media could not be read.
    #[error("media error: {0}")]
    Source(#[from] SourceError),
    /// No decodable track was found in the media.
    #[error("no playable audio track found")]
    NoTrack,
    /// The decoder failed.
    #[error("decode error: {0}")]
    Symphonia(#[from] SymphoniaError),
}

/// Description of the decoded stream format.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreamFormat {
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of channels.
    pub channels: u16,
}

/// A symphonia decoder bound to a single track.
pub struct DecoderStream {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    source: MediaSource,
    /// Format of the decoded samples.
    pub stream_format: StreamFormat,
    /// Total duration in seconds, when the container declares one.
    pub duration: Option<f64>,
}

impl DecoderStream {
    /// Opens the given source and prepares it for decoding.
    pub fn open(source: &MediaSource) -> Result<Self, DecodeError> {
        let reader = source.open_reader()?;

        let mut hint = Hint::new();
        if let Some(ext) = source.extension() {
            hint.with_extension(&ext);
        }

        let mss = MediaSourceStream::new(Box::new(ReadOnlySource::new(reader)), Default::default());

        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(DecodeError::Symphonia)?;

        let format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or(DecodeError::NoTrack)?;

        let track_id = track.id;
        let stream_format = StreamFormat {
            sample_rate: track.codec_params.sample_rate.unwrap_or(44_100),
            channels: track
                .codec_params
                .channels
                .map(|channels| channels.count() as u16)
                .unwrap_or(2),
        };

        let duration = match (track.codec_params.time_base, track.codec_params.n_frames) {
            (Some(time_base), Some(frames)) => {
                let time = time_base.calc_time(frames);
                Some(time.seconds as f64 + time.frac)
            }
            _ => None,
        };

        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())?;

        Ok(Self {
            format,
            decoder,
            track_id,
            source: source.clone(),
            stream_format,
            duration,
        })
    }

    /// Total duration of the track in seconds, when the container declares one.
    pub fn duration(&self) -> Option<f64> {
        self.duration
    }

    /// Rebuilds the demuxer and the decoder from scratch.
    ///
    /// Symphonia asks for a reset when the container layout changes, which
    /// requires the decoder to be recreated.
    fn rebuild(&mut self) -> Result<(), DecodeError> {
        let reader = self.source.open_reader()?;

        let mut hint = Hint::new();
        if let Some(ext) = self.source.extension() {
            hint.with_extension(&ext);
        }

        let mss = MediaSourceStream::new(Box::new(ReadOnlySource::new(reader)), Default::default());
        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(DecodeError::Symphonia)?;

        let format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or(DecodeError::NoTrack)?;

        let track_id = track.id;
        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())?;

        self.format = format;
        self.decoder = decoder;
        self.track_id = track_id;
        Ok(())
    }

    /// Decodes the next packet into interleaved `f32` samples.
    ///
    /// Returns `Ok(None)` when the end of the stream is reached.
    pub fn decode_next(&mut self) -> Result<Option<Vec<f32>>, DecodeError> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::IoError(err))
                    if err.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(None)
                }
                Err(SymphoniaError::ResetRequired) => {
                    self.rebuild()?;
                    continue;
                }
                Err(err) => return Err(DecodeError::Symphonia(err)),
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            let samples = {
                let decoded = self.decoder.decode(&packet)?;
                let spec = *decoded.spec();
                let mut sample_buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
                sample_buffer.copy_interleaved_ref(decoded);
                sample_buffer.samples().to_vec()
            };

            if samples.is_empty() {
                continue;
            }
            return Ok(Some(samples));
        }
    }
}
