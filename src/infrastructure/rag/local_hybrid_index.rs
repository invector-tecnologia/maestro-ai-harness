use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::PathBuf;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::domain::models::rag::{KnowledgeDomain, RagChunk, RagQuery, ScoredChunk};
use crate::domain::ports::rag::{RagError, RagIndexer, RagReranker, RagRetriever};

pub struct LocalHybridIndex {
    index_path: PathBuf,
    chunks: RwLock<Vec<RagChunk>>,
}

impl LocalHybridIndex {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root: PathBuf = root.into();
        let index_path = root.join("maestro").join("rag").join("index.json");

        Self {
            index_path,
            chunks: RwLock::new(Vec::new()),
        }
    }

    pub fn index_path(&self) -> &PathBuf {
        &self.index_path
    }

    async fn load_if_needed(&self) -> Result<(), RagError> {
        let mut guard = self.chunks.write().await;
        if !guard.is_empty() {
            return Ok(());
        }

        if !self.index_path.exists() {
            return Ok(());
        }

        let raw = tokio::fs::read_to_string(&self.index_path)
            .await
            .map_err(|err| RagError::Io(err.to_string()))?;

        let parsed: Vec<RagChunk> = serde_json::from_str(&raw)
            .map_err(|err| RagError::Serialization(err.to_string()))?;
        *guard = parsed;

        Ok(())
    }
}

#[async_trait]
impl RagIndexer for LocalHybridIndex {
    async fn replace_chunks(&self, chunks: Vec<RagChunk>) -> Result<(), RagError> {
        let parent = self
            .index_path
            .parent()
            .ok_or_else(|| RagError::InvalidInput("Invalid index path".to_string()))?;

        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|err| RagError::Io(err.to_string()))?;

        let raw = serde_json::to_string_pretty(&chunks)
            .map_err(|err| RagError::Serialization(err.to_string()))?;

        tokio::fs::write(&self.index_path, raw)
            .await
            .map_err(|err| RagError::Io(err.to_string()))?;

        let mut guard = self.chunks.write().await;
        *guard = chunks;
        Ok(())
    }

    async fn load_chunks(&self) -> Result<Vec<RagChunk>, RagError> {
        self.load_if_needed().await?;
        let guard = self.chunks.read().await;
        Ok(guard.clone())
    }
}

#[async_trait]
impl RagRetriever for LocalHybridIndex {
    async fn retrieve_hybrid(
        &self,
        query: &RagQuery,
        limit: usize,
    ) -> Result<Vec<ScoredChunk>, RagError> {
        self.load_if_needed().await?;
        let chunks = self.chunks.read().await;
        if chunks.is_empty() {
            return Err(RagError::EmptyIndex);
        }

        let q_tokens = tokenize(&query.question);
        let q_tri = trigrams(&query.question);

        let mut scored = Vec::new();
        for chunk in chunks.iter() {
            let lexical = lexical_score(&q_tokens, &chunk.tokens);
            let vector = match (&query.query_embedding, &chunk.embedding) {
                (Some(query_vec), Some(chunk_vec)) => cosine_similarity(query_vec, chunk_vec),
                _ => jaccard_similarity(&q_tri, &trigrams(&chunk.text)),
            };
            let domain_boost = if query.domains.contains(&chunk.metadata.domain) {
                0.15
            } else if query.domains.contains(&KnowledgeDomain::General) {
                0.05
            } else {
                0.0
            };

            let seed = (lexical * 0.6) + (vector * 0.4) + domain_boost;
            if seed > 0.0 {
                scored.push(ScoredChunk {
                    chunk: chunk.clone(),
                    lexical_score: lexical,
                    vector_score: vector,
                    rerank_score: seed,
                });
            }
        }

        scored.sort_by(|left, right| {
            right
                .rerank_score
                .partial_cmp(&left.rerank_score)
                .unwrap_or(Ordering::Equal)
        });

        Ok(scored.into_iter().take(limit).collect())
    }
}

#[async_trait]
impl RagReranker for LocalHybridIndex {
    async fn rerank(
        &self,
        _query: &RagQuery,
        candidates: Vec<ScoredChunk>,
        limit: usize,
    ) -> Result<Vec<ScoredChunk>, RagError> {
        let mut rescored = Vec::with_capacity(candidates.len());

        for mut item in candidates {
            let trust = f32::from(item.chunk.metadata.source_trust_score.min(5)) * 0.03;
            let relevance = f32::from(item.chunk.metadata.project_relevance.min(5)) * 0.03;
            item.rerank_score = (item.lexical_score * 0.5) + (item.vector_score * 0.3) + trust + relevance;
            rescored.push(item);
        }

        rescored.sort_by(|left, right| {
            right
                .rerank_score
                .partial_cmp(&left.rerank_score)
                .unwrap_or(Ordering::Equal)
        });

        Ok(rescored.into_iter().take(limit).collect())
    }
}

