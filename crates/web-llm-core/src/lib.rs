//! Shared types and provider contracts for the web LLM workspace.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, WebLlmError>;

#[derive(Debug, thiserror::Error)]
pub enum WebLlmError {
    #[error("unsupported provider capability: {capability}")]
    UnsupportedCapability { capability: &'static str },
    #[error("feature not implemented yet: {feature}")]
    NotImplemented { feature: &'static str },
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("browser error: {0}")]
    Browser(String),
    #[error("timed out waiting for {operation} after {timeout_ms} ms")]
    Timeout {
        operation: &'static str,
        timeout_ms: u64,
    },
    #[error("unable to determine a data directory for local state")]
    DataDirUnavailable,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl WebLlmError {
    #[must_use]
    pub fn unsupported_capability(capability: &'static str) -> Self {
        Self::UnsupportedCapability { capability }
    }

    #[must_use]
    pub fn not_implemented(feature: &'static str) -> Self {
        Self::NotImplemented { feature }
    }

    #[must_use]
    pub fn browser(message: impl Into<String>) -> Self {
        Self::Browser(message.into())
    }

    #[must_use]
    pub fn timeout(operation: &'static str, timeout_ms: u64) -> Self {
        Self::Timeout {
            operation,
            timeout_ms,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderOptions {
    pub base_url: Option<String>,
    pub profile_dir: Option<PathBuf>,
    pub headed: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AskRequest {
    pub session_id: Option<String>,
    pub thinking_enabled: bool,
    pub search_enabled: bool,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AskResponse {
    pub url: Option<String>,
    pub chat_session_id: Option<String>,
    pub model: Option<String>,
    pub prompt: Option<String>,
    pub requested_session_id: Option<String>,
    pub response: Option<String>,
    pub reasoning: Option<String>,
    pub session_mode: SessionMode,
    pub search_enabled: bool,
    pub thinking_enabled: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    #[default]
    New,
    Continue,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoginRequest {
    pub timeout_ms: Option<u64>,
    pub phone_number: Option<String>,
    pub verification_code: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginResult {
    pub state: LoginState,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoginState {
    #[default]
    LoggedIn,
    VerificationCodeRequired,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InspectRequest {
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InspectResult {
    pub url: Option<String>,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeleteSessionResult {
    pub chat_session_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeleteAllResult {
    pub deleted_all_history: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub ask: bool,
    pub agent: bool,
    pub login: bool,
    pub inspect: bool,
    pub delete_session: bool,
    pub delete_current: bool,
    pub delete_all: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ProviderMetadata {
    pub id: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub capabilities: ProviderCapabilities,
}

pub type ProviderFactory = fn(ProviderOptions) -> Result<Box<dyn ProviderHandle>>;

#[derive(Clone, Copy)]
pub struct ProviderDefinition {
    pub metadata: ProviderMetadata,
    pub factory: ProviderFactory,
}

impl ProviderDefinition {
    pub fn create(self, options: ProviderOptions) -> Result<Box<dyn ProviderHandle>> {
        (self.factory)(options)
    }
}

#[async_trait]
pub trait WebLlmProvider: Send {
    async fn ask(&mut self, prompt: &str, request: AskRequest) -> Result<AskResponse>;
}

#[async_trait]
pub trait LoginCapable: Send {
    async fn login(&mut self, _request: LoginRequest) -> Result<LoginResult> {
        Err(WebLlmError::unsupported_capability("login"))
    }
}

#[async_trait]
pub trait InspectCapable: Send {
    async fn inspect(&mut self, _request: InspectRequest) -> Result<InspectResult> {
        Err(WebLlmError::unsupported_capability("inspect"))
    }
}

#[async_trait]
pub trait DeleteCapable: Send {
    async fn delete_session(&mut self, _session_id: &str) -> Result<DeleteSessionResult> {
        Err(WebLlmError::unsupported_capability("delete_session"))
    }

    async fn delete_current_session(&mut self) -> Result<DeleteSessionResult> {
        Err(WebLlmError::unsupported_capability("delete_current"))
    }

    async fn delete_all_history(&mut self) -> Result<DeleteAllResult> {
        Err(WebLlmError::unsupported_capability("delete_all"))
    }
}

pub trait ProviderHandle:
    WebLlmProvider + LoginCapable + InspectCapable + DeleteCapable + Send
{
    fn metadata(&self) -> ProviderMetadata;
}
