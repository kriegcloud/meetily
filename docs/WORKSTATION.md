# CachyOS workstation source build

This workstation uses the `kriegcloud/meetily` fork, a Vulkan build, and a
user-local installation at `~/.local/lib/meetily-app`. Meeting data remains in
`~/.local/share/com.meetily.ai` and recordings in `~/Documents/meetily-recordings`.

## Rebuild

Run from the repository root. Use pnpm 9 because pnpm 12 ignores this project's
`package.json` overrides. Use clang 18 for the pinned Whisper bindings.

```sh
cd frontend
npx --yes pnpm@9.15.9 install --frozen-lockfile
npx --yes pnpm@9.15.9 build
cd ..
export LIBCLANG_PATH=/usr/lib/llvm18/lib
export CARGO_BUILD_JOBS=6
export CMAKE_BUILD_PARALLEL_LEVEL=6
cargo build --locked --release -p llama-helper --features vulkan
mkdir -p frontend/src-tauri/binaries
cp target/release/llama-helper frontend/src-tauri/binaries/llama-helper-x86_64-unknown-linux-gnu
cp /usr/bin/ffmpeg frontend/src-tauri/binaries/ffmpeg-x86_64-unknown-linux-gnu
cd frontend
./node_modules/.bin/tauri build --no-bundle --features vulkan \
  --config '{"build":{"beforeBuildCommand":""}}' -- --locked
cd ..
```

The installed tree needs `bin/meetily`, `bin/llama-helper`, `bin/ffmpeg`, and
`lib/meetily/templates/*.json`. Close Meetily before replacing its installation;
retain a copy of the old tree and back up the SQLite database with SQLite's
backup API. Copy the three binaries from `target/release` (FFmpeg from `/usr/bin`)
and templates from `frontend/src-tauri/templates` into a staging tree, then swap
the staging tree into place. The launcher retains
`WEBKIT_DISABLE_DMABUF_RENDERER=1` for KDE/Wayland rendering.
Use the Tauri build command for the desktop binary: a plain Cargo build does
not enable Tauri's production asset protocol and opens an empty development view.

## Audio

Choose `pulse (input)` for the microphone and
`meetily_monitor (System Audio) (output)` for system audio. The latter requires
the existing `pcm.meetily_monitor` definition in `~/.asoundrc`, which uses ALSA's
PulseAudio plugin and `@DEFAULT_MONITOR@`. It follows the current output device.
The Linux device resolver strips the display-only ` (System Audio)` label
before opening the actual ALSA PCM.

Desktop launches inherit `XDG_RUNTIME_DIR` from the session. Shell diagnostics
need `XDG_RUNTIME_DIR=/run/user/1000` to reach the workstation's PipeWire socket.

Check the monitor's recording volume as well as the speaker volume:

```sh
XDG_RUNTIME_DIR=/run/user/1000 pactl get-source-volume @DEFAULT_MONITOR@
XDG_RUNTIME_DIR=/run/user/1000 pactl set-source-volume @DEFAULT_MONITOR@ 100%
```

During the November 6 repair, the Bose monitor had a recording volume of 8%
(about -66 dB), and a known spoken phrase captured as digital silence. Restoring
the monitor to 100% made system audio audible to the recorder and restored live
transcript segments. This controls the monitor's recording level separately
from the headphone playback volume.

## Check local inference

Generate a known spoken phrase with `espeak-ng`, then convert it with FFmpeg:

```sh
ffmpeg -i phrase.wav -ar 16000 -ac 1 -f f32le phrase.f32
XDG_RUNTIME_DIR=/run/user/1000 cargo run --locked --release -p meetily \
  --features vulkan --example transcription_check -- \
  "$HOME/.local/share/com.meetily.ai/models" phrase.f32
```

The check opens the configured ALSA microphone and monitor, runs the voice
activity detector, and requires a nonempty transcript from the existing
`large-v3` model. A desktop recording test separately verifies capture through
the app, visible transcript updates, and saved meeting data.
