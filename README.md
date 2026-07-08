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

## Quick Start

```rust
use st_tts::Tts;

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

`tauri-plugin-supertonic` depends on `st-tts`. Publish in this order:

```bash
cargo publish -p st-tts
cargo publish -p tauri-plugin-supertonic
```
