use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const HF_BASE: &str = "https://huggingface.co";

static REQUIRED_FILES: &[&str] = &[
    "onnx/duration_predictor.onnx",
    "onnx/text_encoder.onnx",
    "onnx/vector_estimator.onnx",
    "onnx/vocoder.onnx",
    "onnx/tts.json",
    "onnx/unicode_indexer.json",
];

static VOICE_STYLE_FILES: &[&str] = &[
    "voice_styles/F1.json",
    "voice_styles/F2.json",
    "voice_styles/F3.json",
    "voice_styles/F4.json",
    "voice_styles/F5.json",
    "voice_styles/M1.json",
    "voice_styles/M2.json",
    "voice_styles/M3.json",
    "voice_styles/M4.json",
    "voice_styles/M5.json",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub sha256: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManifest {
    pub model_id: String,
    pub version: String,
    pub files: Vec<FileEntry>,
}

impl ModelManifest {
    pub fn file_map(&self) -> HashMap<&str, &FileEntry> {
        self.files
            .iter()
            .map(|f| {
                let key = f.path.split('/').last().unwrap_or(&f.path);
                (key, f)
            })
            .collect()
    }
}

pub enum ModelSource {
    HuggingFace(String),
    Local(PathBuf),
}

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub file: String,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub phase: DownloadPhase,
}

#[derive(Debug, Clone)]
pub enum DownloadPhase {
    Resolving,
    Downloading,
    Verifying,
    Complete,
}

pub struct ModelStore {
    cache_dir: PathBuf,
    client: reqwest::Client,
}

impl ModelStore {
    pub fn new(cache_dir: PathBuf) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("supertonic-rs/0.1.0")
            .build()
            .expect("Failed to create HTTP client");

        ModelStore { cache_dir, client }
    }

    pub fn with_cache_root(cache_root: PathBuf, model_id: &str) -> Self {
        let cache_dir = cache_root.join("models").join(model_id.replace('/', "--"));
        ModelStore::new(cache_dir)
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn resolve_model_dir(&self) -> PathBuf {
        self.cache_dir.clone()
    }

    pub fn is_cached(&self) -> bool {
        if !self.cache_dir.exists() {
            return false;
        }
        let onnx_ok = REQUIRED_FILES.iter().all(|f| self.cache_dir.join(f).exists());
        let voices_ok = VOICE_STYLE_FILES.iter().all(|f| self.cache_dir.join(f).exists());
        onnx_ok && voices_ok
    }

    pub async fn ensure_downloaded(
        &self,
        model_id: &str,
        on_progress: Option<Arc<dyn Fn(DownloadProgress) + Send + Sync>>,
    ) -> Result<PathBuf, anyhow::Error> {
        tokio::fs::create_dir_all(&self.cache_dir).await?;

        if self.is_cached() {
            return Ok(self.cache_dir.clone());
        }

        let all_files: Vec<&str> = REQUIRED_FILES
            .iter()
            .chain(VOICE_STYLE_FILES.iter())
            .copied()
            .collect();

        for rel_path in &all_files {
            let url = format!("{}/{}/resolve/main/{}", HF_BASE, model_id, rel_path);
            let dest = self.cache_dir.join(rel_path);

            if dest.exists() {
                continue;
            }

            if let Some(parent) = dest.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            if let Some(ref cb) = on_progress {
                cb(DownloadProgress {
                    file: rel_path.to_string(),
                    bytes_downloaded: 0,
                    total_bytes: None,
                    phase: DownloadPhase::Downloading,
                });
            }

            let response = self.client.get(&url).send().await?;
            let status = response.status();
            if !status.is_success() {
                anyhow::bail!(
                    "Failed to download {}: HTTP {} (model files may not exist at this URL)",
                    rel_path,
                    status
                );
            }

            let total = response.content_length();
            let mut downloaded: u64 = 0;
            let mut bytes = Vec::new();
            let mut stream = response.bytes_stream();
            let mut last_progress = std::time::Instant::now();
            let mut last_bytes: u64 = 0;

            use tokio_stream::StreamExt;
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                downloaded += chunk.len() as u64;
                bytes.extend_from_slice(&chunk);

                if let Some(ref cb) = on_progress {
                    let elapsed = last_progress.elapsed();
                    let delta = downloaded - last_bytes;
                    if elapsed.as_millis() >= 500 && delta >= 1024 * 100 {
                        cb(DownloadProgress {
                            file: rel_path.to_string(),
                            bytes_downloaded: downloaded,
                            total_bytes: total,
                            phase: DownloadPhase::Downloading,
                        });
                        last_progress = std::time::Instant::now();
                        last_bytes = downloaded;
                    }
                }
            }

            if let Some(ref cb) = on_progress {
                cb(DownloadProgress {
                    file: rel_path.to_string(),
                    bytes_downloaded: downloaded,
                    total_bytes: total,
                    phase: DownloadPhase::Complete,
                });
            }

            tokio::fs::write(&dest, &bytes).await?;

            if let Some(ref cb) = on_progress {
                cb(DownloadProgress {
                    file: rel_path.to_string(),
                    bytes_downloaded: downloaded,
                    total_bytes: total,
                    phase: DownloadPhase::Complete,
                });
            }
        }

        Ok(self.cache_dir.clone())
    }

    pub fn verify_integrity(&self, manifest: &ModelManifest) -> Result<bool, anyhow::Error> {
        for entry in &manifest.files {
            let path = self.cache_dir.join(&entry.path);
            if !path.exists() {
                tracing::warn!("Missing file: {}", entry.path);
                return Ok(false);
            }

            if let Some(expected_sha) = &entry.sha256 {
                let actual = compute_sha256(&path)?;
                if &actual != expected_sha {
                    tracing::warn!(
                        "Checksum mismatch for {}: expected {}, got {}",
                        entry.path,
                        expected_sha,
                        actual
                    );
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    pub fn prepare_load(&self) -> Result<PathBuf, anyhow::Error> {
        let onnx_dir = self.cache_dir.join("onnx");
        if !onnx_dir.exists() {
            anyhow::bail!(
                "ONNX model directory not found at {}. Call ensure_downloaded() first.",
                onnx_dir.display()
            );
        }
        Ok(onnx_dir)
    }

    pub fn resolve_voice_style(&self, voice_name: &str) -> PathBuf {
        let base = self.cache_dir.join("voice_styles");
        let path = base.join(format!("{}.json", voice_name));
        if path.exists() {
            return path;
        }
        base.join(format!("{}.json", voice_name))
    }

    pub async fn list_voices(&self) -> Result<Vec<String>, anyhow::Error> {
        let voice_dir = self.cache_dir.join("voice_styles");
        if !voice_dir.exists() {
            return Ok(Vec::new());
        }
        let mut voices = Vec::new();
        let mut entries = tokio::fs::read_dir(&voice_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "json" {
                    if let Some(stem) = path.file_stem() {
                        voices.push(stem.to_string_lossy().to_string());
                    }
                }
            }
        }
        voices.sort();
        Ok(voices)
    }

    pub fn clear_cache(&self) -> Result<(), anyhow::Error> {
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)?;
        }
        Ok(())
    }
}

pub fn compute_sha256(path: &Path) -> Result<String, anyhow::Error> {
    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sha256() {
        let dir = std::env::temp_dir().join("supertonic-test-sha256");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.txt");
        std::fs::write(&path, b"hello world").unwrap();

        let hash = compute_sha256(&path).unwrap();
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
