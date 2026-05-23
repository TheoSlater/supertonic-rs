use ndarray::Array3;
use regex::Regex;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use unicode_normalization::UnicodeNormalization;

pub const AVAILABLE_LANGS: &[&str] = &[
    "en", "ko", "ja", "ar", "bg", "cs", "da", "de", "el", "es", "et", "fi",
    "fr", "hi", "hr", "hu", "id", "it", "lt", "lv", "nl", "pl", "pt", "ro",
    "ru", "sk", "sl", "sv", "tr", "uk", "vi", "na",
];

pub fn is_valid_lang(lang: &str) -> bool {
    AVAILABLE_LANGS.contains(&lang)
}

pub struct UnicodeProcessor {
    indexer: Vec<i64>,
}

impl UnicodeProcessor {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, anyhow::Error> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let indexer: Vec<i64> = serde_json::from_reader(reader)?;
        Ok(UnicodeProcessor { indexer })
    }

    pub fn process(
        &self,
        text_list: &[String],
        lang_list: &[String],
    ) -> Result<(Vec<Vec<i64>>, Array3<f32>), anyhow::Error> {
        let mut processed_texts: Vec<String> = Vec::new();
        for (text, lang) in text_list.iter().zip(lang_list.iter()) {
            processed_texts.push(preprocess_text(text, lang)?);
        }

        let text_ids_lengths: Vec<usize> = processed_texts
            .iter()
            .map(|t| t.chars().count())
            .collect();

        let max_len = *text_ids_lengths.iter().max().unwrap_or(&0);

        let mut text_ids = Vec::new();
        for text in &processed_texts {
            let mut row = vec![0i64; max_len];
            let unicode_vals = text_to_unicode_values(text);
            for (j, &val) in unicode_vals.iter().enumerate() {
                if val < self.indexer.len() {
                    row[j] = self.indexer[val];
                } else {
                    row[j] = -1;
                }
            }
            text_ids.push(row);
        }

        let text_mask = get_text_mask(&text_ids_lengths);

        Ok((text_ids, text_mask))
    }
}

pub fn preprocess_text(text: &str, lang: &str) -> Result<String, anyhow::Error> {
    let mut text: String = text.nfkd().collect();

    let emoji_pattern = Regex::new(
        r"[\x{1F600}-\x{1F64F}\x{1F300}-\x{1F5FF}\x{1F680}-\x{1F6FF}\x{1F700}-\x{1F77F}\x{1F780}-\x{1F7FF}\x{1F800}-\x{1F8FF}\x{1F900}-\x{1F9FF}\x{1FA00}-\x{1FA6F}\x{1FA70}-\x{1FAFF}\x{2600}-\x{26FF}\x{2700}-\x{27BF}\x{1F1E6}-\x{1F1FF}]+",
    )
    .unwrap();
    text = emoji_pattern.replace_all(&text, "").to_string();

    let replacements = [
        ("\u{2013}", "-"),
        ("\u{2011}", "-"),
        ("\u{2014}", "-"),
        ("_", " "),
        ("\u{201C}", "\""),
        ("\u{201D}", "\""),
        ("\u{2018}", "'"),
        ("\u{2019}", "'"),
        ("\u{00B4}", "'"),
        ("`", "'"),
        ("[", " "),
        ("]", " "),
        ("|", " "),
        ("/", " "),
        ("#", " "),
        ("\u{2192}", " "),
        ("\u{2190}", " "),
    ];

    for (from, to) in &replacements {
        text = text.replace(from, to);
    }

    let special_symbols = ["\u{2665}", "\u{2606}", "\u{2661}", "\u{00A9}", "\\"];
    for symbol in &special_symbols {
        text = text.replace(symbol, "");
    }

    let expr_replacements = [("@", " at "), ("e.g.,", "for example, "), ("i.e.,", "that is, ")];
    for (from, to) in &expr_replacements {
        text = text.replace(from, to);
    }

    text = Regex::new(r" ,").unwrap().replace_all(&text, ",").to_string();
    text = Regex::new(r" \.").unwrap().replace_all(&text, ".").to_string();
    text = Regex::new(r" !").unwrap().replace_all(&text, "!").to_string();
    text = Regex::new(r" \?").unwrap().replace_all(&text, "?").to_string();
    text = Regex::new(r" ;").unwrap().replace_all(&text, ";").to_string();
    text = Regex::new(r" :").unwrap().replace_all(&text, ":").to_string();
    text = Regex::new(r" '").unwrap().replace_all(&text, "'").to_string();

    while text.contains("\"\"") {
        text = text.replace("\"\"", "\"");
    }
    while text.contains("''") {
        text = text.replace("''", "'");
    }
    while text.contains("``") {
        text = text.replace("``", "`");
    }

    text = Regex::new(r"\s+")
        .unwrap()
        .replace_all(&text, " ")
        .to_string();
    text = text.trim().to_string();

    if !text.is_empty() {
        let ends_with_punct = Regex::new(
            r#"[.!?;:,'"\u{201C}\u{201D}\u{2018}\u{2019})\] »。』】〉》›»]$"#,
        )
        .unwrap();
        if !ends_with_punct.is_match(&text) {
            text.push('.');
        }
    }

    if !is_valid_lang(lang) {
        anyhow::bail!("Invalid language: {}. Available: {:?}", lang, AVAILABLE_LANGS);
    }

    text = format!("<{}>{}</{}>", lang, text, lang);

    Ok(text)
}

