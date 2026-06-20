use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::domain::models::rag::{
    KnowledgeDomain, RagAnswer, RagChunk, RagDocument, RagEvalCase, RagEvalCaseResult,
    RagEvalReport, RagIngestionReport, RagMetadata, RagQuery, ScoredChunk, SourceType,
};
use crate::domain::ports::rag::{RagEmbedder, RagError, RagIndexer, RagReranker, RagRetriever};

#[derive(Debug, Error)]
pub enum RagApplicationError {
    #[error("RAG operation failed: {0}")]
    Rag(#[from] RagError),
    #[error("I/O error while preparing corpus: {0}")]
    Io(#[from] std::io::Error),
    #[error("Task join error while scanning corpus")]
    Join,
}

enum QueryMode {
    Baseline,
    Enhanced,
}

pub struct RagService {
    indexer: Arc<dyn RagIndexer>,
    retriever: Arc<dyn RagRetriever>,
    reranker: Arc<dyn RagReranker>,
    embedder: Option<Arc<dyn RagEmbedder>>,
    answer_cache: Arc<RwLock<HashMap<String, RagAnswer>>>,
    rag_dir: PathBuf,
}

impl RagService {
    pub fn new(
        indexer: Arc<dyn RagIndexer>,
        retriever: Arc<dyn RagRetriever>,
        reranker: Arc<dyn RagReranker>,
    ) -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let rag_dir = cwd.join("maestro").join("rag");
        Self::new_with_options(indexer, retriever, reranker, None, rag_dir)
    }

    pub fn new_with_options(
        indexer: Arc<dyn RagIndexer>,
        retriever: Arc<dyn RagRetriever>,
        reranker: Arc<dyn RagReranker>,
        embedder: Option<Arc<dyn RagEmbedder>>,
        rag_dir: PathBuf,
    ) -> Self {
        Self {
            indexer,
            retriever,
            reranker,
            embedder,
            answer_cache: Arc::new(RwLock::new(HashMap::new())),
            rag_dir,
        }
    }

    pub async fn ingest_paths(
        &self,
        roots: Vec<PathBuf>,
        chunk_size_chars: usize,
    ) -> Result<RagIngestionReport, RagApplicationError> {
        let file_paths = tokio::task::spawn_blocking(move || {
            let mut paths = Vec::new();
            for root in roots {
                if root.exists() {
                    let _ = collect_supported_files(&root, &mut paths);
                }
            }
            paths
        })
        .await
        .map_err(|_| RagApplicationError::Join)?;

        let mut documents = Vec::new();
        for path in file_paths {
            let content = match tokio::fs::read_to_string(&path).await {
                Ok(value) => value,
                Err(_) => continue,
            };

            if content.trim().is_empty() {
                continue;
            }

            let path_text = path.to_string_lossy().to_string();
            documents.push(RagDocument {
                id: Uuid::new_v4().to_string(),
                path: path_text.clone(),
                content,
                domain: detect_domain(&path_text),
            });
        }

        let mut chunks = Vec::new();
        for document in &documents {
            let mut part_index = 0_usize;
            for piece in chunk_text(&document.content, chunk_size_chars.max(300)) {
                part_index += 1;
                let embedding = self.embedding_for_text(&piece).await;
                let metadata = RagMetadata {
                    source_path: document.path.clone(),
                    source_type: infer_source_type(&document.path),
                    domain: document.domain.clone(),
                    subtopic: infer_subtopic(&document.path),
                    source_trust_score: infer_trust_score(&document.path),
                    project_relevance: infer_relevance_score(&document.path),
                    updated_at: "2026-06-20".to_string(),
                };

                chunks.push(RagChunk {
                    id: format!("{}-{part_index}", document.id),
                    document_id: document.id.clone(),
                    tokens: tokenize(&piece),
                    text: piece,
                    embedding,
                    metadata,
                });
            }
        }

        self.indexer.replace_chunks(chunks.clone()).await?;
        info!(
            docs = documents.len(),
            chunks = chunks.len(),
            "RAG ingestion completed"
        );

        Ok(RagIngestionReport {
            documents_indexed: documents.len(),
            chunks_indexed: chunks.len(),
        })
    }

    pub async fn query(
        &self,
        question: &str,
        top_k: usize,
    ) -> Result<RagAnswer, RagApplicationError> {
        if let Some(hit) = self.answer_cache.read().await.get(question).cloned() {
            return Ok(hit);
        }

        let response = self
            .query_with_mode(question, top_k, QueryMode::Enhanced)
            .await?;
        self.answer_cache
            .write()
            .await
            .insert(question.to_string(), response.clone());

        Ok(response)
    }

    pub async fn evaluate(&self, top_k: usize) -> Result<RagEvalReport, RagApplicationError> {
        let cases = self.load_or_create_eval_dataset().await?;

        let mut baseline_passed = 0_usize;
        let mut enhanced_passed = 0_usize;
        let mut baseline_total = 0.0_f32;
        let mut enhanced_total = 0.0_f32;
        let mut case_results = Vec::new();

        for case in &cases {
            let baseline = self
                .query_with_mode(&case.question, top_k, QueryMode::Baseline)
                .await?;
            let enhanced = self
                .query_with_mode(&case.question, top_k, QueryMode::Enhanced)
                .await?;

            let baseline_score = grounded_score(&baseline.answer, &case.required_terms);
            let enhanced_score = grounded_score(&enhanced.answer, &case.required_terms);

            baseline_total += baseline_score;
            enhanced_total += enhanced_score;

            if baseline_score >= 0.5 {
                baseline_passed += 1;
            }
            if enhanced_score >= 0.5 {
                enhanced_passed += 1;
            }

            case_results.push(RagEvalCaseResult {
                question: case.question.clone(),
                baseline_score,
                enhanced_score,
                baseline_citations: baseline.citations.len(),
                enhanced_citations: enhanced.citations.len(),
            });
        }

        let average_baseline_score = if cases.is_empty() {
            0.0
        } else {
            baseline_total / cases.len() as f32
        };
        let average_enhanced_score = if cases.is_empty() {
            0.0
        } else {
            enhanced_total / cases.len() as f32
        };

        let mut report = RagEvalReport {
            cases_total: cases.len(),
            baseline_cases_passed: baseline_passed,
            enhanced_cases_passed: enhanced_passed,
            average_baseline_score,
            average_enhanced_score,
            report_path: String::new(),
            case_results,
        };

        self.persist_eval_report(&mut report).await?;

        Ok(report)
    }

    async fn query_with_mode(
        &self,
        question: &str,
        top_k: usize,
        mode: QueryMode,
    ) -> Result<RagAnswer, RagApplicationError> {
        let mut query = RagQuery::classify(question);
        query.query_embedding = self.embedding_for_text(question).await;

        let expanded_limit = top_k.saturating_mul(4).max(top_k);
        let candidates = self
            .retriever
            .retrieve_hybrid(&query, expanded_limit)
            .await?;

        let selected = match mode {
            QueryMode::Baseline => candidates.into_iter().take(top_k).collect::<Vec<_>>(),
            QueryMode::Enhanced => self.reranker.rerank(&query, candidates, top_k).await?,
        };

        let citations = unique_citations(&selected);
        let answer = build_grounded_answer(question, &selected, &citations);

        Ok(RagAnswer {
            answer,
            citations,
            selected_chunks: selected,
        })
    }

    async fn embedding_for_text(&self, text: &str) -> Option<Vec<f32>> {
        if let Some(embedder) = &self.embedder {
            match embedder.embed(text).await {
                Ok(vector) if !vector.is_empty() => return Some(vector),
                Ok(_) => return None,
                Err(err) => {
                    warn!(error = %err, "embedding request failed; falling back to lexical retrieval");
                }
            }
        }
        None
    }

    async fn load_or_create_eval_dataset(&self) -> Result<Vec<RagEvalCase>, RagApplicationError> {
        let versioned_dataset = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("docs")
            .join("Maestro_Manifesto")
            .join("reference")
            .join("rag_eval_dataset.json");

        let runtime_dataset = self.rag_dir.join("eval_dataset.json");
        let candidates = vec![versioned_dataset, runtime_dataset.clone()];

        for path in &candidates {
            if !path.exists() {
                continue;
            }

            let raw = tokio::fs::read_to_string(path).await?;
            let parsed: Vec<RagEvalCase> = serde_json::from_str(&raw).map_err(|err| {
                RagApplicationError::Rag(RagError::Serialization(err.to_string()))
            })?;
            if !parsed.is_empty() {
                return Ok(parsed);
            }
        }

        tokio::fs::create_dir_all(&self.rag_dir).await?;
        let defaults = default_eval_cases();
        let raw = serde_json::to_string_pretty(&defaults)
            .map_err(|err| RagApplicationError::Rag(RagError::Serialization(err.to_string())))?;
        tokio::fs::write(runtime_dataset, raw).await?;

        Ok(defaults)
    }

    async fn persist_eval_report(
        &self,
        report: &mut RagEvalReport,
    ) -> Result<(), RagApplicationError> {
        let report_dir = self.rag_dir.join("reports");
        tokio::fs::create_dir_all(&report_dir).await?;

        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let report_path = report_dir.join(format!("eval-{ts}.json"));

        report.report_path = report_path.to_string_lossy().to_string();

        let raw = serde_json::to_string_pretty(report)
            .map_err(|err| RagApplicationError::Rag(RagError::Serialization(err.to_string())))?;
        tokio::fs::write(&report_path, raw).await?;

        Ok(())
    }
}

fn collect_supported_files(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
    if path.is_file() {
        if is_supported(path) {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }

    let entries = std::fs::read_dir(path)?;
    for entry in entries {
        let entry = entry?;
        let next = entry.path();

        if next
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name == "target" || name == ".git")
            .unwrap_or(false)
        {
            continue;
        }

        if next.is_dir() {
            collect_supported_files(&next, out)?;
        } else if is_supported(&next) {
            out.push(next);
        }
    }

    Ok(())
}

fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext, "md" | "rs" | "toml" | "yml" | "yaml"))
        .unwrap_or(false)
}

