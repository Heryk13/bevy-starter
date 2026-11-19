use bevy::prelude::*;
use muxide::api::{AudioCodec, Muxer, MuxerBuilder, VideoCodec};
use openh264::OpenH264API;
use openh264::encoder::{BitRate, Encoder, EncoderConfig, FrameRate, FrameType, IntraFramePeriod};
use openh264::formats::{RgbaSliceU8, YUVBuffer};
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread::JoinHandle;

// ==== Config =================================================================

// the picture's bits a second: enough for a sharp 1080p at 60 fps in H.264's baseline,
// which is what openh264 writes; tuned by eye
const VIDEO_BITRATE_BPS: u32 = 10_000_000;
// how many frames may wait for the encoder before a new one is dropped instead: half a
// second at 60 fps, so a short stall is caught up and a long one never fills the memory
const FRAMES_WAITING: usize = 30;
// the sound, as `sound.rs` captures it and Opus always counts it
pub(super) const SOUND_RATE_HZ: u32 = 48_000;
pub(super) const SOUND_CHANNELS: u16 = 2;
// what a file is written under while it grows, before it is whole
const PART_EXTENSION: &str = "part";

// ==== Encoder ================================================================

/// A frame read back from the GPU: the moment of the video it shows, in seconds, and its
/// RGBA bytes, each row padded to the 256 bytes a GPU copy is aligned to.
pub(super) struct VideoFrame {
    pub(super) pts_secs: f64,
    pub(super) padded_rgba: Vec<u8>,
}

/// An Opus packet of the game's sound, and the moment of the video it starts at.
#[cfg_attr(
    not(windows),
    expect(
        dead_code,
        reason = "only Windows records sound, so nothing builds one elsewhere"
    )
)]
pub(super) struct SoundPacket {
    pub(super) pts_secs: f64,
    pub(super) data: Vec<u8>,
}

/// What a recording is written with: its size, its frames a second, and the sound's packets
/// if it records any.
pub(super) struct VideoFormat {
    pub(super) size: UVec2,
    pub(super) frame_rate: u32,
    pub(super) sound: Option<Receiver<SoundPacket>>,
}

/// Starts the thread that encodes a recording into `path`: H.264 frames from the sender it
/// returns, the sound's packets from `format.sound`, both into one `.mp4`. It is written
/// under `path` with `.part` after it and renamed once whole, so a crash leaves no broken
/// video. The thread ends, the file written, once the sender is dropped and the sound's
/// sender is too.
pub(super) fn start_encoder(
    path: PathBuf,
    format: VideoFormat,
) -> (SyncSender<VideoFrame>, JoinHandle<()>) {
    let (frames, received) = sync_channel(FRAMES_WAITING);
    let thread = std::thread::spawn(move || {
        if let Err(err) = encode(&path, format, &received) {
            error!("could not write the recording {}: {err}", path.display());
            // a file that never finished plays nowhere, and would sit in the gallery
            let _ = std::fs::remove_file(part_path(&path));
        }
    });
    (frames, thread)
}

