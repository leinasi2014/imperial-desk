# Web LLM Adapter Rust Design

## Goal

Build a Rust implementation of the existing web-LLM adapter with the same core product shape:

- provider-agnostic CLI entry
- web-session based LLM access
- explicit session continuation via session id
- local tool coordinator for agent mode
- profile-backed login reuse
- room to add more web-model providers later

This document defines the initial architecture for the Rust version before implementation starts.

## Scope

Phase 1 target:

- implement one provider: `deepseek-web`
- support `login`, `ask`, `agent`, `inspect`
- persist browser profile and last-session metadata
- preserve the current JS behavior where plain `ask` starts a fresh session by default
- keep agent mode provider-agnostic

Phase 1 non-goals:

- full parity for every delete/history endpoint on day one
- WSL forwarding bridge
- plugin loading from dynamic libraries
- distributed multi-process coordinator

## Product Constraints

The Rust version should preserve these user-facing rules:

- `ask` without `--session-id` creates a new web chat
- `ask --session-id <id>` continues that exact conversation
- reasoning/thinking defaults on
- smart search defaults off
- the agent loop is generic and should not know DeepSeek-specific DOM details
- adding a second provider should not require changing the coordinator core

## Architecture Summary

The system is split into five layers:

1. CLI layer
2. Provider registry
3. Provider adapters
4. Agent coordinator
5. Browser backend

Provider adapters own site-specific behavior.
The coordinator owns tool-loop behavior.
The browser backend owns page automation, persistent profile launch, DOM interaction, and network/event capture.

## Workspace Layout

This project should follow a Rust workspace layout similar to `D:\workspaces\openfang\crates`:

- top-level workspace manifest
- implementation crates grouped under `crates/`
- one responsibility per crate
- the CLI crate remains thin and depends on lower-level crates

### Proposed Workspace Layout

```text
web-llm-adapter-rust/
  Cargo.toml
  Cargo.lock
  crates/
    web-llm-cli/
      Cargo.toml
      src/
    web-llm-core/
      Cargo.toml
      src/
    web-llm-agent/
      Cargo.toml
      src/
    web-llm-browser/
      Cargo.toml
      src/
    web-llm-state/
      Cargo.toml
      src/
    web-llm-provider/
      Cargo.toml
      src/
        lib.rs
        registry.rs
        providers/
          mod.rs
          deepseek/
            mod.rs
            common/
              mod.rs
              models.rs
              session.rs
            web/
              mod.rs
              provider.rs
              selectors.rs
              parser.rs
              mutations.rs
            api/
              mod.rs
              provider.rs
              client.rs
  docs/
    design.md
```

### Crate Responsibilities

`web-llm-cli`

- command parsing
- output formatting
- provider selection
- wiring dependencies together

`web-llm-core`

- shared request/response types
- provider traits
- capability metadata
- common errors
- configuration models

`web-llm-agent`

- provider-agnostic coordinator
- protocol parser and validator
- local tool registry
- step trace model

`web-llm-browser`

- browser backend traits
- Chromium/CDP implementation
- DOM helpers
- network capture helpers

`web-llm-state`

- local session state persistence
- profile/state path resolution
- JSON state schema and migrations

`web-llm-provider`

- provider registry
- provider id to implementation mapping
- vendor-specific implementations grouped by provider
- split per access mode such as `web` and `api`
- shared provider-local code such as auth/session helpers

### Provider Internal Layout

Use one provider crate and keep vendor code inside it:

```text
crates/web-llm-provider/src/
  lib.rs
  registry.rs
  providers/
    mod.rs
    deepseek/
      mod.rs
      common/
        mod.rs
        models.rs
        session.rs
      web/
        mod.rs
        provider.rs
        selectors.rs
        parser.rs
        mutations.rs
      api/
        mod.rs
        provider.rs
        client.rs
```

Design rules for this crate:

- one vendor gets one top-level module under `providers/`
- one transport gets one submodule under the vendor, such as `web` or `api`
- `common/` holds vendor-specific logic shared by multiple transports
- the registry exports multiple provider ids from one vendor, for example `deepseek-web` and `deepseek-api`
- adding `deepseek-api` should not require moving DeepSeek code across crates again

### Why This Layout

This layout is preferred because:

- it matches the `openfang/crates` style you referenced
- it keeps the workspace small while still leaving room for many providers
- it lets one vendor expose multiple provider ids such as `deepseek-web` and `deepseek-api`
- browser automation can evolve independently from provider logic
- coordinator logic stays reusable for non-DeepSeek providers
- the CLI binary remains thin and easy to replace later

