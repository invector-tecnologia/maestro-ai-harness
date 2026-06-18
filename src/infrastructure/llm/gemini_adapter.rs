use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, RedirectUrl, RefreshToken,
    Scope, TokenResponse, TokenUrl,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::application::config::{AuthMode, ProviderConfig};
use crate::domain::ports::llm_provider::LlmProvider;
use crate::domain::ports::role::RoleError;
use crate::infrastructure::llm::provider_registry::ProviderRegistryError;

pub struct GeminiAdapter {
    client: Client,
    endpoint: String,
    auth_mode: AuthMode,
    bearer_token: RwLock<Option<String>>, // Usamos RwLock para cachear o token em memória
}

impl GeminiAdapter {
    pub fn from_provider_config(
        provider: &ProviderConfig,
    ) -> Result<Arc<dyn LlmProvider>, ProviderRegistryError> {
        let timeout = Duration::from_millis(provider.timeout_ms);
        let client = Client::builder().timeout(timeout).build().map_err(|_| {
            ProviderRegistryError::InconsistentConfig(format!(
                "Nao foi possivel construir cliente HTTP para provider {}",
                provider.name
            ))
        })?;

        Ok(Arc::new(Self {
            client,
            endpoint: provider.endpoint.clone(),
            auth_mode: provider.auth_mode.clone(),
            bearer_token: RwLock::new(provider.auth_token.clone()),
        }))
    }

    async fn get_access_token(&self) -> Result<String, RoleError> {
        // Fast path: retorna o token se já foi cacheado ou configurado
        if let Some(token) = self.bearer_token.read().await.as_ref() {
            return Ok(token.clone());
        }

        if self.auth_mode == AuthMode::Browser {
            if let Ok(entry) = keyring::Entry::new("maestro_ai", "gemini_refresh_token") {
                if let Ok(refresh_secret) = entry.get_password() {
                    info!("Token de refresh seguro encontrado, tentando renovar o access token...");
                    if let Ok(access_token) = refresh_access_token(refresh_secret).await {
                        *self.bearer_token.write().await = Some(access_token.clone());
                        return Ok(access_token);
                    }
                }
            }

            let token = perform_oauth_flow().await?;
            // Salva em cache para as próximas chamadas da TUI
            *self.bearer_token.write().await = Some(token.clone());
            return Ok(token);
        }

        Err(RoleError::LlmError)
    }

    /// Limpa as credenciais cacheadas em memória e no OS Keyring
    pub async fn logout(&self) {
        *self.bearer_token.write().await = None;
        let _ = Self::clear_credentials();
    }

    /// Remove o refresh token do OS Keyring de forma estática (para uso direto pela CLI)
    pub fn clear_credentials() -> Result<(), RoleError> {
        if let Ok(entry) = keyring::Entry::new("maestro_ai", "gemini_refresh_token") {
            if entry.delete_credential().is_ok() {
                info!("Refresh token removido de forma segura do OS Keyring.");
            } else {
                info!("Nenhum token encontrado no OS Keyring para remover.");
            }
        }
        Ok(())
    }
}

// Representações da API Google Cloud (Vertex AI) / Gemini
#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}
#[derive(Debug, Serialize)]
struct GeminiContent {
    role: &'static str,
    parts: Vec<GeminiPart>,
}
#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}
#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}
#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiResponseContent,
}
#[derive(Debug, Deserialize)]
struct GeminiResponseContent {
    parts: Option<Vec<GeminiResponsePart>>,
}
#[derive(Debug, Deserialize)]
struct GeminiResponsePart {
    text: Option<String>,
}

#[async_trait]
impl LlmProvider for GeminiAdapter {
    async fn generate_completion(&self, prompt: &str) -> Result<String, RoleError> {
        let started_at = std::time::Instant::now();

        let token = self.get_access_token().await?;

        let request = GeminiRequest {
            contents: vec![GeminiContent {
                role: "user",
                parts: vec![GeminiPart {
                    text: prompt.to_string(),
                }],
            }],
        };

        let mut builder = self.client.post(&self.endpoint).json(&request);
        if !token.is_empty() {
            builder = builder.bearer_auth(token);
        }

        let response = builder.send().await.map_err(|error| {
            error!(latency_ms = started_at.elapsed().as_millis(), error = %error, "falha de requisicao para provider gemini");
            RoleError::LlmError
        })?;

        let status = response.status();
        if !status.is_success() {
            let err_text = response.text().await.unwrap_or_default();
            error!(latency_ms = started_at.elapsed().as_millis(), status = %status, "resposta HTTP invalida do provider gemini: {}", err_text);
            return Err(RoleError::LlmError);
        }

        let payload: GeminiResponse = response.json().await.map_err(|error| {
            error!(latency_ms = started_at.elapsed().as_millis(), error = %error, "payload invalido recebido do provider gemini");
            RoleError::LlmError
        })?;

        if let Some(candidates) = payload.candidates {
            if let Some(candidate) = candidates.first() {
                if let Some(parts) = &candidate.content.parts {
                    if let Some(part) = parts.first() {
                        if let Some(text) = &part.text {
                            info!(
                                latency_ms = started_at.elapsed().as_millis(),
                                "completion gerado com sucesso no provider gemini"
                            );
                            return Ok(text.trim().to_string());
                        }
                    }
                }
            }
        }

        error!("resposta sem conteudo do provider gemini");
        Err(RoleError::LlmError)
    }
}

