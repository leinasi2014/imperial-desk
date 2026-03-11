use std::time::{Duration, Instant};

use async_trait::async_trait;
use imperial_desk_browser::{create_default_browser_backend, BrowserBackend, BrowserLaunchOptions};
use imperial_desk_core::{
    AskRequest, AskResponse, DeleteAllResult, DeleteCapable, DeleteSessionResult, InspectCapable,
    InspectRequest, InspectResult, LoginCapable, LoginRequest, LoginResult, LoginState,
    ProviderCapabilities, ProviderDefinition, ProviderHandle, ProviderMetadata, ProviderOptions,
    Result, SessionMode, WebLlmError, WebLlmProvider,
};
use imperial_desk_state::{clear_recent_session, StatePaths};
use serde_json::json;
use tokio::time::sleep;

use crate::providers::deepseek::{
    common::{
        models::DEFAULT_BASE_URL,
        session::{
            build_session_url, clear_recent_session_if_matches, extract_session_id_from_url,
            load_required_recent_session_id, remember_recent_session, sanitize_session_id,
        },
    },
    web::{
        mutations::run_authorized_mutation,
        parser::{detect_model_in_text, sanitize_response},
        selectors::{
            INPUT_SELECTORS, INSPECT_NEW_CHAT_SELECTORS, SEARCH_LABELS, SEND_BUTTON_SELECTORS,
            THINKING_LABELS,
        },
    },
};

pub const PROVIDER_ID: &str = "deepseek-web";
const DEFAULT_ASK_TIMEOUT_MS: u64 = 180_000;
const DEFAULT_INSPECT_WAIT_MS: u64 = 5_000;
const DEFAULT_LOGIN_TIMEOUT_MS: u64 = 300_000;
const POLL_INTERVAL_MS: u64 = 1_500;
const RESPONSE_STABLE_MS: u64 = 3_000;
const PHONE_INPUT_PLACEHOLDER: &str = "请输入手机号";
const CODE_INPUT_PLACEHOLDER: &str = "请输入验证码";
const SEND_CODE_BUTTON_TEXT: &str = "发送验证码";
const LOGIN_BUTTON_TEXT: &str = "登录";

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoginSurface {
    ChatReady,
    SignInForm,
}

pub struct DeepseekWebProvider {
    browser: Option<Box<dyn BrowserBackend>>,
    metadata: ProviderMetadata,
    state_paths: StatePaths,
    launch_options: BrowserLaunchOptions,
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
        display_name: "DeepSeek Web",
        description: "DeepSeek web chat adapter backed by Chromium CDP",
        capabilities: ProviderCapabilities {
            ask: true,
            agent: true,
            login: true,
            inspect: true,
            delete_session: true,
            delete_current: true,
            delete_all: true,
        },
    }
}

fn create_provider(options: ProviderOptions) -> Result<Box<dyn ProviderHandle>> {
    let state_paths = StatePaths::resolve(PROVIDER_ID, options.profile_dir.as_deref())?;
    state_paths.ensure_layout()?;
    let launch_options = BrowserLaunchOptions::from_provider_options(
        &options,
        DEFAULT_BASE_URL,
        state_paths.profile_dir.clone(),
    );

    Ok(Box::new(DeepseekWebProvider {
        browser: None,
        metadata: metadata(),
        state_paths,
        launch_options,
    }))
}

impl DeepseekWebProvider {
    fn normalize_login_field(value: Option<&str>) -> Option<String> {
        value
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    }

    async fn browser(&mut self) -> Result<&mut (dyn BrowserBackend + '_)> {
        if self.browser.is_none() {
            self.browser = Some(create_default_browser_backend(&self.launch_options).await?);
        }

        match self.browser.as_deref_mut() {
            Some(browser) => Ok(browser),
            None => Err(WebLlmError::browser("browser backend failed to initialize")),
        }
    }

    async fn open_home_page(&mut self) -> Result<()> {
        let base_url = self.launch_options.base_url.clone();
        self.browser().await?.goto(&base_url).await
    }

    async fn open_preferred_conversation(&mut self, session_id: Option<&str>) -> Result<()> {
        if let Some(session_id) = session_id {
            let target_url = build_session_url(&self.launch_options.base_url, session_id);
            if self.browser().await?.goto(&target_url).await.is_ok() {
                return Ok(());
            }
        }

        self.open_home_page().await
    }