Rust structure rules:

- keep binary crates thin
- keep cross-crate contracts in `web-llm-core`
- keep provider-specific code out of the coordinator crate
- keep browser backend code out of provider registry code
- prefer adding a new vendor module before adding a new crate
- only split a vendor back out into its own crate if compile times or ownership boundaries become painful

## Core Traits

### Provider Trait

The coordinator only needs a minimal provider contract:

```rust
#[async_trait::async_trait]
pub trait WebLlmProvider: Send {
    async fn ask(&mut self, prompt: &str, request: AskRequest) -> Result<AskResponse>;
}
```

Command-specific capabilities should be modeled explicitly:

```rust
#[async_trait::async_trait]
pub trait LoginCapable {
    async fn login(&mut self, request: LoginRequest) -> Result<LoginResult>;
}

#[async_trait::async_trait]
pub trait InspectCapable {
    async fn inspect(&mut self, request: InspectRequest) -> Result<InspectResult>;
}

#[async_trait::async_trait]
pub trait DeleteCapable {
    async fn delete_session(&mut self, session_id: &str) -> Result<DeleteSessionResult>;
    async fn delete_current_session(&mut self) -> Result<DeleteSessionResult>;
    async fn delete_all_history(&mut self) -> Result<DeleteAllResult>;
}
```

Registry metadata should be static:

```rust
pub struct ProviderDefinition {
    pub id: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub capabilities: ProviderCapabilities,
    pub factory: fn(&ProviderOptions) -> Result<Box<dyn ProviderHandle>>,
}
```

The CLI should query capabilities before invoking a command.

### Agent Coordinator

The coordinator remains provider-agnostic.

Responsibilities:

- build the initial JSON-only protocol prompt
- parse model responses
- execute one local tool at a time
- feed tool results back into the same provider session
- stop on `final`
- stop on `max_steps`

### Browser Backend Trait

Provider adapters should not depend directly on a concrete browser engine.

```rust
#[async_trait::async_trait]
pub trait BrowserBackend: Send {
    async fn goto(&mut self, url: &str) -> Result<()>;
    async fn click_first_visible(&mut self, selectors: &[&str]) -> Result<bool>;
    async fn fill_first_visible(&mut self, selectors: &[&str], text: &str) -> Result<()>;
    async fn body_text(&mut self) -> Result<String>;
    async fn evaluate_json<T: serde::de::DeserializeOwned>(
        &mut self,
        script: &str,
    ) -> Result<T>;
    async fn current_url(&mut self) -> Result<String>;
}
```

This keeps provider code testable and makes browser backend replacement possible.

## Interactive Login Design

The CLI should support both scripted login and a guided interactive login flow.

Primary goal:

- make `web` providers usable without requiring users to manually operate the browser unless the site forces a challenge step

Secondary goals:

- keep provider ids stable for automation
- keep the interactive UX grouped by vendor and transport
- support multiple login methods per web provider
- preserve room for future vendors beyond DeepSeek

### Login Entry Modes

The system should support two entry modes:

1. interactive login wizard
2. direct non-interactive login flags

Interactive mode is the default for humans:

```text
web-llm login
```

Direct mode is for scripts and agents:

```text
web-llm login --provider deepseek-web --method wechat
web-llm login --provider deepseek-web --method phone-sms
```

The underlying provider id remains transport-specific, such as `deepseek-web`.
The interactive wizard only changes how the CLI presents choices.

### Interactive Login Wizard Flow

The TUI should guide the user through a small focused flow:

1. select vendor
2. select transport
3. select login method
4. enter the provider-specific login screen

For DeepSeek the flow should look like:

1. `DeepSeek`
2. `Web` or `API`
3. if `Web`, choose:
   - `WeChat QR`
   - `Phone SMS`

If `API` is chosen before it exists, the UI should show that the route is reserved but not implemented.

The wizard is a presentation layer only.
Internally the resolved target should still become a concrete provider id plus login method:

- provider id: `deepseek-web`
- login method: `wechat`

### Login State Model

The current login state is too small for multi-step and multi-method flows.
The design should evolve `LoginRequest` and `LoginResult` so the provider can explicitly tell the CLI what the next screen should be.

Target shape:

