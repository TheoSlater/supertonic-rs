use std::sync::Mutex;

use async_trait::async_trait;
use ndarray::Array;
use ort::session::Session;
use ort::value::Value;

use supertonic_core::{Config, InferenceEngine, TensorValue, load_cfgs};

pub struct OrtEngine {
    dp_session: Mutex<Session>,
    text_enc_session: Mutex<Session>,
    vector_est_session: Mutex<Session>,
    vocoder_session: Mutex<Session>,
    config: Config,
}

impl OrtEngine {
    pub fn load(onnx_dir: &std::path::Path) -> Result<Self, anyhow::Error> {
        let cfgs = load_cfgs(onnx_dir)?;

        tracing::info!("Loading ONNX models from {}", onnx_dir.display());

        let dp_session = Mutex::new(Session::builder()?.commit_from_file(
            onnx_dir.join("duration_predictor.onnx"),
        )?);
        let text_enc_session = Mutex::new(Session::builder()?.commit_from_file(
            onnx_dir.join("text_encoder.onnx"),
        )?);
        let vector_est_session = Mutex::new(Session::builder()?.commit_from_file(
            onnx_dir.join("vector_estimator.onnx"),
        )?);
        let vocoder_session = Mutex::new(Session::builder()?.commit_from_file(
            onnx_dir.join("vocoder.onnx"),
        )?);

        tracing::info!("All 4 ONNX sessions created successfully");

        Ok(OrtEngine {
            dp_session,
            text_enc_session,
            vector_est_session,
            vocoder_session,
            config: cfgs,
        })
    }
}

fn tensor_to_ort(tensor: &TensorValue) -> Result<Value, anyhow::Error> {
    match tensor {
        TensorValue::F32(arr) => Ok(Value::from_array(arr.clone())?.into()),
        TensorValue::I64(arr) => Ok(Value::from_array(arr.clone())?.into()),
    }
}

fn extract_f32_tensor(outputs: &ort::session::SessionOutputs, key: &str) -> Result<TensorValue, anyhow::Error> {
    let (shape, data) = outputs[key].try_extract_tensor::<f32>()?;
    let dims: Vec<usize> = shape.iter().map(|d| *d as usize).collect();
    let arr: TensorValue = Array::from_shape_vec(dims, data.to_vec())?.into_dyn().into();
    Ok(arr)
}

#[async_trait]
impl InferenceEngine for OrtEngine {
    fn config(&self) -> &Config {
        &self.config
    }

    async fn predict_duration(
        &self,
        text_ids: &TensorValue,
        style_dp: &TensorValue,
        text_mask: &TensorValue,
    ) -> Result<TensorValue, anyhow::Error> {
        let text_ids_v = tensor_to_ort(text_ids)?;
        let style_dp_v = tensor_to_ort(style_dp)?;
        let text_mask_v = tensor_to_ort(text_mask)?;
        let mut session = self.dp_session.lock().unwrap();

        let outputs = session.run(ort::inputs! {
            "text_ids" => &text_ids_v,
            "style_dp" => &style_dp_v,
            "text_mask" => &text_mask_v,
        })?;

        extract_f32_tensor(&outputs, "duration")
    }

    async fn encode_text(
        &self,
        text_ids: &TensorValue,
        style_ttl: &TensorValue,
        text_mask: &TensorValue,
    ) -> Result<TensorValue, anyhow::Error> {
        let text_ids_v = tensor_to_ort(text_ids)?;
        let style_ttl_v = tensor_to_ort(style_ttl)?;
        let text_mask_v = tensor_to_ort(text_mask)?;
        let mut session = self.text_enc_session.lock().unwrap();

        let outputs = session.run(ort::inputs! {
            "text_ids" => &text_ids_v,
            "style_ttl" => &style_ttl_v,
            "text_mask" => &text_mask_v,
        })?;

        extract_f32_tensor(&outputs, "text_emb")
    }

    async fn estimate_vector(
        &self,
        noisy_latent: &TensorValue,
        text_emb: &TensorValue,
        style_ttl: &TensorValue,
        latent_mask: &TensorValue,
        text_mask: &TensorValue,
        current_step: &TensorValue,
        total_step: &TensorValue,
    ) -> Result<TensorValue, anyhow::Error> {
        let noisy_latent_v = tensor_to_ort(noisy_latent)?;
        let text_emb_v = tensor_to_ort(text_emb)?;
        let style_ttl_v = tensor_to_ort(style_ttl)?;
        let latent_mask_v = tensor_to_ort(latent_mask)?;
        let text_mask_v = tensor_to_ort(text_mask)?;
        let current_step_v = tensor_to_ort(current_step)?;
        let total_step_v = tensor_to_ort(total_step)?;
        let mut session = self.vector_est_session.lock().unwrap();

        let outputs = session.run(ort::inputs! {
            "noisy_latent" => &noisy_latent_v,
            "text_emb" => &text_emb_v,
            "style_ttl" => &style_ttl_v,
            "latent_mask" => &latent_mask_v,
            "text_mask" => &text_mask_v,
            "current_step" => &current_step_v,
            "total_step" => &total_step_v,
        })?;

        extract_f32_tensor(&outputs, "denoised_latent")
    }

    async fn vocode(
        &self,
        latent: &TensorValue,
    ) -> Result<TensorValue, anyhow::Error> {
        let latent_v = tensor_to_ort(latent)?;
        let mut session = self.vocoder_session.lock().unwrap();

        let outputs = session.run(ort::inputs! {
            "latent" => &latent_v,
        })?;

        extract_f32_tensor(&outputs, "wav_tts")
    }
}
