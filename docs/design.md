# Imperial Desk Design

## Purpose

This document no longer describes a purely pre-implementation design.
It records:

- the intended product shape
- the architecture already implemented in the repository
- the known gaps between the current code and the target design

The source of truth for current status is the code in this repository.
This document should stay aligned with that code.

## Naming Status

The repository and product name are now `Imperial Desk`.

That rename is already reflected at the repository level:

- repository directory: `imperial-desk`
- Plane project: `Imperial Desk`
- documentation and workflow identity: `Imperial Desk`

The current codebase now uses Imperial Desk-prefixed implementation names such as:

- crate names prefixed with `imperial-desk-`
- the CLI binary name `imperial-desk`
- the local state root directory `imperial-desk`

In this document:

- `Imperial Desk` is the project and product name
- `imperial-desk-*` names refer to current implementation identifiers in code

## Product Goal

Imperial Desk is a Rust workspace for a provider-agnostic LLM command system with:

- a shared CLI entrypoint
- web-session based access to LLM providers
- explicit session continuation via session id
- reusable browser automation and local state
- room for a generic agent coordinator and additional providers later

Conceptually, Imperial Desk is the command center for a single operator.
The current code realization of that concept is still a web-provider adapter focused on `deepseek-web`.

The current implementation focus is `deepseek-web`.

## Current Status Summary

Implemented today in the current codebase:

- Rust workspace with six crates under `crates/`
- the current CLI crate `imperial-desk-cli` with command surface for:
  - `providers`
  - `login`
  - `config`
  - `ask`
  - `agent`
  - `inspect`
  - `delete`
  - `delete-current`
  - `delete-all`
- shared core request/response types and provider capability contracts
- Chromium/CDP-backed browser abstraction
- local state persistence for profile paths, provider config, and recent session metadata
- `deepseek-web` provider with:
  - `ask`
  - phone SMS login
  - `inspect`
  - session deletion endpoints
- SMS verification TUI for the login second step

Implemented only as placeholders or partials:

- `imperial-desk-agent` is currently a thin single-ask wrapper, not a real tool-loop coordinator
- `deepseek-api` code exists as a placeholder but is not registered in the provider registry
- login wizard and WeChat QR login are not implemented
- structured logging is not integrated yet
- automated tests are effectively absent even though `cargo test` passes

## Product Constraints

The project should preserve these user-facing rules:

- `ask` without `--session-id` starts a new web chat
- `ask --session-id <id>` continues that conversation
- thinking defaults on
- search defaults off
- provider-specific DOM logic stays out of the generic coordinator
- adding a second provider should not require rewriting the coordinator core

These constraints are already partially enforced in the current codebase.

## Repository Layout

Actual repository layout:

```text
imperial-desk/
  Cargo.toml
  Cargo.lock
  crates/
    imperial-desk-cli/
    imperial-desk-core/
    imperial-desk-agent/
    imperial-desk-browser/
    imperial-desk-state/
    imperial-desk-provider/
  docs/
    design.md
```

## Crate Responsibilities

The crate names below are the current code identifiers.

`imperial-desk-cli`

- command parsing
- provider option wiring
- terminal output
- SMS verification TUI

`imperial-desk-core`

- shared request and response types
- provider contracts
- capability metadata
- common errors

`imperial-desk-agent`

- future home of the provider-agnostic multi-step tool coordinator
- currently only wraps one `ask` call

`imperial-desk-browser`

- browser backend abstraction
- Chromium/CDP implementation using `chromiumoxide`

`imperial-desk-state`

- local state path resolution
- provider config persistence
- recent session persistence

`imperial-desk-provider`

- provider registry
- vendor implementations
- DeepSeek-specific shared and transport-specific logic

## Provider Layout

The provider crate is already structured for vendor-plus-transport separation:

```text
crates/imperial-desk-provider/src/
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

Current reality:

- `deepseek-web` is implemented and registered
- `deepseek-api` has placeholder code but is not exported by the registry yet

## Architecture Summary

The implementation still follows five layers:

1. CLI layer
2. Provider registry
3. Provider adapters
4. Agent coordinator
5. Browser backend

Current status by layer:

- CLI layer: implemented
- Provider registry: implemented for `deepseek-web` only
- Provider adapters: `deepseek-web` implemented, `deepseek-api` placeholder
- Agent coordinator: scaffold only
- Browser backend: implemented with Chromium/CDP

## Current Core Contracts

The code currently uses these shapes, or close equivalents.

### AskRequest

```rust
pub struct AskRequest {
    pub session_id: Option<String>,
    pub thinking_enabled: bool,
    pub search_enabled: bool,
    pub timeout_ms: Option<u64>,
}
```

### AskResponse

```rust
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
```

### LoginState

Current code:

```rust
pub enum LoginState {
    LoggedIn,
    VerificationCodeRequired,
}
```

Still planned but not implemented:

- `QrCodeReady`
- `ExternalActionRequired`

### Provider Registry

Current registry behavior:

- default provider id is `deepseek-web`
- `provider_definitions()` returns only one provider today

Future target:

- register both `deepseek-web` and `deepseek-api`
- eventually support more vendors

## Browser Backend

The browser layer is already abstracted behind `BrowserBackend`.

Current interface includes:

- `goto`
- `has_first_visible`
- `click_first_visible`
- `fill_first_visible`
- `press_key_on_first_visible`
- `body_text`
- `evaluate_json`
- `current_url`

Current implementation:

- `ChromiumBrowserBackend` using `chromiumoxide`
- persistent user data dir support
- headed or headless launch
- Linux/WSL sandbox disabling when needed

Not implemented yet:

- network event capture
- alternative backends such as WebDriver
- screenshot-based QR extraction

## Local State Model

State is already split into two concepts:

1. browser profile state
2. local convenience state

Current files managed by `imperial-desk-state`:

- provider profile directory
- `provider-config.json`
- `recent-session.json`

Current local state root directory in code:

```text
%LOCALAPPDATA%/imperial-desk/
```

## Current CLI Surface

The current binary and command examples in code use the `imperial-desk` executable name.

Implemented commands:

- `providers`
- `login`
- `config show`
- `config set-phone --phone <number>`
- `config clear-phone`
- `ask`
- `agent`
- `inspect`
- `delete <chat_session_id>`
- `delete-current`
- `delete-all --force`

Implemented global options:

- `--provider`
- `--profile-dir`
- `--base-url`
- `--headed`

Current differences from the earlier design draft:

- `config` commands already exist
- delete commands already exist
- `login --method ...` is not implemented
- `ask --json` and `agent --json` are not implemented
- `agent --max-protocol-retries`, `--tools`, and `--workdir` are not implemented

## Current Login Design

### Implemented Today

The current login flow supports only the phone SMS path:

1. resolve phone number from CLI, env, or provider config
2. open the DeepSeek sign-in surface
3. fill the phone number
4. click `发送验证码`
5. return `VerificationCodeRequired`
6. prompt for the SMS code in a terminal UI
7. submit the code and wait for chat readiness

The current CLI forces a headed browser for `login`.

### Not Implemented Yet

- interactive vendor or transport wizard
- WeChat QR login
- challenge detection and explicit `ExternalActionRequired`
- QR rendering inside the terminal
- provider-driven multi-method login routing

## Agent Coordinator

This is the largest remaining architecture gap.

Target behavior:

- build a JSON-only protocol prompt
- parse `tool_call` and `final`
- execute local tools one at a time
- feed tool results back into the same provider session
- stop on `final` or `max_steps`

Current behavior:

- `imperial-desk-agent` validates the task
- it forwards a single `ask` call to the provider
- it returns that response as the final answer
- there is no protocol parsing, no tool execution, and no repair loop

## Tool Execution Layer

Target built-in tools remain:

- `shell`
- `read_file`
- `write_file`
- `list_files`

Current status:

- not implemented
- no `ToolRegistry`
- no workdir sandboxing in the agent layer
- no tool serialization boundary yet

## DeepSeek Web Provider

### Implemented Today

The current `deepseek-web` provider already handles:

- browser launch with persistent profile
- opening the base URL or a requested session URL
- optionally starting a new chat when no session id is supplied
- toggling thinking and search controls in the UI
- filling and submitting prompts
- waiting for reply stabilization from DOM text
- parsing the final assistant message from rendered DOM
- extracting the current session id from the URL
- storing recent session metadata
- `inspect`
- `delete_session`
- `delete_current_session`
- `delete_all_history`

### Not Implemented Yet

- WeChat QR login
- login challenge detection
- network-stream parsing of replies
- richer diagnostics and traces
- API transport parity

## DeepSeek API Provider

Current state:

- placeholder `DeepseekApiClient`
- placeholder provider returning `NotImplemented`
- not exposed by the provider registry

Target state:

- real request building and authentication
- `ask` support
- registry export
- CLI selection support

## Logging And Diagnostics

Target stack remains:

- `tracing`
- `tracing-subscriber`

Current status:

- not integrated yet
- diagnostics are mostly command errors and provider/browser error strings

## Testing Status

Current state:

- `cargo test` passes
- there are effectively zero meaningful unit or integration tests

Still needed:

- session normalization tests
- parser tests
- path resolution tests
- CLI parsing tests
- fake provider agent loop tests
- tool execution tests

## Security Notes

The eventual tool loop will execute local commands, so these safeguards remain required:

- explicit tool allowlist
- explicit workdir
- bounded output capture
- non-interactive shell execution
- no silent privilege escalation

These rules are still design requirements because the tool layer is not implemented yet.

## Implementation Status By Milestone

### Completed Or Largely Completed

- workspace bootstrap
- crate layout
- core shared contracts
- Chromium browser backend
- local state persistence
- `deepseek-web` base flow
- CLI command surface for current implemented features

### In Progress Or Still Missing

- real provider-agnostic agent coordinator
- tool execution layer
- login wizard
- WeChat QR login
- `deepseek-api`
- structured logging
- automated tests

## Immediate Next Steps

Recommended next implementation order:

1. align the agent crate with the intended protocol loop
2. implement the tool execution layer
3. add tests around core, state, parser, and fake-provider behavior
4. add login wizard and WeChat QR support
5. register and implement `deepseek-api`

## Open Questions

1. Should delete and history management remain in the same provider crate, or move behind richer capability modeling later?
2. Should network-based response capture be added before or after the generic tool loop?
3. When `deepseek-api` lands, should it share a wider contract surface with `deepseek-web`, or stay intentionally narrow?