    async fn wait_for_prompt_or_throw(&mut self, timeout_ms: u64) -> Result<()> {
        let started_at = Instant::now();
        while started_at.elapsed().as_millis() < u128::from(timeout_ms) {
            if self
                .browser()
                .await?
                .has_first_visible(INPUT_SELECTORS)
                .await?
            {
                return Ok(());
            }
            sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
        }

        Err(WebLlmError::browser(format!(
            "unable to find the chat input box. run `cargo run -p imperial-desk-cli -- login --provider {PROVIDER_ID} --headed` and log in manually first"
        )))
    }

    async fn wait_for_login_surface(&mut self, timeout_ms: u64) -> Result<LoginSurface> {
        let started_at = Instant::now();
        while started_at.elapsed().as_millis() < u128::from(timeout_ms) {
            if self
                .browser()
                .await?
                .has_first_visible(INPUT_SELECTORS)
                .await?
            {
                return Ok(LoginSurface::ChatReady);
            }
            if self.has_sign_in_input(PHONE_INPUT_PLACEHOLDER).await?
                || self.has_sign_in_input(CODE_INPUT_PLACEHOLDER).await?
            {
                return Ok(LoginSurface::SignInForm);
            }
            sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
        }

        Err(WebLlmError::timeout(
            "login page or chat prompt",
            timeout_ms,
        ))
    }

