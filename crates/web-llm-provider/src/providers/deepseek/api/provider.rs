use async_trait::async_trait;
use web_llm_core::{
    AskRequest, AskResponse, DeleteAllResult, DeleteCapable, DeleteSessionResult, InspectCapable,
    InspectRequest, InspectResult, LoginCapable, LoginRequest, LoginResult, ProviderCapabilities,
    ProviderDefinition, ProviderHandle, ProviderMetadata, ProviderOptions, Result, WebLlmError,
    WebLlmProvider,
};

pub const PROVIDER_ID: &str = "deepseek-api";

pub struct DeepseekApiProvider {
    metadata: ProviderMetadata,
    _options: ProviderOptions,
}

#[must_use]
pub fn definition() -> ProviderDefinition {
    ProviderDefinition {
        metadata: metadata(),
        factory: create_provider,
    }
}

fn metadata() -> ProviderMetadata {
    ProviderMetadata {
        id: PROVIDER_ID,
        display_name: "DeepSeek API",
        description: "DeepSeek API adapter placeholder",
        capabilities: ProviderCapabilities {
            ask: true,
            agent: true,
            login: false,
            inspect: false,
            delete_session: false,
            delete_current: false,
            delete_all: false,
        },
    }
}

fn create_provider(options: ProviderOptions) -> Result<Box<dyn ProviderHandle>> {
    Ok(Box::new(DeepseekApiProvider {
        metadata: metadata(),
        _options: options,
    }))
}

impl ProviderHandle for DeepseekApiProvider {
    fn metadata(&self) -> ProviderMetadata {
        self.metadata
    }
}

#[async_trait]
impl WebLlmProvider for DeepseekApiProvider {
    async fn ask(&mut self, _prompt: &str, _request: AskRequest) -> Result<AskResponse> {
        Err(WebLlmError::not_implemented("deepseek api ask flow"))
    }
}

#[async_trait]
impl LoginCapable for DeepseekApiProvider {
    async fn login(&mut self, _request: LoginRequest) -> Result<LoginResult> {
        Err(WebLlmError::unsupported_capability("login"))
    }
}

#[async_trait]
impl InspectCapable for DeepseekApiProvider {
    async fn inspect(&mut self, _request: InspectRequest) -> Result<InspectResult> {
        Err(WebLlmError::unsupported_capability("inspect"))
    }
}

#[async_trait]
impl DeleteCapable for DeepseekApiProvider {
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
