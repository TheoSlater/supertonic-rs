use std::sync::Arc;

use base64::Engine;
use serde::Serialize;
use tauri::ipc::Channel;

use st_tts::store::ModelStore;

use st_tts::core::{SynthesisParams, TtsEngine};
use st_tts::backend::OrtEngine;

use crate::state::TtsState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthesizeResponse {
    pub wav_base64: String,
    pub duration_secs: f32,
    pub sample_rate: u32,
}

#[tauri::command]
pub async fn synthesize(
    state: tauri::State<'_, TtsState>,
    text: String,
    lang: Option<String>,
    total_step: Option<usize>,
    speed: Option<f32>,
    silence_duration: Option<f32>,
) -> Result<SynthesizeResponse, String> {
    let engine = state.get_engine().await.map_err(|e| e.to_string())?;
    let style = state.get_style().await.map_err(|e| e.to_string())?;

    let mut params = SynthesisParams::default();
    if let Some(ts) = total_step {
        params.total_step = ts;
    }
    if let Some(sp) = speed {
        params.speed = sp;
    }
    if let Some(sd) = silence_duration {
        params.silence_duration = sd;
    }

    let lang = lang.unwrap_or_else(|| "en".to_string());

    let result = engine
        .synthesize(&text, &lang, &style, &params)
        .await
        .map_err(|e| e.to_string())?;

    let wav_bytes = st_tts::core::encode_wav_bytes(&result.audio, result.sample_rate)
        .map_err(|e| e.to_string())?;

    let wav_base64 = base64::engine::general_purpose::STANDARD.encode(&wav_bytes);

    Ok(SynthesizeResponse {
        wav_base64,
        duration_secs: result.duration_secs,
        sample_rate: result.sample_rate,
    })
}

#[tauri::command]
pub async fn load_model(
    state: tauri::State<'_, TtsState>,
    model_id: Option<String>,
    voice_style: Option<String>,
    on_progress: Channel<ModelProgressEvent>,
) -> Result<(), String> {
    let mid = model_id.unwrap_or_else(|| "Supertone/supertonic-3".to_string());
    let voice = voice_style.unwrap_or_else(|| "M1".to_string());

    let cache_root = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("supertonic");

    let store = Arc::new(ModelStore::with_cache_root(cache_root, &mid));
    state.set_model_store(store.clone()).await;

    let progress_cb: Arc<dyn Fn(st_tts::store::DownloadProgress) + Send + Sync> =
        Arc::new(move |p: st_tts::store::DownloadProgress| {
            let evt = ModelProgressEvent {
                file: p.file,
                bytes_downloaded: p.bytes_downloaded,
                total_bytes: p.total_bytes,
            };
            let _ = on_progress.send(evt);
        });

    store
        .ensure_downloaded(&mid, Some(progress_cb))
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;

    let onnx_dir = store
        .prepare_load()
        .map_err(|e: anyhow::Error| e.to_string())?;

    let ort_engine =
        OrtEngine::load(&onnx_dir).map_err(|e: anyhow::Error| e.to_string())?;

    let unicode_indexer_path = onnx_dir.join("unicode_indexer.json");
    let text_processor = st_tts::core::UnicodeProcessor::new(&unicode_indexer_path)
        .map_err(|e: anyhow::Error| e.to_string())?;

    let engine = Arc::new(TtsEngine::new(Arc::new(ort_engine), text_processor));
    state.set_engine(engine).await;

    let voice_path = store.resolve_voice_style(&voice);
    if voice_path.exists() {
        let style =
            st_tts::core::load_voice_style(&[voice_path.to_string_lossy().to_string()])
                .map_err(|e: anyhow::Error| e.to_string())?;
        state.set_style(style, voice.clone()).await;
    }

    Ok(())
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelProgressEvent {
    pub file: String,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
}

#[tauri::command]
pub async fn list_voices(
    state: tauri::State<'_, TtsState>,
) -> Result<Vec<String>, String> {
    let store = state.get_model_store().await.map_err(|e| e.to_string())?;
    store.list_voices().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn select_voice(
    state: tauri::State<'_, TtsState>,
    voice_name: String,
) -> Result<(), String> {
    let store = state.get_model_store().await.map_err(|e| e.to_string())?;
    let voice_path = store.resolve_voice_style(&voice_name);

    if !voice_path.exists() {
        return Err(format!("Voice '{}' not found", voice_name));
    }

    let style = st_tts::core::load_voice_style(&[voice_path.to_string_lossy().to_string()])
        .map_err(|e: anyhow::Error| e.to_string())?;

    state.set_style(style, voice_name).await;
    Ok(())
}

#[tauri::command]
pub async fn get_status(
    state: tauri::State<'_, TtsState>,
) -> Result<StatusResponse, String> {
    let engine_loaded = state.engine.read().await.is_some();
    let voice_name = state.current_voice_name.read().await.clone();

    let sample_rate = state
        .engine
        .read()
        .await
        .as_ref()
        .map(|e| e.sample_rate())
        .unwrap_or(44100);

    Ok(StatusResponse {
        engine_loaded,
        current_voice: voice_name,
        sample_rate,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub engine_loaded: bool,
    pub current_voice: String,
    pub sample_rate: u32,
}
