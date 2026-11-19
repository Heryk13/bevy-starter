# Screen recording: tasks

Roadmap row 8, built on `main`. The decisions and their reasons live in the spec,
`docs/specs/screen-recording/design.md`; this file only breaks its implementation order into
tasks small enough for one sitting each, with the facts each task needs. The look is the
mockup's block "8" in `docs/roadmap/ui/mockup.html`. The spike's code, the reference for the
three crates' calls, is in `..\spike-recording` (`src/main.rs` for the sound, `src/bin/mp4.rs`
for the file) until step 7 deletes it.

## How to read this

- **Step** = one commit. Its title is the commit message.
- **Who**: *Claude* writes each step in pieces and the user reads and runs it; *you* is a
  `just run`, which only the user's eyes can do; *both* is the review at the end of a step
  (the checklist in `CLAUDE.md`: every name, the order of sections and systems, `just check`).
- **Facts**: the names that cannot be guessed, checked in the Bevy 0.20.0 source, the spike
  and this repository on 2026-11-19.
- **Where**: every step is written and tried uncommitted on `main`, then synced: committed,
  merged down the tree and pushed. When to sync is the user's call each time.
- **Nothing unused, in either build**: the four crates and everything naming them sit behind
  the `dev` feature, and `wasapi` behind `cfg(windows)` too.
- Mark a task done by changing its `[ ]` to `[x]`.

## Departures from the spec, to record in its "What changed while building it"

- **Steps 1 to 6 were written together** (asked by the user, to try them all at once), so
  they are one sitting, the user's runs came at the end, and the seven steps went in as
  one commit with the docs.
- **Capture lives in `dev_tools.rs`** (decided while building 1.1): the constant, its key and
  its label sit in its `Config`, where both tools reach them, as `DEV_TOOLS_Z_INDEX` does.
  `photo.rs` still registers it, so the Dev page lists it right after Photo Mode.
- **The FPS and the F1 hint are lent to the dev layer only while recording**, not moved for
  good (found while building 2.2): an open layer blurs every frame, as the comment on
  `spawn_keys_panel_hint` says, so a layer open all the time would cost every frame of every
  game. `lend_dev_layer` gives both roots `Floating` and the layer's `GlobalZIndex` on
  entering `RecordingState::On`, which `blur.rs`'s observer sends to the layer's camera, and
  `return_dev_layer` gives them back on leaving it. The REC pill opens the layer anyway.
- **`LayerCamera` is `pub(crate)`** in `ui/blur.rs`: the recording finds the dev layer's
  camera by it.
- **The REC pill sits at the top centre**, not the top right (decided while building 4.2):
  slow motion's and speed's capsules open there, and both may be open while recording.
- **A frame is read back only when the video needs one** (decided while building 5.2): a
  `ReadbackOnce` per frame of the video, spawned when the real clock passes its slot, not a
  `Readback` every frame of the game. The copy itself runs every frame while recording; it
  is one small blit.
- **The copy is Bevy's own blit** (found in 5.1): `BlitPipeline` in `bevy::core_pipeline::blit`
  is public, and specialised by the view's `ResolvedCompositingSpace` into an `Rgba8UnormSrgb`
  image, as Bevy's upscaling does for the window, so no shader of our own was needed. A
  readback's rows are padded to 256 bytes; the encoder cuts the padding (`unpad_rows`).
- **The sound starts before the encoder** (decided while building 6.1): `start_sound` waits
  for the capture to begin, so the encoder knows whether the file has a sound track, and a
  Windows too old for process loopback records the video without one.
- **The sound follows the first frame into the file** (found in the user's first run, which
  left an empty `.part`): `muxide` refuses sound before any picture, so the encoder writes
  the waiting packets after each frame, never before the first, and a recording whose
  encoder fails deletes its `.part`.
- **The capture's systems are `start_capture` and `stop_capture`** in `capture.rs`, beside
  `recording.rs`'s `stop_recording`, the one E runs.

## The actions at a glance

| Action | Default | Label on the Dev page and the panel | Heard | Step |
|--------|---------|-------------------------------------|-------|------|
| `capture` (was `take_photo`) | E | Capture | in photo mode; while the recording bar is open or it records | 1 |
| `record` | F9 | Screen Recording | in gameplay, `Pause::Off`, never in photo mode | 3 |