/// Encodes every frame and packet into `path`, then finishes the file and renames it.
fn encode(
    path: &std::path::Path,
    format: VideoFormat,
    frames: &Receiver<VideoFrame>,
) -> Result<(), Box<dyn std::error::Error>> {
    let part = part_path(path);
    let mut builder = MuxerBuilder::new(BufWriter::new(File::create(&part)?)).video(
        VideoCodec::H264,
        format.size.x,
        format.size.y,
        f64::from(format.frame_rate),
    );
    if format.sound.is_some() {
        builder = builder.audio(AudioCodec::Opus, SOUND_RATE_HZ, SOUND_CHANNELS);
    }
    let mut muxer = builder.build()?;
    let config = EncoderConfig::new()
        .bitrate(BitRate::from_bps(VIDEO_BITRATE_BPS))
        .max_frame_rate(FrameRate::from_hz(format.frame_rate as f32))
        // a keyframe every second, so the video seeks quickly
        .intra_frame_period(IntraFramePeriod::from_num_frames(format.frame_rate));
    let mut encoder = Encoder::with_api_config(OpenH264API::from_source(), config)?;
    let (width, height) = (format.size.x as usize, format.size.y as usize);
    let mut rgba = Vec::with_capacity(width * height * 4);
    let mut video_written = false;

    for frame in frames {
        if !unpad_rows(&frame.padded_rgba, width, height, &mut rgba) {
            continue;
        }
        let picture = YUVBuffer::from_rgb8_source(RgbaSliceU8::new(&rgba, (width, height)));
        let bits = encoder.encode(&picture)?;
        let data = bits.to_vec();
        // a frame openh264 chose to skip has no bytes, and nothing to write
        if data.is_empty() {
            continue;
        }
        let keyframe = matches!(bits.frame_type(), FrameType::IDR | FrameType::I);
        muxer.write_video(frame.pts_secs, &data, keyframe)?;
        video_written = true;
        // after the frame, never before the first: an MP4's sound needs a picture to follow,
        // and the packets that came early wait in their channel until then
        if let Some(sound) = &format.sound {
            write_sound(&mut muxer, sound.try_iter())?;
        }
    }
    if !video_written {
        return Err("no frame of the video arrived".into());
    }
    // the frames are over; the sound's last packets come until its thread lets go
    if let Some(sound) = &format.sound {
        write_sound(&mut muxer, sound.iter())?;
    }
    muxer.finish()?;
    std::fs::rename(&part, path)?;
    info!("recording written: {}", path.display());
    Ok(())
}

/// Writes the sound's packets into the video.
fn write_sound<W: std::io::Write>(
    muxer: &mut Muxer<W>,
    packets: impl Iterator<Item = SoundPacket>,
) -> Result<(), muxide::api::MuxerError> {
    for packet in packets {
        muxer.write_audio(packet.pts_secs, &packet.data)?;
    }
    Ok(())
}

// ==== Helpers ================================================================

/// Returns where a recording is written while it grows: its path with `.part` after it.
pub(super) fn part_path(path: &std::path::Path) -> PathBuf {
    let mut part = path.as_os_str().to_owned();
    part.push(".");
    part.push(PART_EXTENSION);
    PathBuf::from(part)
}

/// Copies `padded`, rows of `width` RGBA pixels each padded to the GPU's alignment, into
/// `rgba` with no padding. Returns `false`, leaving it empty, when `padded` holds fewer than
/// `height` such rows: a frame of another size.
fn unpad_rows(padded: &[u8], width: usize, height: usize, rgba: &mut Vec<u8>) -> bool {
    rgba.clear();
    let row = width * 4;
    let Some(stride) = padded.len().checked_div(height) else {
        return false;
    };
    if stride < row {
        return false;
    }
    for line in padded.chunks_exact(stride).take(height) {
        rgba.extend_from_slice(&line[..row]);
    }
    true
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padding_is_cut_from_the_end_of_each_row() {
        // two rows of one pixel, each padded to 8 bytes
        let padded = [1, 2, 3, 4, 0, 0, 0, 0, 5, 6, 7, 8, 0, 0, 0, 0];
        let mut rgba = Vec::new();
        assert!(unpad_rows(&padded, 1, 2, &mut rgba));
        assert_eq!(rgba, [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn a_frame_too_small_for_the_video_is_refused() {
        let mut rgba = Vec::new();
        assert!(!unpad_rows(&[0; 4], 2, 2, &mut rgba));
    }

    #[test]
    fn a_recording_grows_under_a_part_name() {
        let path = std::path::Path::new("gallery").join("2026-11-19_20-11-05.mp4");
        assert_eq!(
            part_path(&path),
            std::path::Path::new("gallery").join("2026-11-19_20-11-05.mp4.part")
        );
    }
}
