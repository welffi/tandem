# Tandem

A Windows video player and editor that plays every audio track of a video at once. Rust and egui. This is a port of the Electron app in `..\Tandem`, with full parity as the goal. The Electron app is the reference: read it, but don't change it.

## Commands

`cargo` may not be on PATH. If so, use `%USERPROFILE%\.cargo\bin\cargo.exe`.

- Build: `cargo build`
- Checks: `cargo test`
- Release: `build.bat`, which makes `dist\Tandem.exe`. Close Tandem first, since a running exe can't be replaced.
- Format: `cargo fmt -- --config max_width=200,use_small_heuristics=Max`

## How it works

- ffmpeg and ffprobe download on first run into the data folder (`src\ffmpeg_install.rs`).
- The data folder is `%APPDATA%\Tandem`. Set `TANDEM_DATA` to use another folder. Test runs must set it to a scratch folder.
- Only one copy runs per data folder. A second copy passes its files to the first through `inbox.txt` and exits (`src\win.rs`).
- Audio is the clock. `audio_out::Player` mixes every track and the playhead follows it.
- The preview is drawn on the CPU in a worker thread (`src\preview.rs`, `src\compose.rs`).
- Unsaved work is kept in `recovery.json` and offered back on the next start.

## Testing

welffi does the feel testing. Build, run the checks, take one screenshot if the look changed, then run `build.bat` and hand it over. Don't write simulated mouse scripts.

Add a check for new logic that has real branches. Keep that logic in a plain function. Checks live in `src\tests\`.

## Keeping the code clean

- Make the smallest change that fully does the job.
- Delete code a change makes unused in that same change.
- No comments in code. Use clear names.
- Keep files to one job and under about 400 lines.
- Colors go in `src\theme.rs`.

## Style for welffi

- Plain voice in docs and notes: simple sentences, no em dashes.
- Commit or push only when asked. Commits use the noreply email.
