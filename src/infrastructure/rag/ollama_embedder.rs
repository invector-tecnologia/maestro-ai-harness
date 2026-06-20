use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::domain::ports::rag::{RagEmbedder, RagError};

pub struct OllamaEmbedder {
    client: Client,
    endpoint: String,
    model: String,
}

impl OllamaEmbedder {
    pub fn new(base_endpoint: &str, model: &str, timeout_ms: u64) -> Result<Self, RagError> {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|err| RagError::ExternalService(err.to_string()))?;

        Ok(Self {
            client,
            endpoint: normalize_embeddings_endpoint(base_endpoint),
            model: model.to_string(),
        })
    }
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}

#[async_trait]
impl RagEmbedder for OllamaEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, RagError> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        let payload = EmbeddingRequest {
            model: self.model.clone(),
            prompt: text.to_string(),
        };

        let response = self
            .client
            .post(&self.endpoint)
            .json(&payload)
            .send()
            .await
            .map_err(|err| RagError::ExternalService(err.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(RagError::ExternalService(format!(
                "embedding request failed with status {status}"
            )));
        }

        let parsed: EmbeddingResponse = response
            .json()
            .await
            .map_err(|err| RagError::ExternalService(err.to_string()))?;

        if parsed.embedding.is_empty() {
            return Err(RagError::ExternalService(
                "embedding payload returned empty vector".to_string(),
            ));
        }

        Ok(parsed.embedding)
    }
}

fn normalize_embeddings_endpoint(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    if trimmed.ends_with("/api/embeddings") {
        return trimmed.to_string();
    }
    if trimmed.ends_with("/v1") {
        return format!("{trimmed}/embeddings");
    }

    format!("{trimmed}/api/embeddings")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_ollama_endpoint() {
        assert_eq!(
            normalize_embeddings_endpoint("http://127.0.0.1:11434"),
            "http://127.0.0.1:11434/api/embeddings"
        );
        assert_eq!(
            normalize_embeddings_endpoint("http://127.0.0.1:11434/"),
            "http://127.0.0.1:11434/api/embeddings"
        );
        assert_eq!(
            normalize_embeddings_endpoint("http://127.0.0.1:11434/v1"),
            "http://127.0.0.1:11434/v1/embeddings"
        );
    }
}