    async fn has_sign_in_input(&mut self, placeholder: &str) -> Result<bool> {
        let placeholder_json = serde_json::to_string(placeholder)?;
        let script = format!(
            r#"() => {{
                const placeholder = {placeholder_json};
                const isVisible = (element) => Boolean(
                    element &&
                    (element.offsetParent !== null || getComputedStyle(element).position === "fixed")
                );
                return Array.from(document.querySelectorAll('input'))
                    .some((element) => isVisible(element) && element.getAttribute('placeholder') === placeholder);
            }}"#,
        );

        Ok(self
            .browser()
            .await?
            .evaluate_json(&script)
            .await?
            .as_bool()
            .unwrap_or(false))
    }

    async fn fill_sign_in_input(&mut self, placeholder: &str, value: &str) -> Result<()> {
        let placeholder_json = serde_json::to_string(placeholder)?;
        let value_json = serde_json::to_string(value)?;
        let script = format!(
            r#"() => {{
                const placeholder = {placeholder_json};
                const value = {value_json};
                const isVisible = (element) => Boolean(
                    element &&
                    (element.offsetParent !== null || getComputedStyle(element).position === "fixed")
                );
                const input = Array.from(document.querySelectorAll('input'))
                    .find((element) => isVisible(element) && element.getAttribute('placeholder') === placeholder);
                if (!input) {{
                    return false;
                }}
                input.focus();
                const descriptor = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value');
                if (descriptor && descriptor.set) {{
                    descriptor.set.call(input, value);
                }} else {{
                    input.value = value;
                }}
                input.dispatchEvent(new Event('input', {{ bubbles: true }}));
                input.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return true;
            }}"#,
        );

        let filled = self
            .browser()
            .await?
            .evaluate_json(&script)
            .await?
            .as_bool()
            .unwrap_or(false);
        if !filled {
            return Err(WebLlmError::browser(format!(
                "unable to find sign-in input with placeholder {placeholder}"
            )));
        }

        Ok(())
    }

    async fn click_sign_in_button(&mut self, label: &str) -> Result<()> {
        let label_json = serde_json::to_string(label)?;
        let script = format!(
            r#"() => {{
                const label = {label_json};
                const isVisible = (element) => Boolean(
                    element &&
                    (element.offsetParent !== null || getComputedStyle(element).position === "fixed")
                );
                const candidates = Array.from(document.querySelectorAll('button, [role="button"]'));
                const button = candidates.find((element) => isVisible(element) && (element.textContent || '').trim().includes(label));
                if (!button) {{
                    return false;
                }}
                button.click();
                return true;
            }}"#,
        );

        let clicked = self
            .browser()
            .await?
            .evaluate_json(&script)
            .await?
            .as_bool()
            .unwrap_or(false);
        if !clicked {
            return Err(WebLlmError::browser(format!(
                "unable to find sign-in button labeled {label}"
            )));
        }

        Ok(())
    }

    async fn maybe_start_new_chat(&mut self) -> Result<()> {
        let clicked = self
            .browser()
            .await?
            .evaluate_json(
                r#"() => {
                    const isVisible = (element) => Boolean(
                        element &&
                        (element.offsetParent !== null || getComputedStyle(element).position === "fixed")
                    );
                    const candidates = Array.from(
                        document.querySelectorAll('button, a, div[role="button"]')
                    );
                    const button = candidates.find((candidate) => {
                        if (!isVisible(candidate)) {
                            return false;
                        }
                        const text = (candidate.textContent ?? "").trim();
                        return /new chat|新对话|开启新对话/i.test(text);
                    });
                    if (!button) {
                        return false;
                    }
                    button.click();
                    return true;
                }"#,
            )
            .await?
            .as_bool()
            .unwrap_or(false);
        if clicked {
            sleep(Duration::from_millis(1_000)).await;
        }

        Ok(())
    }

    async fn set_toggle_state(
        &mut self,
        toggle_name: &'static str,
        labels: &[&str],
        enabled: bool,
    ) -> Result<()> {
        let labels_json = serde_json::to_string(labels)?;
        let enabled_json = serde_json::to_string(&enabled)?;
        let script = format!(
            r#"async () => {{
                const labels = {labels_json};
                const enabled = {enabled_json};
                const isVisible = (element) => Boolean(
                    element &&
                    (element.offsetParent !== null || getComputedStyle(element).position === "fixed")
                );
                const textOf = (element) => (element.innerText || element.textContent || "").trim();
                const matchesLabel = (element) => labels.some((label) => textOf(element).includes(label));
                const isSelected = (element) =>
                    String(element.className || "").includes("ds-toggle-button--selected");

                const candidates = Array.from(
                    document.querySelectorAll('[role="button"], .ds-toggle-button')
                );
                const button = candidates.find(
                    (candidate) => isVisible(candidate) && matchesLabel(candidate)
                );
                if (!button) {{
                    return {{ found: false, selected: null }};
                }}
                if (isSelected(button) !== enabled) {{
                    button.click();
                    await new Promise((resolve) => setTimeout(resolve, 400));
                }}
                return {{ found: true, selected: isSelected(button) }};
            }}"#,
        );
        let result = self.browser().await?.evaluate_json(&script).await?;
        let found = result
            .get("found")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let selected = result.get("selected").and_then(serde_json::Value::as_bool);
        if !found {
            return Err(WebLlmError::browser(format!(
                "unable to find the {toggle_name} toggle in the DeepSeek web UI"
            )));
        }
        if selected != Some(enabled) {
            return Err(WebLlmError::browser(format!(
                "failed to switch {toggle_name} to {}",
                if enabled { "on" } else { "off" }
            )));
        }

        Ok(())
    }

    async fn extract_assistant_message(&mut self) -> Result<Option<String>> {
        let result = self
            .browser()
            .await?
            .evaluate_json(
                r#"() => {
                    const markdownTexts = Array.from(
                        document.querySelectorAll(".ds-message .ds-markdown")
                    )
                        .map((element) => element.textContent?.trim())
                        .filter(Boolean);
                    if (markdownTexts.length > 0) {
                        return markdownTexts.at(-1);
                    }

                    const messageTexts = Array.from(document.querySelectorAll(".ds-message"))
                        .map((element) => element.textContent?.trim())
                        .filter(Boolean);
                    return messageTexts.at(-1) ?? null;
                }"#,
            )
            .await?;

        Ok(result
            .as_str()
            .map(sanitize_response)
            .filter(|value| !value.is_empty()))
    }

    async fn wait_for_assistant_reply(
        &mut self,
        before_snapshot: &str,
        previous_assistant_response: Option<&str>,
        timeout_ms: u64,
    ) -> Result<Option<String>> {
        let started_at = Instant::now();
        let mut last_text = before_snapshot.to_owned();
        let mut stable_for_ms = 0_u64;

        while started_at.elapsed().as_millis() < u128::from(timeout_ms) {
            sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
            let current_text = self.browser().await?.body_text().await?;
            if current_text != last_text {
                last_text = current_text;
                stable_for_ms = 0;
            } else {
                stable_for_ms = stable_for_ms.saturating_add(POLL_INTERVAL_MS);
            }

            if let Some(response) = self.extract_assistant_message().await? {
                let is_new = previous_assistant_response
                    .map(|previous| previous != response)
                    .unwrap_or(true);
                if is_new && stable_for_ms >= RESPONSE_STABLE_MS {
                    return Ok(Some(response));
                }
            }
        }

        Err(WebLlmError::timeout("assistant reply", timeout_ms))
    }

    async fn detect_model(&mut self) -> Result<Option<String>> {
        let text = self.browser().await?.body_text().await?;
        Ok(detect_model_in_text(&text))
    }

    async fn current_session_id(&mut self) -> Result<Option<String>> {
        let url = self.browser().await?.current_url().await?;
        Ok(extract_session_id_from_url(&url))
    }

    async fn remember_last_session(
        &mut self,
        session_id: Option<&str>,
        url: Option<String>,
    ) -> Result<()> {
        remember_recent_session(&self.state_paths.recent_session_path, session_id, url)
    }
}

