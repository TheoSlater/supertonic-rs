use async_trait::async_trait;

use crate::config::Config;

#[derive(Debug, Clone)]
pub enum TensorValue {
    F32(ndarray::Array<f32, ndarray::IxDyn>),
    I64(ndarray::Array<i64, ndarray::IxDyn>),
}

impl TensorValue {
    pub fn as_f32(&self) -> Option<&ndarray::Array<f32, ndarray::IxDyn>> {
        match self {
            TensorValue::F32(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn into_f32(self) -> Option<ndarray::Array<f32, ndarray::IxDyn>> {
        match self {
            TensorValue::F32(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn shape(&self) -> &[usize] {
        match self {
            TensorValue::F32(arr) => arr.shape(),
            TensorValue::I64(arr) => arr.shape(),
        }
    }
}

impl From<ndarray::Array<f32, ndarray::IxDyn>> for TensorValue {
    fn from(arr: ndarray::Array<f32, ndarray::IxDyn>) -> Self {
        TensorValue::F32(arr)
    }
}

impl From<ndarray::Array<i64, ndarray::IxDyn>> for TensorValue {
    fn from(arr: ndarray::Array<i64, ndarray::IxDyn>) -> Self {
        TensorValue::I64(arr)
    }
}

#[derive(Debug, Clone)]
pub struct SynthesisParams {
    pub total_step: usize,
    pub speed: f32,
    pub silence_duration: f32,
    pub rng_seed: Option<u64>,
}

impl Default for SynthesisParams {
    fn default() -> Self {
        SynthesisParams {
            total_step: 8,
            speed: 1.05,
            silence_duration: 0.3,
            rng_seed: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SynthesisResult {
    pub audio: Vec<f32>,
    pub duration_secs: f32,
    pub sample_rate: u32,
}

#[derive(Debug, Clone)]
pub struct ChunkResult {
    pub audio: Vec<f32>,
    pub duration_secs: f32,
    pub chunk_index: usize,
    pub is_last: bool,
}

#[async_trait]
pub trait InferenceEngine: Send + Sync {
    async fn predict_duration(
        &self,
        text_ids: &TensorValue,
        style_dp: &TensorValue,
        text_mask: &TensorValue,
    ) -> Result<TensorValue, anyhow::Error>;

    async fn encode_text(
        &self,
        text_ids: &TensorValue,
        style_ttl: &TensorValue,
        text_mask: &TensorValue,
    ) -> Result<TensorValue, anyhow::Error>;

    async fn estimate_vector(
        &self,
        noisy_latent: &TensorValue,
        text_emb: &TensorValue,
        style_ttl: &TensorValue,
        latent_mask: &TensorValue,
        text_mask: &TensorValue,
        current_step: &TensorValue,
        total_step: &TensorValue,
    ) -> Result<TensorValue, anyhow::Error>;

    async fn vocode(
        &self,
        latent: &TensorValue,
    ) -> Result<TensorValue, anyhow::Error>;

    fn config(&self) -> &Config;
}
