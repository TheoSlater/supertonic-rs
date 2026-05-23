# supertonic-rs — On-Device Neural TTS for Rust

A Rust workspace providing high-quality, on-device text-to-speech via Supertonic's 99M-parameter model running on ONNX Runtime. No cloud, no API calls, no GPU required.

## Crates

| Crate | Description | crates.io |
|-------|-------------|-----------|
| [`supertonic`](supertonic/) | High-level API — one-liner synthesize | [![crates.io](https://img.shields.io/crates/v/supertonic.svg)](https://crates.io/crates/supertonic) |
| [`supertonic-core`](core/) | Engine-agnostic TTS pipeline (text, audio, style) | [![crates.io](https://img.shields.io/crates/v/supertonic-core.svg)](https://crates.io/crates/supertonic-core) |
| [`supertonic-ort-backend`](ort-backend/) | ONNX Runtime backend | [![crates.io](https://img.shields.io/crates/v/supertonic-ort-backend.svg)](https://crates.io/crates/supertonic-ort-backend) |
| [`supertonic-model-store`](model-store/) | Model download & caching from HuggingFace | [![crates.io](https://img.shields.io/crates/v/supertonic-model-store.svg)](https://crates.io/crates/supertonic-model-store) |
| [`tauri-plugin-supertonic`](tauri-plugin/) | Tauri v2 plugin for desktop apps | [![crates.io](https://img.shields.io/crates/v/tauri-plugin-supertonic.svg)](https://crates.io/crates/tauri-plugin-supertonic) |

## Quick Start

```rust
use supertonic::Tts;

// Auto-download model from HuggingFace, then synthesize
let tts = Tts::new("Supertone/supertonic-3", "M1").await?;

// Get WAV bytes
let wav: Vec<u8> = tts.synthesize_wav("Hello world", "en", None).await?;

// Or get raw PCM samples
let result = tts.synthesize("Hello world", "en", None).await?;
// result.audio: Vec<f32>, result.sample_rate: u32
```

## Tauri Integration

```rust
// src-tauri/src/lib.rs
.plugin(tauri_plugin_supertonic::init())
```

```typescript
// Frontend: invoke the plugin
const { wavBase64 } = await invoke("plugin:supertonic|synthesize", {
  text: "Hello world",
  lang: "en",
});
new Audio(`data:audio/wav;base64,${wavBase64}`).play();
```

## Publishing Order

These crates have inter-dependencies. Publish in this order:

1. `supertonic-core`
2. `supertonic-ort-backend` + `supertonic-model-store` (parallel)
3. `supertonic`
4. `tauri-plugin-supertonic`

```bash
cargo publish -p supertonic-core
cargo publish -p supertonic-ort-backend
cargo publish -p supertonic-model-store
cargo publish -p supertonic
cargo publish -p tauri-plugin-supertonic
```
