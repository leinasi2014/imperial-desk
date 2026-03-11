//! Provider-agnostic agent loop scaffold.

use serde::{Deserialize, Serialize};
use web_llm_core::{AskRequest, ProviderHandle, Result, WebLlmError};

const DEFAULT_MAX_STEPS: usize = 8;
const DEFAULT_TIMEOUT_MS: u64 = 180_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOptions {
    pub max_steps: usize,
    pub search_enabled: bool,
    pub thinking_enabled: bool,
    pub timeout_ms: u64,
}

impl Default for AgentOptions {
    fn default() -> Self {
        Self {
            max_steps: DEFAULT_MAX_STEPS,
            search_enabled: false,
            thinking_enabled: true,
            timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunResult {
    pub chat_session_id: Option<String>,
    pub final_answer: String,
    pub max_steps: usize,
    pub steps: usize,
    pub search_enabled: bool,
    pub thinking_enabled: bool,
}

pub struct WebLlmAgent<'a> {
    provider: &'a mut dyn ProviderHandle,
    options: AgentOptions,
}

impl<'a> WebLlmAgent<'a> {
    #[must_use]
    pub fn new(provider: &'a mut dyn ProviderHandle, options: AgentOptions) -> Self {
        Self { provider, options }
    }

    pub async fn run(&mut self, task: &str, session_id: Option<String>) -> Result<AgentRunResult> {
        if task.trim().is_empty() {
            return Err(WebLlmError::InvalidInput("agent task is empty".to_owned()));
        }

        let response = self
            .provider
            .ask(
                task,
                AskRequest {
                    session_id,
                    thinking_enabled: self.options.thinking_enabled,
                    search_enabled: self.options.search_enabled,
                    timeout_ms: Some(self.options.timeout_ms),
                },
            )
            .await?;

        Ok(AgentRunResult {
            chat_session_id: response.chat_session_id,
            final_answer: response.response.unwrap_or_default(),
            max_steps: self.options.max_steps,
            steps: 1,
            search_enabled: self.options.search_enabled,
            thinking_enabled: self.options.thinking_enabled,
        })
    }
}
