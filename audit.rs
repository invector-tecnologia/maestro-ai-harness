use tracing::{info, warn};

/// Estrutura para eventos que passam pela Sandbox (Harness).
pub struct AuditEvent {
    pub action: String,
    pub payload: String,
}

/// Auditor da Sandbox para validar intents destrutivos ou suspeitos de uma IA.
pub struct Auditor;

impl Auditor {
    pub fn new() -> Self {
        Self
    }

    /// Grava as ações no log protegido do Harness.
    pub fn log_event(&self, event: &AuditEvent) {
        info!(
            action = %event.action,
            payload_size = event.payload.len(),
            "Audit event recorded by Harness"
        );
    }

    /// Validações básicas de segurança antes de permitir operações de I/O.
    pub fn validate_safety(&self, payload: &str) -> bool {
        // Exemplo de barreira para comandos indesejados (em um cenário real, usaria regex/parsers avançados)
        if payload.contains("rm -rf") || payload.contains("sudo") {
            warn!(payload = %payload, "Unsafe command intercepted by Harness Sandbox!");
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_safety_allows_safe_commands() {
        let auditor = Auditor::new();
        assert!(auditor.validate_safety("echo 'Hello World'"));
    }

    #[test]
    fn test_validate_safety_blocks_unsafe_commands() {
        let auditor = Auditor::new();
        assert!(!auditor.validate_safety("rm -rf /"));
    }
}