fn create_oauth_client() -> Result<BasicClient, RoleError> {
    // TODO: Insira seu Client ID do Google Cloud Console (Desktop App)
    let client_id = ClientId::new("SUA_CLIENT_ID_AQUI.apps.googleusercontent.com".to_string());

    let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
        .map_err(|_| RoleError::LlmError)?;
    let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
        .map_err(|_| RoleError::LlmError)?;
    let redirect_url =
        RedirectUrl::new("http://127.0.0.1:8080".to_string()).map_err(|_| RoleError::LlmError)?;

    Ok(BasicClient::new(client_id, None, auth_url, Some(token_url)).set_redirect_uri(redirect_url))
}

async fn refresh_access_token(refresh_secret: String) -> Result<String, RoleError> {
    let client = create_oauth_client()?;
    let refresh_token = RefreshToken::new(refresh_secret);

    let token_result = client
        .exchange_refresh_token(&refresh_token)
        .request_async(async_http_client)
        .await
        .map_err(|error| {
            error!(%error, "Falha ao renovar o access token com o refresh token existente");
            RoleError::LlmError
        })?;

    if let Some(new_refresh) = token_result.refresh_token() {
        if let Ok(entry) = keyring::Entry::new("maestro_ai", "gemini_refresh_token") {
            let _ = entry.set_password(new_refresh.secret());
        }
    }

    info!("Access token renovado com sucesso via Refresh Token.");
    Ok(token_result.access_token().secret().clone())
}

async fn perform_oauth_flow() -> Result<String, RoleError> {
    info!("Iniciando fluxo OAuth 2.0 no navegador...");

    let client = create_oauth_client()?;
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (authorize_url, _csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/cloud-platform".to_string(),
        ))
        .add_extra_param("access_type", "offline") // Necessário para receber o Refresh Token
        .add_extra_param("prompt", "consent") // Força o consentimento na primeira vez
        .set_pkce_challenge(pkce_challenge)
        .url();

    // 1. Abre o navegador do usuário
    if open::that(authorize_url.as_str()).is_err() {
        println!(
            "Por favor, abra esta URL no seu navegador:\n{}",
            authorize_url
        );
    }

    // 2. Levanta o servidor localhost temporário para receber o callback
    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .map_err(|_| RoleError::LlmError)?;
    let (mut stream, _) = listener.accept().await.map_err(|_| RoleError::LlmError)?;
    let mut buffer = [0; 2048];
    stream
        .read(&mut buffer)
        .await
        .map_err(|_| RoleError::LlmError)?;

    let request = String::from_utf8_lossy(&buffer);
    let mut auth_code = String::new();

    if let Some(path) = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
    {
        if let Ok(url) = url::Url::parse(&format!("http://localhost{}", path)) {
            for (key, value) in url.query_pairs() {
                if key == "code" {
                    auth_code = value.into_owned();
                }
            }
        }
    }

    let html_response = "HTTP/1.1 200 OK\r\n\r\n<html><body><h2>Autenticação no Google Cloud concluída! Você pode voltar para o terminal.</h2></body></html>";
    let _ = stream.write_all(html_response.as_bytes()).await;

    // 3. Troca o código por um Access Token e Refresh Token
    let token_result = client
        .exchange_code(AuthorizationCode::new(auth_code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await
        .map_err(|_| RoleError::LlmError)?;

    // Salva o Refresh Token de forma segura
    if let Some(refresh_token) = token_result.refresh_token() {
        if let Ok(entry) = keyring::Entry::new("maestro_ai", "gemini_refresh_token") {
            let _ = entry.set_password(refresh_token.secret());
            info!("Refresh token salvo com sucesso de forma segura no OS Keyring.");
        }
    }

    Ok(token_result.access_token().secret().clone())
}
