use bevy::prelude::*;
use opus_rs::{Application, OpusEncoder};
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel, sync_channel};
use std::thread::JoinHandle;
use wasapi::{AudioClient, Direction, SampleType, StreamMode, WaveFormat, initialize_mta};

use super::encoder::{SOUND_CHANNELS, SOUND_RATE_HZ, SoundPacket};

// ==== Config =================================================================

// Opus's usual packet: 20 ms of sound
const PACKET_FRAMES: usize = SOUND_RATE_HZ as usize / 50;
// the sound's bits a second: clear stereo for music and effects alike
const SOUND_BITRATE_BPS: i32 = 128_000;
// how long the thread waits for Windows to hand it sound before it looks at the stop flag
const WAIT_MS: u32 = 100;
// the largest packet Opus writes is far smaller than this
const PACKET_BYTES_MAX: usize = 4000;

// ---- derived ----------------------------------------------------------------
const BYTES_PER_SAMPLE: usize = size_of::<f32>();
const PACKET_BYTES: usize = PACKET_FRAMES * SOUND_CHANNELS as usize * BYTES_PER_SAMPLE;

// ==== Sound ==================================================================

/// The thread hearing the game's sound, and the flag that stops it.
pub(super) struct SoundCapture {
    stop: Arc<AtomicBool>,
    pub(super) thread: JoinHandle<()>,
}

impl SoundCapture {
    /// Tells the thread to stop; it ends, letting go of its packets' sender, within
    /// `WAIT_MS`.
    pub(super) fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Starts hearing the game's own sound, through Windows' process loopback, which leaves
/// every other program out, and encoding it to Opus as it comes. Returns the thread and the
/// packets' receiver, or `None`, logged, when Windows cannot capture it (before Windows 10's
/// 2004 update): the video is then recorded without sound.
pub(super) fn start_sound() -> Option<(SoundCapture, Receiver<SoundPacket>)> {
    let (packets, received) = channel();
    let (ready, is_ready) = sync_channel(1);
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = stop.clone();
    let thread = std::thread::spawn(move || {
        if let Err(err) = capture(&packets, &ready, &thread_stop) {
            // before `ready` is sent this is the reason `start_sound` gives up; after, the
            // packets simply end
            error!("the recording's sound stopped: {err}");
        }
    });
    match is_ready.recv() {
        Ok(true) => Some((SoundCapture { stop, thread }, received)),
        _ => {
            warn!("the game's sound cannot be captured here: recording without it");
            None
        }
    }
}

/// Captures the game's sound until `stop`, sending Opus packets stamped by the sound's own
/// count of frames, which Windows keeps at the clock's pace: it sends silence while the game
/// is quiet. Says on `ready` whether the capture began.
fn capture(
    packets: &std::sync::mpsc::Sender<SoundPacket>,
    ready: &std::sync::mpsc::SyncSender<bool>,
    stop: &AtomicBool,
) -> Result<(), Box<dyn std::error::Error>> {
    let started = start_client();
    let _ = ready.send(started.is_ok());
    let (client, event, capture) = started?;
    let mut opus = OpusEncoder::new(
        SOUND_RATE_HZ as i32,
        usize::from(SOUND_CHANNELS),
        Application::Audio,
    )?;
    opus.bitrate_bps = SOUND_BITRATE_BPS;
    let mut bytes: VecDeque<u8> = VecDeque::new();
    let mut packet = vec![0_u8; PACKET_BYTES_MAX];
    let mut samples = vec![0.0_f32; PACKET_BYTES / BYTES_PER_SAMPLE];
    let mut raw = Vec::with_capacity(PACKET_BYTES);
    let mut packets_sent: u64 = 0;
    while !stop.load(Ordering::Relaxed) {
        if event.wait_for_event(WAIT_MS).is_err() {
            continue;
        }
        while capture.get_next_packet_size()?.unwrap_or(0) > 0 {
            capture.read_from_device_to_deque(&mut bytes)?;
        }
        while bytes.len() >= PACKET_BYTES {
            raw.clear();
            raw.extend(bytes.drain(..PACKET_BYTES));
            for (sample, sample_bytes) in samples.iter_mut().zip(raw.as_chunks().0) {
                *sample = f32::from_le_bytes(*sample_bytes);
            }
            let len = opus.encode(&samples, PACKET_FRAMES, &mut packet)?;
            let sent = packets.send(SoundPacket {
                pts_secs: packet_pts_secs(packets_sent),
                data: packet[..len].to_vec(),
            });
            if sent.is_err() {
                // the encoder is gone: nothing is listening any more
                return Ok(());
            }
            packets_sent += 1;
        }
    }
    client.stop_stream()?;
    Ok(())
}

/// Opens the process loopback on the game's own process and starts it.
fn start_client()
-> Result<(AudioClient, wasapi::Handle, wasapi::AudioCaptureClient), Box<dyn std::error::Error>> {
    initialize_mta().ok()?;
    let format = WaveFormat::new(
        32,
        32,
        SampleType::Float,
        SOUND_RATE_HZ as usize,
        usize::from(SOUND_CHANNELS),
        None,
    );
    // the game's own process, and whatever it starts
    let mut client = AudioClient::new_application_loopback_client(std::process::id(), true)?;
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: 0,
    };
    client.initialize_client(&format, Direction::Capture, mode, None)?;
    let event = client.set_get_eventhandle()?;
    let capture = client.get_audiocaptureclient()?;
    client.start_stream()?;
    Ok((client, event, capture))
}

// ==== Helpers ================================================================

/// Returns the moment the `index`th packet of sound starts at, in seconds from the start.
fn packet_pts_secs(index: u64) -> f64 {
    (index * PACKET_FRAMES as u64) as f64 / f64::from(SOUND_RATE_HZ)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-9;

    #[test]
    fn each_packet_starts_where_the_one_before_ends() {
        let packet_secs = PACKET_FRAMES as f64 / f64::from(SOUND_RATE_HZ);
        assert!(packet_pts_secs(0).abs() < TOLERANCE);
        assert!((packet_pts_secs(3) - 3.0 * packet_secs).abs() < TOLERANCE);
    }
}
