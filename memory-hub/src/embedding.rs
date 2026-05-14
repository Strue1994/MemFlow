use serde::{Deserialize, Serialize};

/// Embedding dimension used throughout the system
pub const EMBEDDING_DIM: usize = 128;

/// Configuration for embedding engine selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub engine: EmbeddingEngineType,
    pub api_key: Option<String>,
    pub model: String,
    pub api_url: Option<String>,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            engine: EmbeddingEngineType::Local,
            api_key: None,
            model: "text-embedding-3-small".to_string(),
            api_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EmbeddingEngineType {
    /// OpenAI-compatible API (text-embedding-3-small, text-embedding-ada-002, etc.)
    OpenAI,
    /// Local SimHash-based embedding (no external deps, deterministic)
    Local,
}

/// Trait for embedding engines
pub trait EmbeddingEngine: Send + Sync {
    fn embed(&self, text: &str) -> Vec<f32>;
    fn engine_type(&self) -> EmbeddingEngineType;
}

/// Factory to create the appropriate engine
pub fn create_embedding_engine(config: &EmbeddingConfig) -> Box<dyn EmbeddingEngine> {
    match config.engine {
        EmbeddingEngineType::OpenAI => {
            if config.api_key.is_some() {
                Box::new(OpenAIEmbedding::new(config))
            } else {
                eprintln!("OpenAI embedding selected but no API key configured, falling back to Local");
                Box::new(LocalEmbedding::new())
            }
        }
        EmbeddingEngineType::Local => Box::new(LocalEmbedding::new()),
    }
}

/// OpenAI-compatible embedding via API call
pub struct OpenAIEmbedding {
    api_key: String,
    model: String,
    api_url: String,
    client: reqwest::blocking::Client,
}

impl OpenAIEmbedding {
    pub fn new(config: &EmbeddingConfig) -> Self {
        Self {
            api_key: config.api_key.clone().unwrap_or_default(),
            model: config.model.clone(),
            api_url: config.api_url.clone().unwrap_or_else(|| "https://api.openai.com/v1/embeddings".to_string()),
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl EmbeddingEngine for OpenAIEmbedding {
    fn embed(&self, text: &str) -> Vec<f32> {
        let body = serde_json::json!({
            "model": self.model,
            "input": text,
        });

        match self.client.post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
        {
            Ok(resp) => {
                if let Ok(data) = resp.json::<serde_json::Value>() {
                    if let Some(embedding) = data["data"][0]["embedding"].as_array() {
                        let vec: Vec<f32> = embedding.iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect();
                        if vec.len() == EMBEDDING_DIM {
                            return vec;
                        }
                        // If API returns different dimension, resize
                        let mut resized = vec![0.0; EMBEDDING_DIM];
                        for (i, v) in vec.iter().enumerate().take(EMBEDDING_DIM.min(vec.len())) {
                            resized[i] = *v;
                        }
                        return resized;
                    }
                }
                // Fallback on API error
                LocalEmbedding::compute_simhash(text)
            }
            Err(_) => LocalEmbedding::compute_simhash(text),
        }
    }

    fn engine_type(&self) -> EmbeddingEngineType {
        EmbeddingEngineType::OpenAI
    }
}

/// Local SimHash-based embedding (much better than the original simple_embed)
pub struct LocalEmbedding;

impl LocalEmbedding {
    pub fn new() -> Self { Self }

    /// Compute a SimHash-style fingerprint and convert to vector
    pub fn compute_simhash(text: &str) -> Vec<f32> {
        let mut v = vec![0i64; EMBEDDING_DIM];

        // Tokenize by word boundaries for better semantic capture
        for token in text.split_whitespace() {
            let hash = Self::murmur_hash(token);
            for i in 0..EMBEDDING_DIM {
                if (hash >> (i % 64)) & 1 == 1 {
                    v[i] += 1;
                } else {
                    v[i] -= 1;
                }
            }
        }

        // Convert to normalized f32 vector
        let max_val = v.iter().map(|x| x.abs()).max().unwrap_or(1).max(1) as f32;
        v.iter().map(|x| (*x as f32) / max_val).collect()
    }

    /// Simple MurmurHash-style hash for each token
    fn murmur_hash(input: &str) -> u64 {
        let bytes = input.as_bytes();
        let mut h: u64 = 0xc6a4a7935bd1e995u64;
        let m: u64 = 0xc6a4a7935bd1e995u64;
        let r: u32 = 47;

        for chunk in bytes.chunks(8) {
            let mut k: u64 = 0;
            for (i, &byte) in chunk.iter().enumerate() {
                k |= (byte as u64) << (i * 8);
            }
            k = k.wrapping_mul(m);
            k ^= k >> r;
            k = k.wrapping_mul(m);
            h ^= k;
            h = h.wrapping_mul(m);
        }

        h ^= bytes.len() as u64;
        h ^= h >> r;
        h = h.wrapping_mul(m);
        h ^= h >> r;
        h
    }
}

impl EmbeddingEngine for LocalEmbedding {
    fn embed(&self, text: &str) -> Vec<f32> {
        Self::compute_simhash(text)
    }

    fn engine_type(&self) -> EmbeddingEngineType {
        EmbeddingEngineType::Local
    }
}

impl Default for LocalEmbedding {
    fn default() -> Self { Self::new() }
}

/// Compute cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot / (norm_a * norm_b) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_embedding_deterministic() {
        let engine = LocalEmbedding::new();
        let v1 = engine.embed("hello world");
        let v2 = engine.embed("hello world");
        assert_eq!(v1.len(), EMBEDDING_DIM);
        assert_eq!(v1, v2, "Same text should produce same embedding");
    }

    #[test]
    fn test_similar_texts_have_positive_similarity() {
        let engine = LocalEmbedding::new();
        let v1 = engine.embed("I like dark mode");
        let v2 = engine.embed("Dark mode is great");
        let sim = cosine_similarity(&v1, &v2);
        assert!(sim > 0.0, "Similar texts should have positive similarity, got {}", sim);
    }

    #[test]
    fn test_different_texts_lower_similarity() {
        let engine = LocalEmbedding::new();
        let v1 = engine.embed("I like programming in Rust");
        let v2 = engine.embed("What is the weather today?");
        let v_same = engine.embed("I like programming in Rust");
        let sim_diff = cosine_similarity(&v1, &v2);
        let sim_same = cosine_similarity(&v1, &v_same);
        assert!(sim_diff < sim_same, "Different texts should have lower similarity than same text");
    }

    #[test]
    fn test_embedding_dimension() {
        let engine = LocalEmbedding::new();
        let vec = engine.embed("test");
        assert_eq!(vec.len(), EMBEDDING_DIM);
    }

    #[test]
    fn test_empty_text() {
        let engine = LocalEmbedding::new();
        let vec = engine.embed("");
        assert_eq!(vec.len(), EMBEDDING_DIM);
        // Empty text should produce zero-ish vector
        let sum: f32 = vec.iter().map(|x| x.abs()).sum();
        assert!(sum == 0.0);
    }
}
