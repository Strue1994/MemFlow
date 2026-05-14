# P2-2: Vector Search Integration

## Priority

P2 - Medium-term

## Key Files / Modules

- `memory-hub/src/recall_enhancer.rs`
- `memory-hub/Cargo.toml`
- `memory-hub/src/embeddings.rs`

## Goals

替换当前的伪向量检索，实现真正的语义搜索。

## Specific Requirements

1.  **Embedding Model Integration**
   - 评估 `fastembed-rs` 或 OpenAI Embedding API
   - 在存储时生成向量
   - 在检索时对查询生成向量

2.  **Similarity Search**
   - 使用余弦相似度进行搜索
   - 暴力搜索 (初期实现)

3.  **Optional: Vector Index**
   - 引入 `hnswlib` 或 `pgvector`
   - 提高检索速度

4.  **Configuration**
   - 模型选择可配置
   - 向量维度可配置

## Acceptance Criteria

- [ ] 可以基于语义含义检索到相关记忆
- [ ] 检索结果与关键词无关

## Implementation

```rust
use std::sync::Arc;

pub struct RecallEnhancer {
    embedding_model: Arc<dyn EmbeddingModel>,
    index: Arc<Mutex<AnnoyIndex>>,
}

pub trait EmbeddingModel: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>, Error>;
}

pub struct OpenAIEmbedder {
    client: OpenAIClient,
    model: String,
}

impl EmbeddingModel for OpenAIEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>, Error> {
        let response = self.client.embeddings_create(&self.model, text)?;
        Ok(response.data[0].embedding)
    }
}

impl RecallEnhancer {
    pub fn search(&self, query: &str, k: usize) -> Result<Vec<SearchResult>, Error> {
        let query_vec = self.embedding_model.embed(query)?;
        
        let index = self.index.lock().unwrap();
        let results = index.search(&query_vec, k);
        
        Ok(results)
    }
}
```

```rust
// Dependencies to add:
// fastembed-rs = "3"  # or use openai crate
// hnswlib = "0.8"
```