fn chunk_text(content: &str, max_chars: usize) -> Vec<String> {
    if content.trim().is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut current = String::new();

    for line in content.lines() {
        let next_len = current.len() + line.len() + 1;
        if next_len > max_chars && !current.trim().is_empty() {
            chunks.push(current.trim().to_string());
            current.clear();
        }

        current.push_str(line);
        current.push('\n');
    }

    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }

    chunks
}

fn tokenize(input: &str) -> Vec<String> {
    input
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.trim().is_empty())
        .map(|token| token.to_lowercase())
        .collect()
}

fn detect_domain(path_text: &str) -> KnowledgeDomain {
    KnowledgeDomain::from_text(path_text)
}

fn infer_source_type(path: &str) -> SourceType {
    if path.ends_with(".rs") {
        return SourceType::Code;
    }
    if path.ends_with(".md") {
        if path.contains("Manifesto") || path.contains("ARCHITECTURE") {
            return SourceType::Spec;
        }
        return SourceType::InternalGuide;
    }
    if path.ends_with(".toml") {
        return SourceType::Spec;
    }

    SourceType::Other
}

fn infer_subtopic(path: &str) -> String {
    let lowered = path.to_lowercase();
    if lowered.contains("architecture") {
        return "architecture".to_string();
    }
    if lowered.contains("conventions") {
        return "conventions".to_string();
    }
    if lowered.contains("readiness") {
        return "readiness".to_string();
    }
    if lowered.contains("prompt") {
        return "prompt".to_string();
    }
    if lowered.contains("cache") {
        return "cache".to_string();
    }

    "general".to_string()
}

