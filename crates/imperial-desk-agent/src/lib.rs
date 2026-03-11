//! Provider-agnostic agent loop scaffold.

use imperial_desk_core::{AskRequest, ProviderHandle, Result, WebLlmError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const AGENT_PROTOCOL_SPEC: &str = r#"Respond with exactly one JSON object and no extra text.
Allowed response shapes:
{"type":"tool_call","tool":"<name>","arguments":{...},"reason":"optional"}
{"type":"final","answer":"<text>"}"#;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AgentProtocolMessage {
    ToolCall {
        tool: String,
        arguments: Value,
        #[serde(default)]
        reason: Option<String>,
    },
    Final {
        answer: String,
    },
}

#[derive(Debug, Serialize)]
struct ToolCallResult<'a> {
    r#type: &'static str,
    tool: &'a str,
    arguments: &'a Value,
    ok: bool,
    error: &'static str,
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
        if self.options.max_steps == 0 {
            return Err(WebLlmError::InvalidInput(
                "max_steps must be greater than 0".to_owned(),
            ));
        }

        let mut active_session_id = session_id;
        let mut next_prompt = build_initial_protocol_prompt(task);

        for step in 1..=self.options.max_steps {
            let response = self
                .provider
                .ask(
                    &next_prompt,
                    AskRequest {
                        session_id: active_session_id.clone(),
                        thinking_enabled: self.options.thinking_enabled,
                        search_enabled: self.options.search_enabled,
                        timeout_ms: Some(self.options.timeout_ms),
                    },
                )
                .await?;

            if response.chat_session_id.is_some() {
                active_session_id = response.chat_session_id.clone();
            }

            let raw_response = response.response.unwrap_or_default();
            match parse_protocol_message(&raw_response) {
                Ok(AgentProtocolMessage::Final { answer }) => {
                    return Ok(AgentRunResult {
                        chat_session_id: active_session_id,
                        final_answer: answer,
                        max_steps: self.options.max_steps,
                        steps: step,
                        search_enabled: self.options.search_enabled,
                        thinking_enabled: self.options.thinking_enabled,
                    });
                }
                Ok(AgentProtocolMessage::ToolCall {
                    tool,
                    arguments,
                    reason,
                }) => {
                    next_prompt = build_tool_result_prompt(&tool, &arguments, reason.as_deref())?;
                }
                Err(_) if step < self.options.max_steps => {
                    next_prompt = build_protocol_repair_prompt(&raw_response);
                }
                Err(error) => {
                    return Err(error);
                }
            }
        }

        Err(WebLlmError::InvalidInput(format!(
            "agent did not produce a final answer within {} steps",
            self.options.max_steps
        )))
    }
}

fn build_initial_protocol_prompt(task: &str) -> String {
    format!("{AGENT_PROTOCOL_SPEC}\n\nTask:\n{task}")
}

fn build_protocol_repair_prompt(previous_response: &str) -> String {
    format!(
        "{AGENT_PROTOCOL_SPEC}\n\nYour previous response was not valid protocol JSON.\nReturn a corrected JSON object only.\n\nPrevious response:\n{previous_response}"
    )
}

fn build_tool_result_prompt(tool: &str, arguments: &Value, reason: Option<&str>) -> Result<String> {
    let tool_result = ToolCallResult {
        r#type: "tool_result",
        tool,
        arguments,
        ok: false,
        error: "tool execution is not implemented in this runtime yet",
    };
    let tool_result_json = serde_json::to_string_pretty(&tool_result)?;
    let reason_line = reason
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("\nReason:\n{value}"))
        .unwrap_or_default();

    Ok(format!(
        "{AGENT_PROTOCOL_SPEC}\n\nYour previous response requested a tool call.{reason_line}\nUse this tool result and continue.\n{tool_result_json}"
    ))
}

fn parse_protocol_message(raw_response: &str) -> Result<AgentProtocolMessage> {
    let raw_response = raw_response.trim();
    if raw_response.is_empty() {
        return Err(WebLlmError::InvalidInput(
            "agent protocol response was empty".to_owned(),
        ));
    }

    if let Ok(parsed) = serde_json::from_str::<AgentProtocolMessage>(raw_response) {
        return Ok(parsed);
    }

    if let Some(json_object) = extract_first_json_object(raw_response) {
        return serde_json::from_str::<AgentProtocolMessage>(&json_object).map_err(|error| {
            WebLlmError::InvalidInput(format!("agent protocol response was invalid json: {error}"))
        });
    }

    Err(WebLlmError::InvalidInput(
        "agent protocol response did not contain a valid json object".to_owned(),
    ))
}