```rust
pub enum LoginMethod {
    Wechat,
    PhoneSms,
}

pub enum LoginState {
    LoggedIn,
    QrCodeReady,
    VerificationCodeRequired,
    ExternalActionRequired,
}
```

Intent of these states:

- `LoggedIn`: browser session is authenticated and chat input is available
- `QrCodeReady`: provider has prepared a QR code image or renderable payload
- `VerificationCodeRequired`: provider has already submitted the phone number and is waiting for the SMS code
- `ExternalActionRequired`: the provider hit a blocking site challenge such as a slider or image captcha and needs the user to act in the browser

The CLI should react to state transitions instead of hard-coding DeepSeek-specific assumptions.

### WeChat QR Login Design

This should become the preferred DeepSeek web login path.

Reasoning:

- the login page exposes a stable WeChat QR `iframe`
- the QR area can be captured as an element screenshot
- QR scanning avoids entering phone numbers and SMS verification codes
- it also avoids part of the image-captcha friction seen in the phone flow

Observed DeepSeek page facts:

- the WeChat login area is rendered as an `iframe`
- the `iframe` source is under `https://open.weixin.qq.com/connect/qrconnect`
- the QR area can be screenshot directly from the browser automation layer

Provider flow:

1. open `https://chat.deepseek.com/sign_in`
2. detect the WeChat QR `iframe`
3. capture the QR area as an image
4. return `LoginState::QrCodeReady`
5. keep the browser session alive and poll for successful login
6. once the chat prompt appears, return `LoginState::LoggedIn`

CLI flow:

1. user chooses `DeepSeek -> Web -> WeChat QR`
2. CLI asks provider to prepare WeChat login
3. provider returns QR image data
4. TUI renders the QR code centered in an OpenFang-style panel
5. footer shows status such as:
   - `waiting for scan`
   - `scan detected, confirm in WeChat`
   - `login completed`
6. if login succeeds, the TUI exits cleanly

### QR Rendering Strategy

The QR image should be rendered inside the terminal UI without requiring the user to inspect the browser window.

Preferred rendering pipeline:

1. provider captures the QR area as PNG bytes
2. CLI converts the PNG to a terminal-friendly representation
3. TUI draws the representation in a centered panel

Rendering fallback order:

1. block-rendered QR using black and white cell blocks
2. if image decode or render fails, print a clear message and keep the browser window open as fallback

The design should not depend on cross-origin DOM access inside the WeChat `iframe`.
Element screenshot is preferred because it works even when inner-frame DOM is restricted.

### Phone SMS Login Design

Phone SMS remains a secondary login method.

Flow:

1. CLI resolves the phone number from:
   - `login --phone`
   - provider-derived env var such as `WEB_LLM_DEEPSEEK_WEB_PHONE`
   - optional compatibility alias such as `WEB_LLM_DEEPSEEK_PHONE`
   - provider config file on disk
2. provider opens the DeepSeek sign-in page
3. provider fills the phone number in the background
4. provider clicks `发送验证码`
5. if the site immediately asks for a human challenge, return `ExternalActionRequired`
6. after the challenge is cleared and the page is ready, return `VerificationCodeRequired`
7. CLI opens a TUI input screen for the SMS code
8. user types the code in the terminal
9. provider submits `登录` and waits for the normal chat input to appear

Observed risk:

- the phone route may trigger an image challenge after clicking `发送验证码`

Because of that risk, phone login should be documented as a fallback path, not the preferred first path.

Rules:

- the phone number may be stored locally for convenience
- the verification code must never be stored on disk
- the second login step should reuse the existing browser session when possible
- if the browser is already logged in, `login` should return `LoggedIn` immediately
- if a challenge interrupts the flow, the browser should remain open and the TUI should tell the user exactly what to do

### Local Phone Number Storage

Provider-specific config should live next to other local state:

```text
%LOCALAPPDATA%/web-llm-adapter-rust/deepseek-web/provider-config.json
```

Example:

```json
{
  "phone_number": "13800138000"
}
```

The CLI should expose:

- `config show`
- `config set-phone --phone <number>`
- `config clear-phone`

### TUI Screen Design

The login TUI should borrow the visual direction from `openfang-cli`:

- orange accent
- charcoal background
- muted stone body text
- rounded bordered panels
- clear footer key hints

Initial screens:

1. vendor selection
2. transport selection
3. login method selection
4. WeChat QR display
5. SMS verification input
6. external action required

The UI should stay small and single-purpose.
This is not a dashboard.

### Browser Ownership During Login