fn infer_trust_score(path: &str) -> u8 {
    let lowered = path.to_lowercase();
    if lowered.contains("manifesto")
        || lowered.contains("architecture")
        || lowered.contains("conventions")
    {
        return 5;
    }
    if lowered.contains("readme") {
        return 4;
    }
    if lowered.ends_with(".rs") {
        return 4;
    }

    3
}

fn infer_relevance_score(path: &str) -> u8 {
    let lowered = path.to_lowercase();
    if lowered.contains("src/") || lowered.contains("docs/maestro_manifesto") {
        return 5;
    }
    if lowered.contains("docs/") {
        return 4;
    }

    3
}

fn unique_citations(chunks: &[ScoredChunk]) -> Vec<String> {
    let mut set = HashSet::new();
    let mut ordered = Vec::new();

    for item in chunks {
        let source = item.chunk.metadata.source_path.clone();
        if set.insert(source.clone()) {
            ordered.push(source);
        }
    }

    ordered
}

fn build_grounded_answer(question: &str, chunks: &[ScoredChunk], citations: &[String]) -> String {
    let mut out = String::new();
    out.push_str("Grounded answer:\n");
    out.push_str("- Question: ");
    out.push_str(question);
    out.push('\n');

    let mut idx = 0_usize;
    while idx < chunks.len() && idx < 3 {
        let preview = chunks[idx]
            .chunk
            .text
            .lines()
            .take(2)
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str("- Evidence: ");
        out.push_str(&preview);
        out.push('\n');
        idx += 1;
    }

    if !citations.is_empty() {
        out.push_str("- Sources: ");
        out.push_str(&citations.join(", "));
    }

    out
}