impl ProviderHandle for DeepseekWebProvider {
    fn metadata(&self) -> ProviderMetadata {
        self.metadata
    }
}

#[async_trait]
impl WebLlmProvider for DeepseekWebProvider {
    async fn ask(&mut self, prompt: &str, request: AskRequest) -> Result<AskResponse> {
        let prompt = prompt.trim();
        if prompt.is_empty() {
            return Err(WebLlmError::InvalidInput("prompt is empty".to_owned()));
        }

        let timeout_ms = request.timeout_ms.unwrap_or(DEFAULT_ASK_TIMEOUT_MS);
        let requested_session_id = sanitize_session_id(request.session_id.as_deref());
        let should_start_fresh_session = requested_session_id.is_none();

        self.open_preferred_conversation(requested_session_id.as_deref())
            .await?;
        self.wait_for_prompt_or_throw(timeout_ms).await?;

        if should_start_fresh_session {
            self.maybe_start_new_chat().await?;
        }

        self.set_toggle_state("thinking", THINKING_LABELS, request.thinking_enabled)
            .await?;
        self.set_toggle_state("search", SEARCH_LABELS, request.search_enabled)
            .await?;

        let before_snapshot = self.browser().await?.body_text().await?;
        let previous_assistant_response = self.extract_assistant_message().await?;
        self.browser()
            .await?
            .fill_first_visible(INPUT_SELECTORS, prompt)
            .await?;

        let submitted = self
            .browser()
            .await?
            .click_first_visible(SEND_BUTTON_SELECTORS)
            .await?;
        if !submitted
            && !self
                .browser()
                .await?
                .press_key_on_first_visible(INPUT_SELECTORS, "Enter")
                .await?
        {
            return Err(WebLlmError::browser("unable to submit prompt"));
        }

        let response = self
            .wait_for_assistant_reply(
                &before_snapshot,
                previous_assistant_response.as_deref(),
                timeout_ms,
            )
            .await?;
        let url = self.browser().await?.current_url().await?;
        let chat_session_id = self.current_session_id().await?;
        let session_mode = if requested_session_id.is_some()
            && requested_session_id.as_deref() == chat_session_id.as_deref()
        {
            SessionMode::Continue
        } else {
            SessionMode::New
        };
        let model = self.detect_model().await?;
        self.remember_last_session(chat_session_id.as_deref(), Some(url.clone()))
            .await?;

        Ok(AskResponse {
            url: Some(url),
            chat_session_id,
            model,
            prompt: Some(prompt.to_owned()),
            requested_session_id,
            response,
            reasoning: None,
            session_mode,
            search_enabled: request.search_enabled,
            thinking_enabled: request.thinking_enabled,
        })
    }
}

#[async_trait]
impl LoginCapable for DeepseekWebProvider {
    async fn login(&mut self, request: LoginRequest) -> Result<LoginResult> {
        let timeout_ms = request.timeout_ms.unwrap_or(DEFAULT_LOGIN_TIMEOUT_MS);
        if request.verification_code.is_none() || self.browser.is_none() {
            self.open_home_page().await?;
        }
        match self.wait_for_login_surface(timeout_ms).await? {
            LoginSurface::ChatReady => {
                return Ok(LoginResult {
                    state: LoginState::LoggedIn,
                });
            }
            LoginSurface::SignInForm => {}
        }

        let phone_number = Self::normalize_login_field(request.phone_number.as_deref())
            .ok_or_else(|| {
                WebLlmError::browser(format!(
                    "missing phone number. store it in {} or pass --phone",
                    self.state_paths.provider_config_path.display()
                ))
            })?;
        self.fill_sign_in_input(PHONE_INPUT_PLACEHOLDER, &phone_number)
            .await?;

        if request.verification_code.is_none() {
            self.click_sign_in_button(SEND_CODE_BUTTON_TEXT).await?;
            return Ok(LoginResult {
                state: LoginState::VerificationCodeRequired,
            });
        }

        let verification_code =
            Self::normalize_login_field(request.verification_code.as_deref())
                .ok_or_else(|| WebLlmError::browser("verification code is empty"))?;
        self.fill_sign_in_input(CODE_INPUT_PLACEHOLDER, &verification_code)
            .await?;
        self.click_sign_in_button(LOGIN_BUTTON_TEXT).await?;
        self.wait_for_prompt_or_throw(timeout_ms).await?;
        Ok(LoginResult {
            state: LoginState::LoggedIn,
        })
    }
}

