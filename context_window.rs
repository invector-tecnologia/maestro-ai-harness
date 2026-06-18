use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum HarnessError {
    #[error("Context window limit exceeded. Max: {max}, Requested: {requested}")]
    LimitExceeded { max: usize, requested: usize },
}

/// Newtype para garantir a segurança em tempo de compilação das contagens de token/caracteres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenUsage(pub usize);

/// Módulo responsável por garantir que as IAs operem dentro dos limites seguros da janela de contexto.
pub struct ContextWindowHarness {
    max_tokens: usize,
    current_tokens: usize,
}

impl ContextWindowHarness {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            current_tokens: 0,
        }
    }

    /// Consome os tokens contabilizados, validando se excedem o limite estabelecido no Harness.
    pub fn consume_tokens(&mut self, amount: TokenUsage) -> Result<(), HarnessError> {
        let requested_total = self.current_tokens.saturating_add(amount.0);

        if requested_total > self.max_tokens {
            warn!(
                max = self.max_tokens,
                requested = requested_total,
                current = self.current_tokens,
                "Context window limit exceeded by AI agent"
            );
            return Err(HarnessError::LimitExceeded {
                max: self.max_tokens,
                requested: requested_total,
            });
        }

        self.current_tokens = requested_total;
        info!(
            consumed = amount.0,
            total = self.current_tokens,
            "Tokens consumed successfully within harness limits"
        );
        
        Ok(())
    }

    pub fn available_tokens(&self) -> usize {
        self.max_tokens.saturating_sub(self.current_tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consume_within_limits() {
        let mut harness = ContextWindowHarness::new(100);
        assert!(harness.consume_tokens(TokenUsage(50)).is_ok());
        assert_eq!(harness.available_tokens(), 50);
    }

    #[test]
    fn test_consume_exceeds_limits() {
        let mut harness = ContextWindowHarness::new(100);
        let result = harness.consume_tokens(TokenUsage(150));
        assert!(result.is_err());
        assert_eq!(harness.available_tokens(), 100); // Não deve alterar o estado
    }
}