fn grounded_score(answer: &str, required_terms: &[String]) -> f32 {
    if required_terms.is_empty() {
        return 0.0;
    }

    let answer_lower = answer.to_lowercase();
    let hits = required_terms
        .iter()
        .filter(|term| answer_lower.contains(&term.to_lowercase()))
        .count();

    hits as f32 / required_terms.len() as f32
}

fn default_eval_cases() -> Vec<RagEvalCase> {
    vec![
        RagEvalCase {
            question: "How should shared async state be managed in this Rust project?".to_string(),
            required_terms: vec!["rwlock".to_string(), "tokio".to_string()],
        },
        RagEvalCase {
            question: "What should this harness validate before runtime actions?".to_string(),
            required_terms: vec!["readiness".to_string(), "validation".to_string()],
        },
        RagEvalCase {
            question: "How can prompt engineering be governed in maestro?".to_string(),
            required_terms: vec!["prompt".to_string(), "template".to_string()],
        },
        RagEvalCase {
            question: "What are important KV cache concerns for latency?".to_string(),
            required_terms: vec!["cache".to_string(), "latency".to_string()],
        },
    ]
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::infrastructure::rag::local_hybrid_index::LocalHybridIndex;

    use super::*;

    fn unique_root() -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("maestro-rag-app-{now}"))
    }

    #[tokio::test]
    async fn ingests_and_queries_local_corpus() {
        let root = unique_root();
        let docs = root.join("docs");
        let created = std::fs::create_dir_all(&docs);
        assert!(created.is_ok());

        let write_doc = std::fs::write(
            docs.join("kv_cache.md"),
            "KV cache prefill and decode optimization with paged attention.",
        );
        assert!(write_doc.is_ok());

        let index = Arc::new(LocalHybridIndex::new(&root));
        let service = RagService::new_with_options(
            index.clone(),
            index.clone(),
            index.clone(),
            None,
            root.join("maestro").join("rag"),
        );

        let ingestion = service.ingest_paths(vec![docs.clone()], 512).await;
        assert!(ingestion.is_ok());

        if let Ok(report) = ingestion {
            assert_eq!(report.documents_indexed, 1);
            assert!(report.chunks_indexed >= 1);
        }

        let query = service.query("How to optimize kv cache?", 3).await;
        assert!(query.is_ok());

        if let Ok(answer) = query {
            assert!(answer.answer.to_lowercase().contains("cache"));
            assert!(!answer.citations.is_empty());
        }

        let eval = service.evaluate(3).await;
        assert!(eval.is_ok());
        if let Ok(report) = eval {
            assert!(report.cases_total >= 4);
            assert!(!report.report_path.is_empty());
        }

        let _ = std::fs::remove_dir_all(root);
    }
}
