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
}