pub fn text_to_unicode_values(text: &str) -> Vec<usize> {
    text.chars().map(|c| c as usize).collect()
}

pub fn length_to_mask(lengths: &[usize], max_len: Option<usize>) -> Array3<f32> {
    let bsz = lengths.len();
    let max_len = max_len.unwrap_or_else(|| *lengths.iter().max().unwrap_or(&0));

    let mut mask = Array3::<f32>::zeros((bsz, 1, max_len));
    for (i, &len) in lengths.iter().enumerate() {
        for j in 0..len.min(max_len) {
            mask[[i, 0, j]] = 1.0;
        }
    }
    mask
}

pub fn get_text_mask(text_ids_lengths: &[usize]) -> Array3<f32> {
    let max_len = *text_ids_lengths.iter().max().unwrap_or(&0);
    length_to_mask(text_ids_lengths, Some(max_len))
}

pub fn sample_noisy_latent(
    duration: &[f32],
    sample_rate: i32,
    base_chunk_size: i32,
    chunk_compress: i32,
    latent_dim: i32,
    rng_seed: Option<u64>,
) -> (Array3<f32>, Array3<f32>) {
    let bsz = duration.len();
    let max_dur = duration.iter().fold(0.0f32, |a, &b| a.max(b));

    let wav_len_max = (max_dur * sample_rate as f32) as usize;
    let wav_lengths: Vec<usize> = duration
        .iter()
        .map(|&d| (d * sample_rate as f32) as usize)
        .collect();

    let chunk_size = (base_chunk_size * chunk_compress) as usize;
    let latent_len = (wav_len_max + chunk_size - 1) / chunk_size;
    let latent_dim_val = (latent_dim * chunk_compress) as usize;

    let mut noisy_latent = Array3::<f32>::zeros((bsz, latent_dim_val, latent_len));

    use rand::SeedableRng;
    use rand_distr::{Distribution, Normal};
    let mut rng = if let Some(seed) = rng_seed {
        rand::rngs::StdRng::seed_from_u64(seed)
    } else {
        rand::rngs::StdRng::from_entropy()
    };
    let normal = Normal::new(0.0, 1.0).unwrap();

    for b in 0..bsz {
        for d in 0..latent_dim_val {
            for t in 0..latent_len {
                noisy_latent[[b, d, t]] = normal.sample(&mut rng);
            }
        }
    }

    let latent_lengths: Vec<usize> = wav_lengths
        .iter()
        .map(|&len| (len + chunk_size - 1) / chunk_size)
        .collect();

    let latent_mask = length_to_mask(&latent_lengths, Some(latent_len));

    for b in 0..bsz {
        for d in 0..latent_dim_val {
            for t in 0..latent_len {
                noisy_latent[[b, d, t]] *= latent_mask[[b, 0, t]];
            }
        }
    }

    (noisy_latent, latent_mask)
}

const MAX_CHUNK_LENGTH: usize = 300;

const ABBREVIATIONS: &[&str] = &[
    "Dr.", "Mr.", "Mrs.", "Ms.", "Prof.", "Sr.", "Jr.", "St.", "Ave.", "Rd.",
    "Blvd.", "Dept.", "Inc.", "Ltd.", "Co.", "Corp.", "etc.", "vs.", "i.e.",
    "e.g.", "Ph.D.",
];