fn extract_first_json_object(text: &str) -> Option<String> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    for (start_index, ch) in chars.iter().copied() {
        if ch != '{' {
            continue;
        }

        let mut depth = 0_usize;
        let mut in_string = false;
        let mut escaped = false;

        for (end_index, current) in text[start_index..].char_indices() {
            if in_string {
                if escaped {
                    escaped = false;
                    continue;
                }
                match current {
                    '\\' => escaped = true,
                    '"' => in_string = false,
                    _ => {}
                }
                continue;
            }

            match current {
                '"' => in_string = true,
                '{' => depth += 1,
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let end = start_index + end_index + current.len_utf8();
                        return Some(text[start_index..end].to_owned());
                    }
                }
                _ => {}
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, sync::Mutex};

    use async_trait::async_trait;
    use imperial_desk_core::{
        AskRequest, AskResponse, DeleteAllResult, DeleteCapable, DeleteSessionResult,
        InspectCapable, InspectRequest, InspectResult, LoginCapable, LoginRequest, LoginResult,
        ProviderCapabilities, ProviderHandle, ProviderMetadata, Result, SessionMode, WebLlmError,
        WebLlmProvider,
    };

    use super::*;

    #[derive(Default)]
    struct ScriptedProvider {
        responses: VecDeque<AskResponse>,
        prompts: Mutex<Vec<String>>,
    }

    impl ScriptedProvider {
        fn from_responses(responses: Vec<AskResponse>) -> Self {
            Self {
                responses: responses.into(),
                prompts: Mutex::new(Vec::new()),
            }
        }

        fn prompts(&self) -> Vec<String> {
            self.prompts.lock().expect("prompt mutex poisoned").clone()
        }
    }

    #[async_trait]
    impl WebLlmProvider for ScriptedProvider {
        async fn ask(&mut self, prompt: &str, _request: AskRequest) -> Result<AskResponse> {
            self.prompts
                .lock()
                .expect("prompt mutex poisoned")
                .push(prompt.to_owned());
            self.responses.pop_front().ok_or_else(|| {
                WebLlmError::InvalidInput("scripted provider ran out of responses".to_owned())
            })
        }
    }

    #[async_trait]
    impl LoginCapable for ScriptedProvider {
        async fn login(&mut self, _request: LoginRequest) -> Result<LoginResult> {
            Err(WebLlmError::unsupported_capability("login"))
        }
    }

    #[async_trait]
    impl InspectCapable for ScriptedProvider {
        async fn inspect(&mut self, _request: InspectRequest) -> Result<InspectResult> {
            Err(WebLlmError::unsupported_capability("inspect"))
        }
    }

    #[async_trait]
    impl DeleteCapable for ScriptedProvider {
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

    impl ProviderHandle for ScriptedProvider {
        fn metadata(&self) -> ProviderMetadata {
            ProviderMetadata {
                id: "scripted",
                display_name: "Scripted",
                description: "Scripted test provider",
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
    }

    fn scripted_response(chat_session_id: &str, response: &str) -> AskResponse {
        AskResponse {
            chat_session_id: Some(chat_session_id.to_owned()),
            response: Some(response.to_owned()),
            session_mode: SessionMode::Continue,
            ..AskResponse::default()
        }
    }

    #[test]
    fn parse_protocol_message_extracts_markdown_embedded_json() {
        let raw = "```json\n{\"type\":\"final\",\"answer\":\"done\"}\n```";
        let parsed = parse_protocol_message(raw).expect("protocol should parse");
        assert_eq!(
            parsed,
            AgentProtocolMessage::Final {
                answer: "done".to_owned()
            }
        );
    }

    #[tokio::test]
    async fn run_returns_final_answer_from_protocol_response() {
        let responses = vec![scripted_response(
            "session-1",
            "{\"type\":\"final\",\"answer\":\"done\"}",
        )];
        let mut provider = ScriptedProvider::from_responses(responses);
        let mut agent = WebLlmAgent::new(&mut provider, AgentOptions::default());

        let result = agent
            .run("inspect cwd", None)
            .await
            .expect("run should succeed");

        assert_eq!(result.final_answer, "done");
        assert_eq!(result.steps, 1);
        assert_eq!(result.chat_session_id.as_deref(), Some("session-1"));
        assert!(provider.prompts()[0].contains("inspect cwd"));
    }

    #[tokio::test]
    async fn run_handles_tool_call_and_feeds_back_tool_result() {
        let responses = vec![
            scripted_response(
                "session-1",
                "{\"type\":\"tool_call\",\"tool\":\"shell\",\"arguments\":{\"command\":\"pwd\"},\"reason\":\"inspect cwd\"}",
            ),
            scripted_response(
                "session-1",
                "{\"type\":\"final\",\"answer\":\"tool unavailable, stopping\"}",
            ),
        ];
        let mut provider = ScriptedProvider::from_responses(responses);
        let mut agent = WebLlmAgent::new(&mut provider, AgentOptions::default());

        let result = agent
            .run("inspect cwd", None)
            .await
            .expect("run should succeed");
        let prompts = provider.prompts();

        assert_eq!(result.final_answer, "tool unavailable, stopping");
        assert_eq!(result.steps, 2);
        assert!(prompts[1].contains("\"type\": \"tool_result\""));
        assert!(prompts[1].contains("\"tool\": \"shell\""));
    }

    #[tokio::test]
    async fn run_repairs_invalid_protocol_response() {
        let responses = vec![
            scripted_response("session-1", "not valid json"),
            scripted_response("session-1", "{\"type\":\"final\",\"answer\":\"repaired\"}"),
        ];
        let mut provider = ScriptedProvider::from_responses(responses);
        let mut agent = WebLlmAgent::new(&mut provider, AgentOptions::default());

        let result = agent
            .run("repair response", None)
            .await
            .expect("run should succeed");
        let prompts = provider.prompts();

        assert_eq!(result.final_answer, "repaired");
        assert_eq!(result.steps, 2);
        assert!(prompts[1].contains("not valid protocol JSON"));
        assert!(prompts[1].contains("Return a corrected JSON object only"));
    }

    #[tokio::test]
    async fn run_errors_when_max_steps_is_exhausted() {
        let responses = vec![scripted_response(
            "session-1",
            "{\"type\":\"tool_call\",\"tool\":\"shell\",\"arguments\":{\"command\":\"pwd\"}}",
        )];
        let mut provider = ScriptedProvider::from_responses(responses);
        let mut agent = WebLlmAgent::new(
            &mut provider,
            AgentOptions {
                max_steps: 1,
                ..AgentOptions::default()
            },
        );

        let error = agent
            .run("inspect cwd", None)
            .await
            .expect_err("run should fail");

        assert!(
            matches!(error, WebLlmError::InvalidInput(message) if message.contains("final answer within 1 steps"))
        );
    }
}