fn tokenize(input: &str) -> Vec<String> {
    input
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.trim().is_empty())
        .map(|token| token.to_lowercase())
        .collect()
}

fn lexical_score(query_tokens: &[String], chunk_tokens: &[String]) -> f32 {
    if query_tokens.is_empty() || chunk_tokens.is_empty() {
        return 0.0;
    }

    let chunk_set: HashSet<&str> = chunk_tokens.iter().map(String::as_str).collect();
    let hits = query_tokens
        .iter()
        .filter(|token| chunk_set.contains(token.as_str()))
        .count();

    hits as f32 / query_tokens.len() as f32
}

fn trigrams(input: &str) -> HashSet<String> {
    let normalized = input.to_lowercase();
    let chars: Vec<char> = normalized.chars().collect();

    if chars.len() < 3 {
        let mut single = HashSet::new();
        if !normalized.is_empty() {
            single.insert(normalized);
        }
        return single;
    }

    let mut set = HashSet::new();
    let mut i = 0_usize;
    while i + 2 < chars.len() {
        let tri: String = [chars[i], chars[i + 1], chars[i + 2]].iter().collect();
        set.insert(tri);
        i += 1;
    }

    set
}

fn jaccard_similarity(left: &HashSet<String>, right: &HashSet<String>) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let mut intersection = 0_usize;
    for token in left {
        if right.contains(token) {
            intersection += 1;
        }
    }

    let union = left.len() + right.len() - intersection;
    if union == 0 {
        return 0.0;
    }

    intersection as f32 / union as f32
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let n = left.len().min(right.len());
    if n == 0 {
        return 0.0;
    }

    let mut dot = 0.0_f32;
    let mut left_norm = 0.0_f32;
    let mut right_norm = 0.0_f32;

    let mut idx = 0_usize;
    while idx < n {
        let l = left[idx];
        let r = right[idx];
        dot += l * r;
        left_norm += l * l;
        right_norm += r * r;
        idx += 1;
    }

    if left_norm <= f32::EPSILON || right_norm <= f32::EPSILON {
        return 0.0;
    }

    dot / (left_norm.sqrt() * right_norm.sqrt())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::domain::models::rag::{
        KnowledgeDomain, RagChunk, RagMetadata, RagQuery, SourceType,
    };

    use super::*;

    fn unique_root() -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("maestro-rag-test-{now}"))
    }

    fn sample_chunk(path: &str, text: &str, domain: KnowledgeDomain) -> RagChunk {
        RagChunk {
            id: format!("chunk-{path}"),
            document_id: format!("doc-{path}"),
            text: text.to_string(),
            tokens: text
                .split(|c: char| !c.is_alphanumeric())
                .filter(|token| !token.is_empty())
                .map(|token| token.to_lowercase())
                .collect(),
            embedding: None,
            metadata: RagMetadata {
                source_path: path.to_string(),
                source_type: SourceType::Code,
                domain,
                subtopic: "test".to_string(),
                source_trust_score: 5,
                project_relevance: 5,
                updated_at: "2026-06-20".to_string(),
            },
        }
    }

    #[tokio::test]
    async fn stores_and_retrieves_chunks() {
        let root = unique_root();
        let index = LocalHybridIndex::new(&root);

        let chunks = vec![sample_chunk(
            "src/main.rs",
            "tokio async runtime with trait-based architecture",
            KnowledgeDomain::Rust,
        )];

        let stored = index.replace_chunks(chunks.clone()).await;
        assert!(stored.is_ok());

        let loaded = index.load_chunks().await;
        assert!(loaded.is_ok());

        if let Ok(value) = loaded {
            assert_eq!(value.len(), 1);
            assert_eq!(value[0], chunks[0]);
        }

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn hybrid_retrieval_prefers_domain_match() {
        let root = unique_root();
        let index = LocalHybridIndex::new(&root);

        let chunks = vec![
            sample_chunk(
                "docs/rust.md",
                "tokio channels and trait object safety in rust",
                KnowledgeDomain::Rust,
            ),
            sample_chunk(
                "docs/kv.md",
                "kv cache prefill decode path and paged attention",
                KnowledgeDomain::KvCache,
            ),
        ];

        let stored = index.replace_chunks(chunks).await;
        assert!(stored.is_ok());

        let query = RagQuery::classify("How to improve kv cache prefill latency?");
        let result = index.retrieve_hybrid(&query, 2).await;
        assert!(result.is_ok());

        if let Ok(items) = result {
            assert!(!items.is_empty());
            assert_eq!(items[0].chunk.metadata.domain, KnowledgeDomain::KvCache);
        }

        let _ = std::fs::remove_dir_all(root);
    }
}
