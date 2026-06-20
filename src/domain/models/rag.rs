use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum KnowledgeDomain {
    Rust,
    VectorDb,
    Harness,
    PromptEngineering,
    KvCache,
    General,
}

impl KnowledgeDomain {
    pub fn from_text(value: &str) -> Self {
        let lowered = value.to_lowercase();

        if lowered.contains("kv cache") || lowered.contains("kv-cache") || lowered.contains("paged attention") {
            return Self::KvCache;
        }

        if lowered.contains("prompt") || lowered.contains("few-shot") || lowered.contains("system prompt") {
            return Self::PromptEngineering;
        }

        if lowered.contains("harness") || lowered.contains("orchestration") || lowered.contains("governance") {
            return Self::Harness;
        }

        if lowered.contains("qdrant")
            || lowered.contains("pgvector")
            || lowered.contains("weaviate")
            || lowered.contains("milvus")
            || lowered.contains("vector")
        {
            return Self::VectorDb;
        }

        if lowered.contains("rust") || lowered.contains("tokio") || lowered.contains("trait") {
            return Self::Rust;
        }

        Self::General
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceType {
    OfficialDoc,
    InternalGuide,
    Code,
    Spec,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RagMetadata {
    pub source_path: String,
    pub source_type: SourceType,
    pub domain: KnowledgeDomain,
    pub subtopic: String,
    pub source_trust_score: u8,
    pub project_relevance: u8,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RagDocument {
    pub id: String,
    pub path: String,
    pub content: String,
    pub domain: KnowledgeDomain,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RagChunk {
    pub id: String,
    pub document_id: String,
    pub text: String,
    pub tokens: Vec<String>,
    #[serde(default)]
    pub embedding: Option<Vec<f32>>,
    pub metadata: RagMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoredChunk {
    pub chunk: RagChunk,
    pub lexical_score: f32,
    pub vector_score: f32,
    pub rerank_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RagQuery {
    pub question: String,
    pub domains: Vec<KnowledgeDomain>,
    #[serde(default)]
    pub query_embedding: Option<Vec<f32>>,
}

impl RagQuery {
    pub fn classify(question: &str) -> Self {
        let mut domains = Vec::new();
        let primary = KnowledgeDomain::from_text(question);
        if primary != KnowledgeDomain::General {
            domains.push(primary);
        }

        let lowered = question.to_lowercase();
        if lowered.contains("rust") && !domains.contains(&KnowledgeDomain::Rust) {
            domains.push(KnowledgeDomain::Rust);
        }
        if (lowered.contains("vector") || lowered.contains("qdrant") || lowered.contains("pgvector"))
            && !domains.contains(&KnowledgeDomain::VectorDb)
        {
            domains.push(KnowledgeDomain::VectorDb);
        }
        if lowered.contains("harness") && !domains.contains(&KnowledgeDomain::Harness) {
            domains.push(KnowledgeDomain::Harness);
        }
        if lowered.contains("prompt") && !domains.contains(&KnowledgeDomain::PromptEngineering) {
            domains.push(KnowledgeDomain::PromptEngineering);
        }
        if lowered.contains("kv") && !domains.contains(&KnowledgeDomain::KvCache) {
            domains.push(KnowledgeDomain::KvCache);
        }

        if domains.is_empty() {
            domains.push(KnowledgeDomain::General);
        }

        Self {
            question: question.to_string(),
            domains,
            query_embedding: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RagEvalCase {
    pub question: String,
    pub required_terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RagEvalCaseResult {
    pub question: String,
    pub baseline_score: f32,
    pub enhanced_score: f32,
    pub baseline_citations: usize,
    pub enhanced_citations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RagAnswer {
    pub answer: String,
    pub citations: Vec<String>,
    pub selected_chunks: Vec<ScoredChunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RagIngestionReport {
    pub documents_indexed: usize,
    pub chunks_indexed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RagEvalReport {
    pub cases_total: usize,
    pub baseline_cases_passed: usize,
    pub enhanced_cases_passed: usize,
    pub average_baseline_score: f32,
    pub average_enhanced_score: f32,
    pub report_path: String,
    pub case_results: Vec<RagEvalCaseResult>,
}