## Step 1: `refactor: share E between photo mode and recording`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 1.1 `TAKE_PHOTO` becomes `CAPTURE` (`"capture"`, labelled "Capture"), reachable from `recording.rs`; photo mode's hint keeps the words "Take Photo" | Claude | `register_inner_key` and `register_photo_hint` in `photo.rs`; a settings file holding `take_photo` keeps it as unknown (`Bindings::load`) |
| [x] | 1.2 `photo_stamp` and `photo_file_name` move to the "Dates" section of `dev_tools.rs` as `file_stamp` and `numbered_file_name`, their tests with them | Claude | beside `civil_from_days` and the `SECS_PER_*` constants |
| [x] | 1.3 `just run`: E still takes a photo; the Dev page lists "Capture". Review | both | |

## Step 2: `refactor: draw the FPS and the F1 hint on the dev layer`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 2.1 `fps.rs`: an observer gives Bevy's FPS root `Floating` and the dev layer's `GlobalZIndex` | Claude | `FpsText` is private to Bevy, so the observer finds the root by `On<Add, GlobalZIndex>` holding `FPS_OVERLAY_ZINDEX` (`bevy::dev_tools::fps_overlay`, `i32::MAX - 32`); `Floating` in `crate::ui::blur`; `DEV_TOOLS_Z_INDEX` in `dev_tools.rs` |
| [x] | 2.2 `keys_panel.rs`: the F1 hint spawns on the dev layer | Claude | `spawn_keys_panel_hint`; `floating_root(DEV_TOOLS_Z_INDEX)` with `Pickable::IGNORE` lets the pointer through |
| [x] | 2.3 `just run`: both look as before, over the menu, the pause menu and the explorer; photo mode still hides both. Review | both | the dev layer's camera now draws every frame (`activate_layer_cameras`) |

## Step 3: `feat: open the recording bar on F9`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 3.1 `Cargo.toml`: `openh264`, `opus-rs`, `muxide` optional, `wasapi` optional under `[target.'cfg(windows)'.dependencies]`, all in the `dev` feature (`dep:...`) | Claude | the spike's versions: `openh264` 0.9.10, `opus-rs` 0.1.42, `muxide` 0.5.0, `wasapi` 0.27.0; the step that first uses each adds it, so nothing is unused: settled while writing |
| [x] | 3.2 `src/dev_tools/recording.rs`: the `RECORD` action (F9), `RecordingState` (`Off`, `Bar`, `Countdown`, `On`), F9 and Esc opening and closing the bar, only in `Screen::Gameplay` and `Pause::Off`; added to `dev_tools.rs` after speed | Claude | `init_state` needs `StatesPlugin`, which `DefaultPlugins` brings; `DespawnOnExit(RecordingState::Bar)` |
| [x] | 3.3 `recording/bar.rs`: the bar at the bottom centre, taking the pointer under a root that lets it through, with `ClosesOnEsc`; Sound on/off, 30/60 fps, the timer and its list, a line, Record with the E keycap; `RecordingChoices` | Claude | the file explorer's drawer for the pointer; `ButtonKind` and `icon_button` in `ui/button.rs`; `keycap(CAPTURE, ..)` from `crate::overlays`; icons `Volume2`, `VolumeX`, `Film`, `Timer`, `ChevronDown`, `CircleDot`, `Check` |
| [x] | 3.4 The pure helpers and their tests: the frame rate's flip, the timer's rows | Claude | |
| [x] | 3.5 `just run`: the choices flip and stay after closing and opening; the list closes on a choice; F9 does nothing in photo mode or over the pause menu; on `3d/player` the cursor comes free under the bar. Review | both | |

## Step 4: `feat: count down and show REC`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 4.1 E or Record enters `Countdown`, or `On` at once with no timer; the big number in a round glass, counted in `Time<Real>`; F9 cancels it | Claude | `TITLE_FONT_SIZE_PX` or larger in `TEXT`; `Res<Time<Real>>` |
| [x] | 4.2 The REC pill in the top right in `On`: the blinking red dot, the elapsed time, a stop icon, the E keycap; E or F9 back to `Off` | Claude | `DANGER` for the dot; `Icon::Square` for the stop |
| [x] | 4.3 `photo_mode.rs`'s F3 runs only in `RecordingState::Off`; leaving `Screen::Gameplay` goes back to `Off` | Claude | `.run_if(in_state(RecordingState::Off))`; `OnExit(Screen::Gameplay)` |
| [x] | 4.4 The pure helpers and their tests: the countdown's number, the elapsed time's text | Claude | |
| [x] | 4.5 `just run`: the countdown and the pill, E and F9 stop, F9 cancels, F3 does nothing meanwhile. Review | both | |

