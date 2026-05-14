use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub vector: Vec<f32>,
    pub text: String,
}

pub trait EmbeddingModel: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError>;
    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>, EmbedError> {
        let mut results = Vec::new();
        for text in texts {
            results.push(self.embed(text)?);
        }
        Ok(results)
    }
}

#[derive(Debug)]
pub struct EmbedError {
    message: String,
}

impl std::fmt::Display for EmbedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for EmbedError {}

pub struct SimpleEmbedding;

impl EmbeddingModel for SimpleEmbedding {
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        let mut hash: u64 = 0;
        for (i, c) in text.chars().enumerate() {
            hash = hash.wrapping_add((c as u64).wrapping_mul(i as u64 + 1));
        }
        let mut vec = vec![0.0; 128];
        for i in 0..128 {
            vec[i] = ((hash >> (i % 8)) & 0xFF) as f32 / 255.0;
        }
        Ok(vec)
    }
}

pub struct OpenAIEmbedding {
    api_key: String,
    model: String,
}

impl OpenAIEmbedding {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "text-embedding-3-small".to_string(),
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
}

impl EmbeddingModel for OpenAIEmbedding {
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        Ok(vec![0.0; 128])
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}

pub struct VectorIndex {
    vectors: Vec<Embedding>,
}

impl VectorIndex {
    pub fn new() -> Self {
        Self { vectors: Vec::new() }
    }

    pub fn add(&mut self, text: String, vector: Vec<f32>) {
        self.vectors.push(Embedding { vector, text });
    }

    pub fn search(&self, query: &[f32], k: usize) -> Vec<(String, f32)> {
        let mut scores: Vec<(String, f32)> = self
            .vectors
            .iter()
            .map(|e| (e.text.clone(), cosine_similarity(query, &e.vector)))
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k);
        scores
    }

    pub fn search_with_threshold(&self, query: &[f32], k: usize, threshold: f32) -> Vec<(String, f32)> {
        self.search(query, k)
            .into_iter()
            .filter(|(_, score)| *score >= threshold)
            .collect()
    }
}

impl Default for VectorIndex {
    fn default() -> Self {
        Self::new()
    }
}