pub fn chunk_text(text: &str, max_len: Option<usize>) -> Vec<String> {
    let max_len = max_len.unwrap_or(MAX_CHUNK_LENGTH);
    let text = text.trim();

    if text.is_empty() {
        return vec![String::new()];
    }

    let para_re = Regex::new(r"\n\s*\n").unwrap();
    let paragraphs: Vec<&str> = para_re.split(text).collect();
    let mut chunks = Vec::new();

    for para in paragraphs {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }

        if para.len() <= max_len {
            chunks.push(para.to_string());
            continue;
        }

        let sentences = split_sentences(para);
        let mut current = String::new();
        let mut current_len = 0;

        for sentence in sentences {
            let sentence = sentence.trim();
            if sentence.is_empty() {
                continue;
            }

            let sentence_len = sentence.len();
            if sentence_len > max_len {
                if !current.is_empty() {
                    chunks.push(current.trim().to_string());
                    current.clear();
                    current_len = 0;
                }

                let parts: Vec<&str> = sentence.split(',').collect();
                for part in parts {
                    let part = part.trim();
                    if part.is_empty() {
                        continue;
                    }

                    let part_len = part.len();
                    if part_len > max_len {
                        let words: Vec<&str> = part.split_whitespace().collect();
                        let mut word_chunk = String::new();
                        let mut word_chunk_len = 0;

                        for word in words {
                            let word_len = word.len();
                            if word_chunk_len + word_len + 1 > max_len && !word_chunk.is_empty() {
                                chunks.push(word_chunk.trim().to_string());
                                word_chunk.clear();
                                word_chunk_len = 0;
                            }

                            if !word_chunk.is_empty() {
                                word_chunk.push(' ');
                                word_chunk_len += 1;
                            }
                            word_chunk.push_str(word);
                            word_chunk_len += word_len;
                        }

                        if !word_chunk.is_empty() {
                            chunks.push(word_chunk.trim().to_string());
                        }
                    } else {
                        if current_len + part_len + 1 > max_len && !current.is_empty() {
                            chunks.push(current.trim().to_string());
                            current.clear();
                            current_len = 0;
                        }

                        if !current.is_empty() {
                            current.push_str(", ");
                            current_len += 2;
                        }
                        current.push_str(part);
                        current_len += part_len;
                    }
                }
                continue;
            }

            if current_len + sentence_len + 1 > max_len && !current.is_empty() {
                chunks.push(current.trim().to_string());
                current.clear();
                current_len = 0;
            }

            if !current.is_empty() {
                current.push(' ');
                current_len += 1;
            }
            current.push_str(sentence);
            current_len += sentence_len;
        }

        if !current.is_empty() {
            chunks.push(current.trim().to_string());
        }
    }

    if chunks.is_empty() {
        vec![String::new()]
    } else {
        chunks
    }
}

fn split_sentences(text: &str) -> Vec<String> {
    let re = Regex::new(r"([.!?])\s+").unwrap();
    let matches: Vec<_> = re.find_iter(text).collect();

    if matches.is_empty() {
        return vec![text.to_string()];
    }

    let mut sentences = Vec::new();
    let mut last_end = 0;

    for m in matches {
        let before_punc = &text[last_end..m.start()];
        let mut is_abbrev = false;
        for abbrev in ABBREVIATIONS {
            let combined = format!("{}{}", before_punc.trim(), &text[m.start()..m.start() + 1]);
            if combined.ends_with(abbrev) {
                is_abbrev = true;
                break;
            }
        }

        if !is_abbrev {
            sentences.push(text[last_end..m.end()].to_string());
            last_end = m.end();
        }
    }

    if last_end < text.len() {
        sentences.push(text[last_end..].to_string());
    }

    if sentences.is_empty() {
        vec![text.to_string()]
    } else {
        sentences
    }
}

pub fn max_chunk_len_for_lang(lang: &str) -> usize {
    if lang == "ko" || lang == "ja" {
        120
    } else {
        300
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess_text_adds_lang_tags() {
        let result = preprocess_text("Hello.", "en").unwrap();
        assert_eq!(result, "<en>Hello.</en>");
    }

    #[test]
    fn test_preprocess_text_adds_period() {
        let result = preprocess_text("Hello", "en").unwrap();
        assert_eq!(result, "<en>Hello.</en>");
    }

    #[test]
    fn test_preprocess_text_removes_emoji() {
        let result = preprocess_text("Hi 😊.", "en").unwrap();
        assert_eq!(result, "<en>Hi.</en>");
    }

    #[test]
    fn test_is_valid_lang() {
        assert!(is_valid_lang("en"));
        assert!(is_valid_lang("ko"));
        assert!(!is_valid_lang("zz"));
    }

    #[test]
    fn test_chunk_text_short() {
        let chunks = chunk_text("Hello world.", Some(300));
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn test_text_to_unicode_values() {
        let vals = text_to_unicode_values("A");
        assert_eq!(vals, vec![65]);
    }

    #[test]
    fn test_length_to_mask() {
        let mask = length_to_mask(&[3], Some(5));
        assert_eq!(mask[[0, 0, 0]], 1.0);
        assert_eq!(mask[[0, 0, 2]], 1.0);
        assert_eq!(mask[[0, 0, 3]], 0.0);
    }

    #[test]
    fn test_max_chunk_len_for_lang() {
        assert_eq!(max_chunk_len_for_lang("en"), 300);
        assert_eq!(max_chunk_len_for_lang("ko"), 120);
        assert_eq!(max_chunk_len_for_lang("ja"), 120);
    }
}
