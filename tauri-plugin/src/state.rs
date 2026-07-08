use std::sync::Arc;

use st_tts::core::{Style, TtsEngine};
use st_tts::store::ModelStore;
use tokio::sync::RwLock;

pub struct TtsState {
    pub engine: RwLock<Option<Arc<TtsEngine>>>,
    pub current_style: RwLock<Option<Style>>,
    pub current_voice_name: RwLock<String>,
    pub model_store: RwLock<Option<Arc<ModelStore>>>,
}

impl TtsState {
    pub fn new() -> Self {
        TtsState {
            engine: RwLock::new(None),
            current_style: RwLock::new(None),
            current_voice_name: RwLock::new("M1".to_string()),
            model_store: RwLock::new(None),
        }
    }

    pub async fn set_engine(&self, engine: Arc<TtsEngine>) {
        *self.engine.write().await = Some(engine);
    }

    pub async fn get_engine(&self) -> Result<Arc<TtsEngine>, anyhow::Error> {
        self.engine
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("TTS engine not loaded. Call load_model first."))
    }

    pub async fn set_style(&self, style: Style, voice_name: String) {
        *self.current_style.write().await = Some(style);
        *self.current_voice_name.write().await = voice_name;
    }

    pub async fn get_style(&self) -> Result<Style, anyhow::Error> {
        self.current_style
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No voice style loaded."))
    }

    pub async fn set_model_store(&self, store: Arc<ModelStore>) {
        *self.model_store.write().await = Some(store);
    }

    pub async fn get_model_store(&self) -> Result<Arc<ModelStore>, anyhow::Error> {
        self.model_store
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Model store not initialized."))
    }
}

impl Default for TtsState {
    fn default() -> Self {
        Self::new()
    }
}
