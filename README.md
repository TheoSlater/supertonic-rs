<p align="center">
  <a href="https://deepwiki.com/TheoSlater/supertonic-rs"><img src="https://deepwiki.com/badge.svg" alt="Ask DeepWiki"></a>
</p>

# supertonic-rs — On-Device Neural TTS for Rust

A Rust workspace providing high-quality, on-device text-to-speech via Supertonic's 99M-parameter model running on ONNX Runtime. No cloud, no API calls, no GPU required.

## Crates

| Crate | Description | crates.io |
|-------|-------------|-----------|
| [`st-tts`](supertonic/) | High-level API — one-liner synthesize, plus `core`/`backend`/`store` modules for lower-level access | [![crates.io](https://img.shields.io/crates/v/st-tts.svg)](https://crates.io/crates/st-tts) |
| [`tauri-plugin-supertonic`](tauri-plugin/) | Tauri v2 plugin for desktop apps | [![crates.io](https://img.shields.io/crates/v/tauri-plugin-supertonic.svg)](https://crates.io/crates/tauri-plugin-supertonic) |

## Thank you Traun Leyden for the wiki:
https://deepwiki.com/TheoSlater/supertonic-rs/1-overview

## Use `st-tts` in a Rust project

Install:

```bash
cargo add st-tts
cargo add tokio --features macros,rt-multi-thread
cargo add anyhow
```

Minimal app:

```rust
use st_tts::Tts;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // First run downloads the model into the OS data directory.
    let tts = Tts::new("Supertone/supertonic-3", "M1").await?;

    let wav = tts.synthesize_wav("Hello world", "en", None).await?;
    std::fs::write("hello.wav", wav)?;

    Ok(())
}
```

Need raw PCM instead of WAV:

```rust
let result = tts.synthesize("Hello world", "en", None).await?;
// result.audio: Vec<f32>
// result.sample_rate: u32
```

## Use in a Tauri v2 project

Add the Rust plugin:

```bash
cd src-tauri
cargo add tauri-plugin-supertonic
```

Register it:

```rust
// src-tauri/src/lib.rs
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_supertonic::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Allow plugin commands in your Tauri capability:

`src-tauri/capabilities/default.json`:

```json
{
  "permissions": [
    "core:default",
    "supertonic:default"
  ]
}
```

Call it from the frontend:

```bash
npm add tauri-plugin-supertonic-api
```

```typescript
import { loadModel, synthesize } from "tauri-plugin-supertonic-api";

await loadModel("Supertone/supertonic-3", "M1", (event) => {
  console.log(event.file, event.bytesDownloaded, event.totalBytes);
});

const { wavBase64 } = await synthesize("Hello world", "en");
new Audio(`data:audio/wav;base64,${wavBase64}`).play();
```

`loadModel` must run before `synthesize`. The first run downloads the model; later runs reuse the cache.

## Use the OpenTUI CLI

The CLI is an interactive terminal app for typing text, choosing language, voice, and speed, then either hearing the generated speech or saving it as a WAV file.

Requirements:

```bash
curl -fsSL https://bun.sh/install | bash
rustup toolchain install stable
```

For playback inside the app, install one of these audio players: `pw-play`, `paplay`, `aplay`, or `ffplay`.

Run:

```bash
cd supertonic-cli
bun install
bun run src/index.tsx
```

Controls:

```text
Tab          move between fields
Enter        open synthesize popup
Tab/arrows   switch between Hear in app and Save WAV file
Enter        confirm popup choice
Esc          cancel popup or quit
Ctrl+C       quit
```

The first synth downloads `Supertone/supertonic-3`. Save mode writes `supertonic-cli/supertonic.wav`; hear mode writes a temporary preview and plays it.

## Publishing Order

`tauri-plugin-supertonic` depends on `st-tts`. Publish in this order:

```bash
cargo publish -p st-tts
cargo publish -p tauri-plugin-supertonic
```