## Step 5: `feat: record the game alone into a video`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 5.1 Research first, reported before writing: where in the render world the dev layer's camera has the frame under its roots, in both an HDR and a non-HDR gameplay, and what its texture's format and size are | Claude | `blur_layer_backdrops` in `ui/blur.rs` (`Core2d`, `.after(Core2dSystems::PostProcess)`, `.before(ui_pass)`) and its `ViewTarget`; the dev layer's camera is told apart by its `LayerCamera(DEV_TOOLS_Z_INDEX)` |
| [x] | 5.2 `recording/capture.rs`: a blit of that frame into an 8-bit RGBA image of the window's even size, and `Readback` on it while recording | Claude | `bevy::render::gpu_readback::{Readback, ReadbackComplete}`: `Readback::texture(handle)` reads every frame; `ReadbackComplete` carries the bytes |
| [x] | 5.3 The frame slots by `Time<Real>`: which readbacks become frames, and their timestamps; the pure helper and its tests | Claude | |
| [x] | 5.4 `recording/encoder.rs`: the thread, a bounded `std::sync::mpsc::sync_channel`, `openh264` into `muxide` with no sound track, `<name>.mp4.part` renamed on finish, dropped frames counted | Claude | the spike's `src/bin/mp4.rs`: `Encoder::with_api_config(OpenH264API::from_source(), EncoderConfig::new().bitrate(..).max_frame_rate(..).intra_frame_period(..))`, `YUVBuffer::from_rgb8_source(RgbaSliceU8::new(..))`, `frame_type()` `IDR`/`I` is a keyframe, `to_vec()` is Annex B; `MuxerBuilder::new(file).video(VideoCodec::H264, w, h, fps).build()`, `write_video(pts, data, key)`, `finish()` |
| [x] | 5.5 The file: `gallery_dir()`, `file_stamp`, `numbered_file_name`; stopping on E, F9, leaving gameplay and `AppExit` all finish it and join the thread | Claude | `on_message::<AppExit>` |
| [x] | 5.6 The pure helpers and their tests: the even size, the file name over a `fresh_test_dir` | Claude | |
| [x] | 5.7 `just run`: a video of the game alone (no REC, FPS, F1 or dev window) at the window's size, opened from the explorer; slow motion gives a slow video; the log's dropped frames; closing the game mid-recording leaves a whole file. Review | both | Explorer's details pane showed no duration in the spike: look again |

## Step 6: `feat: record the game's sound`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 6.1 `recording/sound.rs`, behind `cfg(windows)`: the `wasapi` thread on `std::process::id()`, started with the first frame | Claude | the spike's `src/main.rs`: `initialize_mta()`, `WaveFormat::new(32, 32, SampleType::Float, 48000, 2, None)`, `AudioClient::new_application_loopback_client(pid, true)`, `initialize_client(&format, Direction::Capture, StreamMode::EventsShared { autoconvert: true, buffer_duration_hns: 0 }, None)`, `set_get_eventhandle`, `get_audiocaptureclient`, `read_from_device_to_deque` |
| [x] | 6.2 `opus-rs` into the same muxer: 20 ms packets of 960 frames, stamped by the frames so far; Sound off adds no track | Claude | `OpusEncoder::new(48000, 2, Application::Audio)`, `bitrate_bps`, `encode(&input, 960, &mut packet)`; `MuxerBuilder::audio(AudioCodec::Opus, 48000, 2)`, `write_audio(pts, packet)` |
| [x] | 6.3 Off Windows: Sound off and fixed, logged once | Claude | |
| [x] | 6.4 The sound's timestamps and its test | Claude | |
| [x] | 6.5 `just run`: the game's sound alone, in time with the picture, with another program playing; Sound off gives a silent video; slow motion keeps the sound at normal speed. Review | both | |

## Step 7: `docs: mark the screen recording spec built`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 7.1 `README.md` (F9 in the dev keys), `docs/flow.md` (`RecordingState`, the keys, the dev layer holding the FPS and F1), `docs/template.md` (the files), the roadmap's row 8, the spec's status and "What changed" | Claude | |
| [x] | 7.2 Delete `..\spike-recording` | Claude | |
| [ ] | 7.3 Sync; `just run` on `main`, `3d/player` and `2d/player` | you | |