The browser should remain under provider control for the full login session.

Why:

- WeChat scan success must be detected by the provider polling the page
- SMS login second step must reuse the same cookies and page state
- site challenges may need the user to complete a step in the already-open browser

The CLI owns the terminal experience.
The provider owns the browser.

### Future Provider Reuse

This login architecture should be generic enough for other web LLM vendors.

Reusable parts:

- wizard structure
- login method enum
- QR display screen
- external action required screen
- provider-driven login state transitions

Provider-specific parts:

- login selectors
- QR location logic
- success detection
- challenge detection

## Browser Backend Decision

Rust does not have a Playwright-equivalent with the same maturity and ergonomics.
The design should keep the browser layer swappable.

Preferred direction:

- phase 1 backend: Chromium CDP-based backend
- avoid coupling provider logic to one crate's page API
- keep browser launch, selectors, JS evaluation, and network observation behind `BrowserBackend`

Backend candidates:

1. `chromiumoxide`
   - strongest fit for direct Chrome DevTools access
   - useful for response/network event capture
   - best fit for persistent profile launching

2. WebDriver-based backend such as `fantoccini` or `thirtyfour`
   - weaker fit for network event capture
   - easier portability if a remote driver is desired later

Decision:

- design for CDP first
- keep a backend trait so this choice is reversible

## Session And State Model

Two state concepts should remain separate:

1. browser profile state
2. local convenience state

### Browser Profile State

Persistent profile directory stores:

- login cookies
- local storage
- site session artifacts

This should be provider-specific by default:

```text
.web-llm-adapter/
  profiles/
    deepseek-web/
```

### Local Convenience State

A small JSON file stores only the last known chat session for convenience commands:

```json
{
  "version": 1,
  "lastSession": {
    "id": "uuid",
    "url": "https://...",
    "updatedAt": "2026-03-11T12:00:00Z"
  }
}
```

This file must not become implicit persistence for normal `ask`.

## Data Types

### AskRequest

```rust
pub struct AskRequest {
    pub session_id: Option<String>,
    pub thinking_enabled: bool,
    pub search_enabled: bool,
    pub timeout: std::time::Duration,
}
```

### AskResponse

```rust
pub struct AskResponse {
    pub url: String,
    pub chat_session_id: Option<String>,
    pub model: Option<String>,
    pub prompt: String,
    pub requested_session_id: Option<String>,
    pub response: Option<String>,
    pub reasoning: Option<String>,
    pub session_mode: SessionMode,
    pub search_enabled: bool,
    pub thinking_enabled: bool,
}
```

### Agent Protocol

The same protocol shape should be preserved:

- `tool_call`
- `final`

```json
{"type":"tool_call","tool":"shell","arguments":{"command":"pwd"},"reason":"inspect cwd"}
```

```json
{"type":"final","answer":"done"}
```

## CLI Surface

Phase 1 CLI should expose:

- `providers`
- `login`
- `ask`
- `agent`
- `inspect`

Phase 2:

- `delete`
- `delete-current`
- `delete-all`

Global options:

- `--provider`
- `--profile-dir`
- `--base-url`
- `--headed`

`login` behavior:

- `login` without flags should launch the interactive wizard
- `login --provider <id> --method <method>` should bypass the wizard
- `login --provider deepseek-web --method wechat` should open the WeChat QR flow
- `login --provider deepseek-web --method phone-sms` should open the SMS flow
- `login --phone <number>` remains valid only for the SMS path
- `login --headed` should keep the browser visible for challenge handling and debugging

`ask` options:

- `--json`
- `--thinking`
- `--search`
- `--session-id`
- `--timeout-ms`

`agent` options:

- `--json`
- `--thinking`
- `--search`
- `--session-id`
- `--timeout-ms`
- `--max-steps`
- `--max-protocol-retries`
- `--tools`
- `--workdir`

## Tool Execution Layer

Initial built-in tools:

- `shell`
- `read_file`
- `write_file`
- `list_files`

Rules:

- tools execute locally, not inside the provider adapter
- shell must be non-interactive
- output should be truncated to a configured cap
- file tools must resolve paths relative to a supplied workdir

Later extension path:

- register tools by name in a `ToolRegistry`
- support `Fn`-based or trait-object tool implementations
- keep serialization boundary stable so external tools can be added later

## DeepSeek Provider Design

DeepSeek should live inside `web-llm-provider::providers::deepseek`.

It should expose at least two provider ids over time:

- `deepseek-web`
- `deepseek-api`

The DeepSeek web provider should map cleanly from the current JS implementation.

Responsibilities:

- launch browser with persistent profile
- open the sign-in route when login is requested
- support multiple login methods for the same web provider
- detect and capture the WeChat QR area
- keep the browser session alive while the CLI renders QR or SMS prompts
- detect challenge states that require human action
- open requested conversation URL or base URL
- optionally click `new chat`
- toggle thinking/search controls
- fill prompt and submit
- wait for response stabilization
- parse either:
  - network event stream from completion endpoint
  - DOM fallback from rendered assistant message
- extract current session id from URL
- persist last-session convenience state

DeepSeek-specific selectors and parsing logic should live under `providers/deepseek/web/`, not in shared coordinator code.

DeepSeek API-specific request building, auth, and streaming logic should live under `providers/deepseek/api/`.

## Error Handling

Rust error policy:

- application layer may use `anyhow`
- library/provider error types should use `thiserror`
- avoid `unwrap` and `expect` in runtime code
- add context to all filesystem, browser, and network failures

Suggested top-level error categories:

- provider not found
- provider capability unsupported
- browser launch failure
- login/session unavailable
- selector not found
- response timeout
- protocol parse failure
- tool execution failure
- filesystem state corruption

## Async Model

Use Tokio.

Reasons:

- browser automation will be async
- CLI commands can share runtime patterns
- future provider implementations may need concurrent event handling

Rules:

- avoid holding locks across `.await`
- prefer message passing over shared mutable global state
- keep provider instances single-owner where possible

## Serialization And Config

Use:

- `serde`
- `serde_json`
- `camino` or `std::path::PathBuf` for paths

Configuration priority:

1. CLI flags
2. env vars
3. provider defaults

Potential env vars:

- `WEB_LLM_PROVIDER`
- `WEB_LLM_PROFILE_DIR`
- `WEB_LLM_BASE_URL`

## Logging And Diagnostics

Use structured logging from the start.

Suggested stack:

- `tracing`
- `tracing-subscriber`

Log categories:

- provider lifecycle
- browser lifecycle
- session selection
- tool execution
- protocol repair retries

Default CLI output should remain quiet.
Verbose logs should be behind an opt-in flag or environment variable.

## Testing Strategy

### Unit Tests

Test:

- session state normalization
- protocol JSON extraction
- protocol validation
- tool argument validation
- path resolution

### Integration Tests

Test:

- CLI parsing
- provider registry
- fake-provider agent loop

### Browser Tests

Use a fake provider and fake browser backend first.
Real site integration should be gated behind explicit opt-in because the web UI is unstable and requires a real login state.

## Security Notes

The agent loop can execute local commands.

Required safeguards:

- explicit tool allowlist
- explicit `workdir`
- bounded output capture
- non-interactive shell execution
- no silent privilege escalation behavior

Future hardening options:

- denylist dangerous shell patterns
- read-only mode
- no-shell mode
- path sandboxing

## Implementation Plan

### Milestone 1

- create workspace manifest and `crates/` layout
- add CLI skeleton
- add provider registry
- add type definitions
- add fake provider for tests

### Milestone 2

- implement generic agent coordinator
- implement built-in tool registry
- add protocol tests

### Milestone 3

- implement `deepseek-web` inside `web-llm-provider`
- keep DeepSeek shared logic under `providers/deepseek/common`
- implement DeepSeek web flow on top of a CDP browser backend
- implement `login`, `ask`, `inspect`
- persist last-session state

### Milestone 4

- add delete/history endpoints
- add richer diagnostics
- add Windows/WSL interop story if still needed

### Milestone 5

- add `deepseek-api` to validate the vendor-plus-transport layout
- optionally add a second vendor after that

## Open Questions

1. Should the first Rust backend be fully native CDP, or is a temporary external browser bridge acceptable for faster parity?
2. Should the first workspace keep only the essential crates, or create the full planned crate split immediately?
3. Should destructive history-delete commands wait until after stable provider capability modeling lands?
4. Should tool execution eventually be split into a separate crate so non-web providers can reuse it?

## Initial Decision Summary

Initial decisions for implementation:

- build as a workspace with thin binary and reusable library crates
- keep provider registry and agent coordinator generic from day one
- implement DeepSeek first
- design browser interaction behind a trait
- prefer CDP-backed browser automation
- keep default `ask` stateless/new-session
- keep tool execution local and explicitly allowlisted
