use crate::core::audio::encode_wav_bytes;
use crate::core::engine::{ChunkResult, InferenceEngine, SynthesisParams, SynthesisResult, TensorValue};
use crate::core::style::Style;
use crate::core::text::{chunk_text, max_chunk_len_for_lang, sample_noisy_latent, UnicodeProcessor};

use ndarray::{Array, Array3};
use std::sync::Arc;

#[derive(Clone)]
pub struct TtsEngine {
    engine: Arc<dyn InferenceEngine>,
    text_processor: Arc<UnicodeProcessor>,
}

impl TtsEngine {
    pub fn new(
        engine: Arc<dyn InferenceEngine>,
        text_processor: UnicodeProcessor,
    ) -> Self {
        TtsEngine {
            engine,
            text_processor: Arc::new(text_processor),
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.engine.config().ae.sample_rate as u32
    }

    pub async fn synthesize(
        &self,
        text: &str,
        lang: &str,
        style: &Style,
        params: &SynthesisParams,
    ) -> Result<SynthesisResult, anyhow::Error> {
        let max_len = max_chunk_len_for_lang(lang);
        let chunks = chunk_text(text, Some(max_len));

        let mut full_audio: Vec<f32> = Vec::new();
        let mut total_duration: f32 = 0.0;

        for (i, chunk) in chunks.iter().enumerate() {
            let result = self
                .infer_single(&[chunk.clone()], &[lang.to_string()], style, params)
                .await?;

            let dur = result.duration_secs;
            let wav_len = (self.sample_rate() as f32 * dur) as usize;
            let wav_chunk = &result.audio[..wav_len.min(result.audio.len())];

            if i == 0 {
                full_audio.extend_from_slice(wav_chunk);
                total_duration = dur;
            } else {
                let silence_len = (params.silence_duration * self.sample_rate() as f32) as usize;
                full_audio.extend(std::iter::repeat(0.0f32).take(silence_len));
                full_audio.extend_from_slice(wav_chunk);
                total_duration += params.silence_duration + dur;
            }
        }

        Ok(SynthesisResult {
            audio: full_audio,
            duration_secs: total_duration,
            sample_rate: self.sample_rate(),
        })
    }

    pub async fn synthesize_stream(
        &self,
        text: &str,
        lang: &str,
        style: &Style,
        params: &SynthesisParams,
        on_chunk: impl FnMut(ChunkResult) -> Result<(), anyhow::Error>,
    ) -> Result<(), anyhow::Error> {
        let max_len = max_chunk_len_for_lang(lang);
        let chunks = chunk_text(text, Some(max_len));
        let mut on_chunk = on_chunk;
        let total = chunks.len();

        for (i, chunk) in chunks.iter().enumerate() {
            let result = self
                .infer_single(&[chunk.clone()], &[lang.to_string()], style, params)
                .await?;

            let dur = result.duration_secs;
            let wav_len = (self.sample_rate() as f32 * dur) as usize;
            let wav_chunk = &result.audio[..wav_len.min(result.audio.len())];

            let is_last = i == total - 1;
            on_chunk(ChunkResult {
                audio: wav_chunk.to_vec(),
                duration_secs: dur,
                chunk_index: i,
                is_last,
            })?;
        }

        Ok(())
    }

    pub async fn synthesize_wav(
        &self,
        text: &str,
        lang: &str,
        style: &Style,
        params: &SynthesisParams,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let result = self.synthesize(text, lang, style, params).await?;
        encode_wav_bytes(&result.audio, result.sample_rate)
    }

    pub async fn batch(
        &self,
        text_list: &[String],
        lang_list: &[String],
        style: &Style,
        params: &SynthesisParams,
    ) -> Result<SynthesisResult, anyhow::Error> {
        self.infer_single(text_list, lang_list, style, params).await
    }

    async fn infer_single(
        &self,
        text_list: &[String],
        lang_list: &[String],
        style: &Style,
        params: &SynthesisParams,
    ) -> Result<SynthesisResult, anyhow::Error> {
        let bsz = text_list.len();
        let cfg = &self.engine.config().ae;
        let ttl_cfg = &self.engine.config().ttl;

        let (text_ids, text_mask) = self.text_processor.process(text_list, lang_list)?;

        let text_ids_shape = (bsz, text_ids[0].len());
        let mut flat = Vec::new();
        for row in &text_ids {
            flat.extend_from_slice(row);
        }
        let text_ids_array: Array<i64, _> = Array::from_shape_vec(text_ids_shape, flat)?.into_dyn();
        let text_ids_t: TensorValue = text_ids_array.into();
        let text_mask_t: TensorValue = text_mask.clone().into_dyn().into();
        let style_dp_t: TensorValue = style.dp.clone().into_dyn().into();

        let duration_out = self
            .engine
            .predict_duration(&text_ids_t, &style_dp_t, &text_mask_t)
            .await?;

        let duration_f32 = duration_out
            .as_f32()
            .ok_or_else(|| anyhow::anyhow!("duration output must be f32"))?;
        let duration_data = duration_f32
            .as_slice()
            .ok_or_else(|| anyhow::anyhow!("duration output not contiguous"))?
            .to_vec();

        let mut duration: Vec<f32> = duration_data;
        for dur in duration.iter_mut() {
            *dur /= params.speed;
        }

        let style_ttl_t: TensorValue = style.ttl.clone().into_dyn().into();

        let text_emb_out = self
            .engine
            .encode_text(&text_ids_t, &style_ttl_t, &text_mask_t)
            .await?;

        let (mut xt, latent_mask) = sample_noisy_latent(
            &duration,
            cfg.sample_rate,
            cfg.base_chunk_size,
            ttl_cfg.chunk_compress_factor,
            ttl_cfg.latent_dim,
            params.rng_seed,
        );

        let total_step_arr: TensorValue = Array::from_elem(bsz, params.total_step as f32).into_dyn().into();

        for step in 0..params.total_step {
            let current_step_arr: TensorValue =
                Array::from_elem(bsz, step as f32).into_dyn().into();

            let xt_t: TensorValue = xt.clone().into_dyn().into();
            let text_emb_t = text_emb_out.clone();
            let latent_mask_t: TensorValue = latent_mask.clone().into_dyn().into();
            let text_mask_t2: TensorValue = text_mask.clone().into_dyn().into();

            let denoised = self
                .engine
                .estimate_vector(
                    &xt_t,
                    &text_emb_t,
                    &style_ttl_t,
                    &latent_mask_t,
                    &text_mask_t2,
                    &current_step_arr,
                    &total_step_arr,
                )
                .await?;

            let denoised_f32 = denoised
                .as_f32()
                .ok_or_else(|| anyhow::anyhow!("denoised output must be f32"))?;

            let shape = denoised_f32.shape().to_vec();
            let data = denoised_f32
                .as_slice()
                .ok_or_else(|| anyhow::anyhow!("denoised output not contiguous"))?;
            xt = Array3::from_shape_vec(
                (shape[0], shape[1], shape[2]),
                data.to_vec(),
            )?;
        }

        let final_latent_t: TensorValue = xt.into_dyn().into();
        let wav_out = self.engine.vocode(&final_latent_t).await?;

        let wav_f32 = wav_out
            .as_f32()
            .ok_or_else(|| anyhow::anyhow!("vocoder output must be f32"))?;
        let wav_data = wav_f32
            .as_slice()
            .ok_or_else(|| anyhow::anyhow!("wav output not contiguous"))?
            .to_vec();

        let dur_val = duration.iter().sum::<f32>() / bsz as f32;

        Ok(SynthesisResult {
            audio: wav_data,
            duration_secs: dur_val,
            sample_rate: cfg.sample_rate as u32,
        })
    }
}
