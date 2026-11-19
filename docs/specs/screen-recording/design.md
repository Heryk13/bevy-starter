# Screen recording: a video of the game alone, with its sound, on F9

Status: **built** on `main` on 2026-11-19, steps 1 to 6 written together at the user's word.
Date: 2026-11-19
Bevy: 0.20.0
Roadmap: row 8 of `docs/roadmap/roadmap.md`, from its "Screen recording, as agreed so far"
section. The look is the mockup's block "8" in `docs/roadmap/ui/mockup.html`. The sound and
the file were proven first in a throwaway spike (below).

## Goal

A game being made needs moving pictures of itself: a trailer, a clip for its page, a bug
that only shows in motion. An OS recorder catches the dev windows, the FPS and every other
program's sound. F9 records the game alone, its own UI included and the dev UI never, with
the game's sound alone, into the gallery the file explorer (row 4) browses, as an `.mp4` any
player opens. Nothing is installed outside cargo.

## The spike, 2026-11-19

A throwaway cargo project outside the repository, `spike-recording`, proved the two risks
the roadmap named before this design was written.

- **The game's sound alone.** Bevy 0.20 mixes every sound in rodio's mixer, held by
  `AudioOutput` in `bevy_audio-0.20.0/src/audio_output.rs`, which is `pub(crate)`: there is no
  point to listen to the mix from outside. Windows 10 (2004) and later can capture one
  process instead, the "process loopback" OBS uses for an application's sound, and the
  `wasapi` crate (0.27) wraps it: `AudioClient::new_application_loopback_client(pid, true)`.
  Ten seconds gave exactly 480000 frames at 48 kHz, in packets of 10 ms with no gap: Windows
  sends silence while the game is quiet, so the sound keeps time with the clock. The menu's
  buttons came through, and an alarm another program played at the same time did not.
- **A file cargo alone can write.** H.264 from `openh264` (0.9, its C built by cargo with
  the compiler Rust already uses), Opus from `opus-rs` (0.1, pure Rust) and the MP4 from
  `muxide` (0.5, pure Rust): ten seconds of a flashing picture with the game's sound and a
  beep on every flash opened in Windows' own player with picture and sound, the beep on the
  flash. AAC (`xaac-rs`) was the fallback, and was not needed.
- **Two things learnt on the way.** The dev build's `.exe` started alone dies at once (its
  Bevy DLLs are only on the path `cargo run` gives it); inside the game none of it matters,
  since the process to capture is its own (`std::process::id()`). And Explorer's details pane
  shows the file's audio bit rate but no duration: the file plays, so this is left for the
  build to look at, not a blocker.
- **Licences:** MIT (`wasapi`), BSD-2-Clause (`openh264`), BSD-3-Clause (`opus-rs`), MIT OR
  Apache-2.0 (`muxide`), all already in `about.toml`. They are dev-only crates, so none of
  them reaches a release build.

## Decided

