use async_trait::async_trait;
use thiserror::Error;

use crate::domain::models::rag::{RagChunk, RagQuery, ScoredChunk};

#[derive(Debug, Error)]
pub enum RagError {
    #[error("RAG IO error: {0}")]
    Io(String),
    #[error("RAG serialization error: {0}")]
    Serialization(String),
    #[error("RAG invalid input: {0}")]
    InvalidInput(String),
    #[error("RAG index is empty")]
    EmptyIndex,
    #[error("RAG external service error: {0}")]
    ExternalService(String),
}

#[async_trait]
pub trait RagIndexer: Send + Sync {
    async fn replace_chunks(&self, chunks: Vec<RagChunk>) -> Result<(), RagError>;
    async fn load_chunks(&self) -> Result<Vec<RagChunk>, RagError>;
}

#[async_trait]
pub trait RagRetriever: Send + Sync {
    async fn retrieve_hybrid(
        &self,
        query: &RagQuery,
        limit: usize,
    ) -> Result<Vec<ScoredChunk>, RagError>;
}

#[async_trait]
pub trait RagReranker: Send + Sync {
    async fn rerank(
        &self,
        query: &RagQuery,
        candidates: Vec<ScoredChunk>,
        limit: usize,
    ) -> Result<Vec<ScoredChunk>, RagError>;
}

#[async_trait]
pub trait RagEmbedder: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, RagError>;
}
