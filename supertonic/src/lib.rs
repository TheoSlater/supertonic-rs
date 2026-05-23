use std::path::{Path, PathBuf};
use std::sync::Arc;

use supertonic_core::{
    encode_wav_bytes, load_voice_style, Style, SynthesisResult, TtsEngine, UnicodeProcessor,
};
use supertonic_model_store::ModelStore;
use supertonic_ort_backend::OrtEngine;

/// Simple high-level TTS interface.
///
/// # Examples
///
/// ```ignore
/// use supertonic::Tts;
///
/// // Auto-download model from HuggingFace, then synthesize
/// let tts = Tts::new("Supertone/supertonic-3", "M1").await?;
/// let wav = tts.synthesize_wav("Hello world", "en").await?;
/// ```
pub struct Tts {
    engine: Arc<TtsEngine>,
    style: Style,
    model_store: Option<Arc<ModelStore>>,
}

impl Tts {
    /// Download model from HuggingFace (if needed) and load it.
    pub async fn new(model_id: &str, voice_name: &str) -> Result<Self, anyhow::Error> {
        let cache_root = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("supertonic");

        let store = Arc::new(ModelStore::with_cache_root(cache_root, model_id));
        store.ensure_downloaded(model_id, None).await?;
        let onnx_dir = store.prepare_load()?;

        let engine = load_engine(&onnx_dir)?;
        let style = load_style(&store, voice_name)?;

        Ok(Tts {
            engine,
            style,
            model_store: Some(store),
        })
    }

    /// Load from a local ONNX directory and voice style file.
    pub fn from_local(
        onnx_dir: impl AsRef<Path>,
        voice_style_path: impl AsRef<Path>,
    ) -> Result<Self, anyhow::Error> {
        let engine = load_engine(onnx_dir.as_ref())?;
        let path_str = voice_style_path.as_ref().to_string_lossy().to_string();
        let style = load_voice_style(&[path_str])?;
        Ok(Tts {
            engine,
            style,
            model_store: None,
        })
    }

    /// Synthesize text → PCM samples.
    pub async fn synthesize(
        &self,
        text: &str,
        lang: &str,
        params: Option<&SynthesisParams>,
    ) -> Result<SynthesisResult, anyhow::Error> {
        let default_params = SynthesisParams::default();
        let params = params.unwrap_or(&default_params);
        self.engine.synthesize(text, lang, &self.style, params).await
    }

    /// Synthesize text → WAV bytes (ready to save or stream).
    pub async fn synthesize_wav(
        &self,
        text: &str,
        lang: &str,
        params: Option<&SynthesisParams>,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let result = self.synthesize(text, lang, params).await?;
        encode_wav_bytes(&result.audio, result.sample_rate)
    }

    /// The loaded style (for inspection / modification).
    pub fn style(&self) -> &Style {
        &self.style
    }

    /// Select a different voice from the cached model store.
    pub fn select_voice(&mut self, voice_name: &str) -> Result<(), anyhow::Error> {
        let store = self
            .model_store
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No model store (loaded from local path)"))?;
        self.style = load_style(store, voice_name)?;
        Ok(())
    }

    /// Sample rate of the loaded model.
    pub fn sample_rate(&self) -> u32 {
        self.engine.sample_rate()
    }

    /// List available voice styles (only works if loaded via `new`).
    pub async fn list_voices(&self) -> Result<Vec<String>, anyhow::Error> {
        let store = self
            .model_store
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No model store (loaded from local path)"))?;
        store.list_voices().await
    }
}

fn load_engine(onnx_dir: &Path) -> Result<Arc<TtsEngine>, anyhow::Error> {
    let ort_engine = OrtEngine::load(onnx_dir)?;
    let indexer_path = onnx_dir.join("unicode_indexer.json");
    let text_processor = UnicodeProcessor::new(&indexer_path)?;
    Ok(Arc::new(TtsEngine::new(Arc::new(ort_engine), text_processor)))
}

fn load_style(store: &ModelStore, voice_name: &str) -> Result<Style, anyhow::Error> {
    let path = store.resolve_voice_style(voice_name);
    let path_str = path.to_string_lossy().to_string();
    load_voice_style(&[path_str])
}

// Re-exports for convenience
pub use supertonic_core::SynthesisParams;