| Question | Decision |
| --- | --- |
| A dev tool | Yes, on `main`: `src/dev_tools/recording.rs` with the parts in `recording/`, in dev builds only, its words in English. It records whatever the game draws, so it needs no player and works in an Empty game |
| Where it works | Gameplay only (`Screen::Gameplay`), as photo mode. F9 opens the bar only in `Pause::Off`, and does nothing in photo mode. While recording, F3 does nothing either: the two tools never run together. The pause menu may open while recording, and is recorded, since it is the game's own UI. Leaving gameplay stops the recording and writes the file |
| The flow | F9 opens a glass bar at the bottom centre and frees the cursor. F9 again, or Esc, closes it without recording, to walk closer to where the shot starts. **E**, or a click on Record, starts: the bar goes, the cursor locks again, a big number counts down in the middle of the screen, and at zero the recording starts and a REC pill shows in the top right corner. **E** stops it and writes the file, and so does F9. F9 during the countdown cancels it |
| E is shared with photo mode | One key belongs to one action (`swap` in `input.rs`), so E cannot be two. Photo mode's `take_photo` becomes `capture`, labelled "Capture" on the Dev page, and both tools read it in their own moment: photo mode in `Pause::Photo`, recording while its bar is open or it records. Photo mode's bar still says "Take Photo" beside the keycap, and recording's says "Record". A settings file with the old name keeps it as unknown, as `Bindings::load` does, and E starts on its default once more. Known and accepted: once row 12 puts doors on E, pressing E while recording opens a door and stops the video. A tool and a door meeting on one key is row 12's to settle, if it ever matters |
| The bar | The mockup's: Sound on / Sound off, 30 / 60 fps, the timer (None, 3, 5, 10 s), a line, then Record with the E keycap. A click on the sound or the frame rate flips it in place; only the timer opens a small list above itself, which closes on a choice. The choices last until the game closes (`RecordingChoices`, as the explorer's `DrawerHeight` does), starting on sound on, 60 fps and 3 s. They are not saved to disk: a recording's setup is a moment's, not a setting |
| The bar and the pointer | The bar is clicked, so it takes the pointer, and its root lets the pointer through, as the file explorer's drawer does: the game around it stays sharp. It carries `ClosesOnEsc`, so Esc closes it, and `3d/player`'s first-person camera frees the cursor while it is open with no change there, as it does for the explorer |
| The countdown | A large solid-white number in a round glass in the middle, counted in `Time<Real>`, so slow motion never slows it. None starts at once |
| The REC pill | Top right, where slow motion's and speed's capsules sit: a red dot blinking once a second, the elapsed time (`00:42`, then `1:02:05` past an hour), a stop icon and the E keycap, which says how to stop it. A hint only: the cursor is locked while playing, and nothing on the pill is clicked |
| What the video shows | The game alone, its own UI included, never the dev UI: REC, the countdown, the FPS, the F1 hint, every dev window. The dev windows already have a layer and a camera of their own (`DEV_TOOLS_Z_INDEX` in `dev_tools.rs`, drawn last by `blur.rs`), and that camera's frame, before it draws its own roots, is exactly the game alone: the point where `blur_layer_backdrops` copies what is under the layer for its glass. The frame is taken there, in the render world, scaled and turned into 8-bit colour by a small blit, and read back to the CPU with Bevy's `gpu_readback::Readback` |
| The two dev roots on the screen camera | Bevy's FPS overlay and the F1 hint are drawn by the screen camera today, so they would be in the video. Both move onto the dev layer: the F1 hint is ours (`spawn_keys_panel_hint`), and the FPS root, which Bevy spawns with `GlobalZIndex(FPS_OVERLAY_ZINDEX)`, gets `Floating` and the dev layer's `GlobalZIndex` from an observer in `fps.rs`. The dev layer's camera then draws every frame, which costs one more UI pass over a frame that already has several |
| Frame rate | 30 or 60, the bar's choice, counted in `Time<Real>`: a frame is taken each time the real clock passes the next slot (`1 / fps`), whatever the game's own frame rate. A game at 144 fps gives 60 of them a second; a game at 40 gives 40 frames stamped at their own slots, and the video holds each one until the next. Slow motion slows the game and not the clock, so the video comes out in slow motion and the sound at normal speed, as the roadmap wants |
| Size and quality | The window's size in physical pixels, rounded down to even numbers, as H.264's colour halves need: what the player sees is what the video holds. `VIDEO_BITRATE_BPS` (10 Mbit/s) in `Config`, a keyframe every second so the file seeks quickly. Tuned by eye once it runs |
| Encoding | On a thread of its own (`std::thread`), fed through a bounded channel, so the game never waits for the encoder. A frame that finds the channel full is dropped and counted, and the log says how many at the end: the video keeps its timestamps, so it stays in time with the sound even when the encoder falls behind. The CPU turns RGBA into YUV with `openh264`'s `YUVBuffer`, on that thread |
| Sound | `wasapi` process loopback on the game's own process, on a thread of its own, 48 kHz stereo `f32`, from the instant the first frame is taken; encoded to Opus in 20 ms packets of 960 frames and stamped by the count of frames so far. Sound off records no sound track at all. `wasapi` is Windows-only (`[target.'cfg(windows)'.dependencies]`): elsewhere the video has no sound, the bar shows Sound off and does not flip, and the log says why once |
| The file | `gallery/` in the data folder, where photos go, named by the second the recording started as photos are (`2026-11-19_20-11-05.mp4`, `_2` after a name taken), so the file explorer lists it and opens it in the OS's player (`Kind::Video` is already there). It is written into `<name>.mp4.part` and renamed once `muxide` finishes, so a crash mid-recording never leaves a broken `.mp4` behind; that is what `write_atomic` promises, for a file written as it grows |
| Closing the game while recording | Stops it and writes the file whole: on `AppExit` the recorder finishes the muxer and the threads are joined before the app returns |
| The dates | `photo_stamp` and `photo_file_name` move from `photo_mode/photo.rs` into the "Dates" section of `dev_tools.rs`, beside `civil_from_days`, named `file_stamp` and `numbered_file_name`: two tools now name files by the second. They move now and not at a third user because the section they go to already holds what the tools share about dates |
| Release builds | Everything here is in `dev_tools/`. The four crates are optional and turned on by the `dev` feature in `Cargo.toml`, so a release build neither compiles nor links them |

## Out of scope

- **Sound on macOS and Linux.** The video records there without sound until someone writes
  that half.
- **Recording outside gameplay** (the menu, the splash) and **while in photo mode**.
- **A size or quality picker**, a microphone, a webcam, a cursor drawn into the video.
- **Saving the bar's choices** to `settings.ron`.
- **Hardware encoders** (NVENC, Media Foundation): `openh264` in software is enough to see
  whether it keeps up, and the frames it drops are counted, so the build shows whether it is.
