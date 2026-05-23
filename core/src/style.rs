use ndarray::Array3;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceStyleData {
    pub style_ttl: StyleComponent,
    pub style_dp: StyleComponent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleComponent {
    pub data: Vec<Vec<Vec<f32>>>,
    pub dims: Vec<usize>,
    #[serde(rename = "type")]
    pub dtype: String,
}

#[derive(Debug, Clone)]
pub struct Style {
    pub ttl: Array3<f32>,
    pub dp: Array3<f32>,
}

pub fn load_voice_style(paths: &[String]) -> Result<Style, anyhow::Error> {
    let bsz = paths.len();
    let first_file = File::open(&paths[0])?;
    let first_reader = BufReader::new(first_file);
    let first_data: VoiceStyleData = serde_json::from_reader(first_reader)?;

    let ttl_dims = &first_data.style_ttl.dims;
    let dp_dims = &first_data.style_dp.dims;

    let ttl_dim1 = ttl_dims[1];
    let ttl_dim2 = ttl_dims[2];
    let dp_dim1 = dp_dims[1];
    let dp_dim2 = dp_dims[2];

    let ttl_size = bsz * ttl_dim1 * ttl_dim2;
    let dp_size = bsz * dp_dim1 * dp_dim2;
    let mut ttl_flat = vec![0.0f32; ttl_size];
    let mut dp_flat = vec![0.0f32; dp_size];

    for (i, path) in paths.iter().enumerate() {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let data: VoiceStyleData = serde_json::from_reader(reader)?;

        let ttl_offset = i * ttl_dim1 * ttl_dim2;
        let mut idx = 0;
        for batch in &data.style_ttl.data {
            for row in batch {
                for &val in row {
                    ttl_flat[ttl_offset + idx] = val;
                    idx += 1;
                }
            }
        }

        let dp_offset = i * dp_dim1 * dp_dim2;
        idx = 0;
        for batch in &data.style_dp.data {
            for row in batch {
                for &val in row {
                    dp_flat[dp_offset + idx] = val;
                    idx += 1;
                }
            }
        }
    }

    let ttl = Array3::from_shape_vec((bsz, ttl_dim1, ttl_dim2), ttl_flat)?;
    let dp = Array3::from_shape_vec((bsz, dp_dim1, dp_dim2), dp_flat)?;

    Ok(Style { ttl, dp })
}

impl Style {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, anyhow::Error> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        load_voice_style(&[path_str])
    }
}