#[async_trait]
impl InspectCapable for DeepseekWebProvider {
    async fn inspect(&mut self, request: InspectRequest) -> Result<InspectResult> {
        self.open_home_page().await?;
        sleep(Duration::from_millis(
            request
                .timeout_ms
                .unwrap_or(DEFAULT_INSPECT_WAIT_MS)
                .min(DEFAULT_INSPECT_WAIT_MS),
        ))
        .await;

        let input_selectors = serde_json::to_string(INPUT_SELECTORS)?;
        let send_selectors = serde_json::to_string(SEND_BUTTON_SELECTORS)?;
        let new_chat_selectors = serde_json::to_string(INSPECT_NEW_CHAT_SELECTORS)?;
        let script = format!(
            r#"() => {{
                const inputSelectors = {input_selectors};
                const sendSelectors = {send_selectors};
                const newChatSelectors = {new_chat_selectors};
                const summarize = (selector) =>
                    Array.from(document.querySelectorAll(selector))
                        .slice(0, 5)
                        .map((element) => ({{
                            selector,
                            tag: element.tagName,
                            text: (element.textContent ?? "").trim().slice(0, 120),
                            placeholder: element.getAttribute("placeholder"),
                            ariaLabel: element.getAttribute("aria-label"),
                            role: element.getAttribute("role"),
                            visible:
                                element instanceof HTMLElement
                                    ? element.offsetParent !== null ||
                                      getComputedStyle(element).position === "fixed"
                                    : true
                        }}));

                return {{
                    title: document.title,
                    url: location.href,
                    promptCandidates: inputSelectors.flatMap(summarize),
                    sendButtonCandidates: sendSelectors.flatMap(summarize),
                    newChatCandidates: newChatSelectors
                        .flatMap(summarize)
                        .filter((candidate) => /new chat|新对话|开启新对话/i.test(candidate.text)),
                    textSample: document.body?.innerText?.slice(0, 1200) ?? ""
                }};
            }}"#,
        );
        let details = self.browser().await?.evaluate_json(&script).await?;
        let url = self.browser().await?.current_url().await?;

        Ok(InspectResult {
            url: Some(url),
            details,
        })
    }
}

#[async_trait]
impl DeleteCapable for DeepseekWebProvider {
    async fn delete_session(&mut self, session_id: &str) -> Result<DeleteSessionResult> {
        let session_id = sanitize_session_id(Some(session_id))
            .ok_or_else(|| WebLlmError::InvalidInput("chat_session_id is required".to_owned()))?;
        let base_url = self.launch_options.base_url.clone();
        let result = run_authorized_mutation(
            self.browser().await?,
            &base_url,
            "/api/v0/chat_session/delete",
            json!({ "chat_session_id": session_id }),
            "failed to delete DeepSeek chat session",
        )
        .await?;
        let _status = result.status;
        clear_recent_session_if_matches(&self.state_paths.recent_session_path, &session_id)?;

        Ok(DeleteSessionResult {
            chat_session_id: session_id,
        })
    }

    async fn delete_current_session(&mut self) -> Result<DeleteSessionResult> {
        let session_id = load_required_recent_session_id(&self.state_paths.recent_session_path)?;
        self.delete_session(&session_id).await
    }

    async fn delete_all_history(&mut self) -> Result<DeleteAllResult> {
        let base_url = self.launch_options.base_url.clone();
        let result = run_authorized_mutation(
            self.browser().await?,
            &base_url,
            "/api/v0/chat_session/delete_all",
            json!({}),
            "failed to delete all DeepSeek chat history",
        )
        .await?;
        let _status = result.status;
        clear_recent_session(&self.state_paths.recent_session_path)?;

        Ok(DeleteAllResult {
            deleted_all_history: true,
        })
    }
}
