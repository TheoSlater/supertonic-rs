use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ae: AeConfig,
    pub ttl: TtlConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AeConfig {
    pub sample_rate: i32,
    pub base_chunk_size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtlConfig {
    pub chunk_compress_factor: i32,
    pub latent_dim: i32,
}

pub fn load_cfgs<P: AsRef<Path>>(onnx_dir: P) -> Result<Config, anyhow::Error> {
    let cfg_path = onnx_dir.as_ref().join("tts.json");
    let file = File::open(cfg_path)?;
    let reader = BufReader::new(file);
    let cfgs: Config = serde_json::from_reader(reader)?;
    Ok(cfgs)
}