- **Editing or trimming** a recording: the OS's player and editors do that.

## Structure

### `main`

`Cargo.toml`: `openh264`, `opus-rs`, `muxide` and (on Windows) `wasapi`, all optional, in the
`dev` feature.

`dev_tools.rs`: `mod recording;` and its plugin after slow motion and speed (F9 after F8);
the "Dates" section gains `file_stamp` and `numbered_file_name` from `photo.rs`.

`photo_mode/photo.rs`: `TAKE_PHOTO` becomes `CAPTURE` (`"capture"`, "Capture"), `pub(super)`
from `dev_tools.rs` or a shared place the two tools reach (settled while writing); the bar's
hint keeps the words "Take Photo". `photo_mode.rs`'s toggle gets
`.run_if(in_state(RecordingState::Off))`.

`fps.rs`: an observer puts the FPS root on the dev layer. `keys_panel.rs`: the F1 hint spawns
on the dev layer.

`recording.rs`: the `RECORD` action (F9, "Screen Recording"); `RecordingState` (`Off`, `Bar`,
`Countdown`, `On`), a `States` behind the `dev` feature, so whatever each spawns carries
`DespawnOnExit` of it; the systems that move between them on F9, E, Esc and the timer; the
countdown and the REC pill. `recording/bar.rs`: the bar, its three controls, the timer's
list, `RecordingChoices`. `recording/capture.rs`: the render-world blit and readback of the
dev layer's frame, the frame slots. `recording/encoder.rs`: the encoding thread, `openh264`
and `muxide`, the `.part` file and its rename. `recording/sound.rs`: the `wasapi` thread and
`opus-rs`, behind `cfg(windows)`.

### Branches

Nothing: the recording is the window, and every branch's game draws into it. `3d/player`
frees the cursor under the bar through `ClosesOnEsc`, which it already reads.

## Text

English literals, as every dev tool's: "Screen Recording" (the Dev page), "Capture" (the
shared action), "Sound on", "Sound off", "30 fps", "60 fps", "None", "3 s", "5 s", "10 s"
(the list's rows say "3 seconds"), "Record", "REC" is not written: the red dot says it.

## Tests

- The frame slots: at 60 fps, a frame is due at 0, again only once `1 / 60` s has passed,
  and a long frame of the game skips the slots it missed rather than sending them all.
- The elapsed time: `42 s` reads `00:42`, `3725 s` reads `1:02:05`.
- The countdown: 3 s at `0.5 s` shows `3`, at `2.5 s` shows `1`, at `3.0 s` is over.
- The bar: the frame rate flips between 30 and 60; the sound does not flip where there is no
  sound to record.
- The size: an odd window size rounds down to even, and nothing goes below 2.
- The file name: the second's stamp, then `_2` after a name taken, over a `fresh_test_dir`
  (the date tests move with `file_stamp` and keep their names).
- The sound's timestamps: packet `n` starts at `n * 960 / 48000` s.

## Implementation order

Each step compiles, passes `just check` and runs; each is a commit on `main` and reaches the
branches by the merge down.

1. **E becomes Capture.** The rename, the dates moved into `dev_tools.rs`. *Try:* E still
   takes a photo in photo mode; the Dev page lists "Capture".
2. **The dev UI on its own layer.** The FPS root and the F1 hint on the dev layer. *Try:*
   both look as before, over the pause menu and the explorer too.
3. **The bar on F9.** `RecordingState`, the bar, its controls and list, `RecordingChoices`,
   Esc and F9 to close. *Try:* the choices flip and stay after closing and opening; on
   `3d/player` the cursor comes free under it.
4. **The countdown and the REC pill**, with nothing recorded yet. *Try:* E counts down and
   shows the pill with its time; E and F9 stop; F9 cancels the countdown; F3 does nothing
   meanwhile.
5. **The frames, into a video with no sound.** The capture point, the readback, the encoding
   thread, the `.part` file, `AppExit`. This is where the build may still learn something:
   the copy taken before the dev layer draws, across HDR and non-HDR cameras, is the one
   piece the spike did not try. *Try:* a video of the game alone, no REC, no FPS, no F1, at the
   window's size, opened from the explorer; closing the game mid-recording leaves a whole file.
6. **The sound.** `wasapi` and `opus-rs`, Sound off, other systems. *Try:* the game's sound
   and nothing else, in time with the picture; slow motion gives a slow picture and normal
   sound.
7. **Docs.** `README.md`, `docs/flow.md` (the state, the keys, the dev layer), `docs/template.md`
   (the files), the roadmap's row 8, this spec's status; delete `spike-recording`.

## What changed while building it

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
