pub fn normalize_chat_completions_endpoint(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        return trimmed.to_string();
    }

    format!("{trimmed}/chat/completions")
}

pub fn normalize_ollama_chat_endpoint(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        return trimmed.to_string();
    }
    if trimmed.ends_with("/v1") {
        return format!("{trimmed}/chat/completions");
    }

    format!("{trimmed}/v1/chat/completions")
}

pub fn normalize_ollama_embeddings_endpoint(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    if trimmed.ends_with("/api/embeddings") || trimmed.ends_with("/v1/embeddings") {
        return trimmed.to_string();
    }
    if trimmed.ends_with("/v1") {
        return format!("{trimmed}/embeddings");
    }

    format!("{trimmed}/api/embeddings")
}

/// Derive the Ollama model-catalog endpoint (`/api/tags`) from a normalized chat
/// completions endpoint, used by the SENSE health probe.
pub fn ollama_tags_endpoint(chat_endpoint: &str) -> String {
    let base = ollama_base(chat_endpoint);
    format!("{base}/api/tags")
}

fn ollama_base(chat_endpoint: &str) -> String {
    let trimmed = chat_endpoint.trim_end_matches('/');
    let without_chat = trimmed
        .strip_suffix("/v1/chat/completions")
        .or_else(|| trimmed.strip_suffix("/chat/completions"))
        .unwrap_or(trimmed);
    without_chat.trim_end_matches('/').to_string()
}

/// Derive the OpenAI model-catalog endpoint (`/models`) from a normalized chat
/// completions endpoint, used by the SENSE health probe.
pub fn openai_models_endpoint(chat_endpoint: &str) -> String {
    let trimmed = chat_endpoint.trim_end_matches('/');
    let base = trimmed
        .strip_suffix("/chat/completions")
        .unwrap_or(trimmed)
        .trim_end_matches('/');
    format!("{base}/models")
}

/// Derive the Anthropic model-catalog endpoint (`/v1/models`) from the messages
/// endpoint, used by the SENSE health probe.
pub fn anthropic_models_endpoint(messages_endpoint: &str) -> String {
    let trimmed = messages_endpoint.trim_end_matches('/');
    let base = trimmed
        .strip_suffix("/v1/messages")
        .unwrap_or(trimmed)
        .trim_end_matches('/');
    format!("{base}/v1/models")
}

/// Whether a configured model name matches any catalog entry, tolerating tag
/// suffixes such as Ollama's `:latest` and version-suffixed identifiers.
pub fn model_in_catalog<'a>(model: &str, catalog: impl IntoIterator<Item = &'a str>) -> bool {
    let model = model.trim();
    if model.is_empty() {
        return false;
    }
    catalog.into_iter().any(|name| {
        let name = name.trim();
        name == model || name.split(':').next() == Some(model) || name.starts_with(model)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_chat_completion_endpoint() {
        assert_eq!(
            normalize_chat_completions_endpoint("https://api.openai.com/v1"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            normalize_chat_completions_endpoint("https://api.openai.com/v1/chat/completions"),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn normalizes_ollama_chat_endpoint() {
        assert_eq!(
            normalize_ollama_chat_endpoint("http://localhost:11434"),
            "http://localhost:11434/v1/chat/completions"
        );
        assert_eq!(
            normalize_ollama_chat_endpoint("http://localhost:11434/v1"),
            "http://localhost:11434/v1/chat/completions"
        );
    }

    #[test]
    fn normalizes_ollama_embeddings_endpoint() {
        assert_eq!(
            normalize_ollama_embeddings_endpoint("http://127.0.0.1:11434"),
            "http://127.0.0.1:11434/api/embeddings"
        );
        assert_eq!(
            normalize_ollama_embeddings_endpoint("http://127.0.0.1:11434/v1"),
            "http://127.0.0.1:11434/v1/embeddings"
        );
        assert_eq!(
            normalize_ollama_embeddings_endpoint("http://127.0.0.1:11434/api/embeddings"),
            "http://127.0.0.1:11434/api/embeddings"
        );
    }

    #[test]
    fn derives_ollama_tags_endpoint() {
        assert_eq!(
            ollama_tags_endpoint("http://localhost:11434/v1/chat/completions"),
            "http://localhost:11434/api/tags"
        );
        assert_eq!(
            ollama_tags_endpoint("http://localhost:11434/chat/completions"),
            "http://localhost:11434/api/tags"
        );
    }

    #[test]
    fn derives_openai_models_endpoint() {
        assert_eq!(
            openai_models_endpoint("https://api.openai.com/v1/chat/completions"),
            "https://api.openai.com/v1/models"
        );
    }

    #[test]
    fn derives_anthropic_models_endpoint() {
        assert_eq!(
            anthropic_models_endpoint("https://api.anthropic.com/v1/messages"),
            "https://api.anthropic.com/v1/models"
        );
    }

    #[test]
    fn matches_model_in_catalog_with_tag_tolerance() {
        assert!(model_in_catalog("llama3", ["llama3:latest", "qwen2.5:7b"]));
        assert!(model_in_catalog("gpt-4o-mini", ["gpt-4o-mini", "gpt-4o"]));
        assert!(!model_in_catalog("missing-model", ["llama3:latest"]));
        assert!(!model_in_catalog("", ["llama3:latest"]));
    }
}
