mod login_tui;

use std::{
    env,
    io::{self, Read},
    path::PathBuf,
};

use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, Parser, Subcommand};
use imperial_desk_agent::{AgentOptions, WebLlmAgent};
use imperial_desk_core::{
    AskRequest, InspectRequest, LoginRequest, LoginState, ProviderCapabilities, ProviderDefinition,
    ProviderOptions,
};
use imperial_desk_provider::{provider_definition, provider_definitions, DEFAULT_PROVIDER_ID};
use imperial_desk_state::{
    clear_provider_config, load_provider_config, save_provider_config, ProviderConfig, StatePaths,
};
use login_tui::prompt_verification_code;

#[derive(Parser)]
#[command(
    name = "imperial-desk",
    version,
    about = "Imperial Desk CLI for provider-agnostic LLM access"
)]
struct Cli {
    #[arg(global = true, long, default_value = DEFAULT_PROVIDER_ID)]
    provider: String,
    #[arg(global = true, long)]
    profile_dir: Option<PathBuf>,
    #[arg(global = true, long)]
    base_url: Option<String>,
    #[arg(global = true, long, default_value_t = false)]
    headed: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Providers,
    Login {
        #[arg(long)]
        timeout_ms: Option<u64>,
        #[arg(long)]
        phone: Option<String>,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Ask {
        prompt: Option<String>,
        #[arg(long, action = ArgAction::Set, default_value_t = true)]
        thinking: bool,
        #[arg(long, action = ArgAction::Set, default_value_t = false)]
        search: bool,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long)]
        timeout_ms: Option<u64>,
    },
    Agent {
        prompt: Option<String>,
        #[arg(long, action = ArgAction::Set, default_value_t = true)]
        thinking: bool,
        #[arg(long, action = ArgAction::Set, default_value_t = false)]
        search: bool,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long, default_value_t = 8)]
        max_steps: usize,
        #[arg(long)]
        timeout_ms: Option<u64>,
    },
    Inspect {
        #[arg(long)]
        timeout_ms: Option<u64>,
    },
    Delete {
        chat_session_id: String,
    },
    DeleteCurrent,
    DeleteAll {
        #[arg(long, default_value_t = false)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    Show,
    SetPhone {
        #[arg(long)]
        phone: String,
    },
    ClearPhone,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let provider_id = cli.provider.clone();
    let common_provider_options = provider_options(&cli);

    match cli.command {
        Command::Providers => {
            for definition in provider_definitions() {
                let metadata = definition.metadata;
                println!(
                    "{}: {} [{}]",
                    metadata.id,
                    metadata.description,
                    format_capabilities(metadata.capabilities)
                );
            }
        }
        Command::Login { timeout_ms, phone } => {
            let mut login_provider_options = common_provider_options.clone();
            login_provider_options.headed = true;
            let state_paths = resolve_state_paths(&provider_id, &common_provider_options)?;
            let resolved_phone = resolve_phone_number(phone, &provider_id, &state_paths)?;
            let mut provider = resolve_provider_definition(&provider_id)?
                .create(login_provider_options)
                .context("failed to create provider")?;
            ensure_capability(provider.metadata().capabilities.login, "login")?;
            let first_step = provider
                .login(LoginRequest {
                    timeout_ms,
                    phone_number: resolved_phone.clone(),
                    verification_code: None,
                })
                .await
                .context("provider login failed")?;
            match first_step.state {
                LoginState::LoggedIn => {
                    println!("login flow completed");
                }
                LoginState::VerificationCodeRequired => {
                    let phone = resolved_phone.ok_or_else(|| {
                        anyhow!(
                            "login requires a phone number. set it with `imperial-desk config set-phone --provider {provider_id} --phone <number>` or pass `--phone`"
                        )
                    })?;
                    let code = prompt_verification_code(&phone)?;
                    let second_step = provider
                        .login(LoginRequest {
                            timeout_ms,
                            phone_number: Some(phone),
                            verification_code: Some(code),
                        })
                        .await
                        .context("provider verification failed")?;
                    if second_step.state != LoginState::LoggedIn {
                        return Err(anyhow!("provider login did not complete"));
                    }
                    println!("login flow completed");
                }
            }
        }
        Command::Config { command } => {
            let state_paths = resolve_state_paths(&provider_id, &common_provider_options)?;
            match command {
                ConfigCommand::Show => {
                    let config = load_provider_config(&state_paths.provider_config_path)
                        .context("failed to load provider config")?;
                    let masked_phone = config
                        .and_then(|value| value.phone_number)
                        .map(|value| mask_phone(&value))
                        .unwrap_or_else(|| "<not set>".to_owned());
                    println!("provider: {provider_id}");
                    println!(
                        "config_path: {}",
                        state_paths.provider_config_path.display()
                    );
                    println!("phone_number: {masked_phone}");
                }
                ConfigCommand::SetPhone { phone } => {
                    let normalized_phone = normalize_phone_number(&phone)
                        .ok_or_else(|| anyhow!("phone number is empty"))?;
                    save_provider_config(
                        &state_paths.provider_config_path,
                        &ProviderConfig {
                            phone_number: Some(normalized_phone.clone()),
                        },
                    )
                    .context("failed to save provider config")?;
                    println!(
                        "saved phone number for {provider_id} at {}",
                        state_paths.provider_config_path.display()
                    );
                }
                ConfigCommand::ClearPhone => {
                    clear_provider_config(&state_paths.provider_config_path)
                        .context("failed to clear provider config")?;
                    println!(
                        "cleared stored phone number for {provider_id} at {}",
                        state_paths.provider_config_path.display()
                    );
                }
            }
        }
        Command::Ask {
            prompt,
            thinking,
            search,
            session_id,
            timeout_ms,
        } => {
            let prompt = read_prompt(prompt)?;
            let mut provider = resolve_provider_definition(&provider_id)?
                .create(common_provider_options.clone())
                .context("failed to create provider")?;
            ensure_capability(provider.metadata().capabilities.ask, "ask")?;
            let response = provider
                .ask(
                    &prompt,
                    AskRequest {
                        session_id,
                        thinking_enabled: thinking,
                        search_enabled: search,
                        timeout_ms,
                    },
                )
                .await
                .context("provider ask failed")?;
            if let Some(answer) = response.response {
                println!("{answer}");
            }
        }
        Command::Agent {
            prompt,
            thinking,
            search,
            session_id,
            max_steps,
            timeout_ms,
        } => {
            let prompt = read_prompt(prompt)?;
            let mut provider = resolve_provider_definition(&provider_id)?
                .create(common_provider_options.clone())
                .context("failed to create provider")?;
            ensure_capability(provider.metadata().capabilities.agent, "agent")?;
            let mut agent = WebLlmAgent::new(
                provider.as_mut(),
                AgentOptions {
                    max_steps,
                    search_enabled: search,
                    thinking_enabled: thinking,
                    timeout_ms: timeout_ms.unwrap_or(180_000),
                },
            );
            let result = agent
                .run(&prompt, session_id)
                .await
                .context("agent run failed")?;
            println!("{}", result.final_answer);
        }
        Command::Inspect { timeout_ms } => {
            let mut provider = resolve_provider_definition(&provider_id)?
                .create(common_provider_options.clone())
                .context("failed to create provider")?;
            ensure_capability(provider.metadata().capabilities.inspect, "inspect")?;
            let result = provider
                .inspect(InspectRequest { timeout_ms })
                .await
                .context("provider inspect failed")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Delete { chat_session_id } => {
            let mut provider = resolve_provider_definition(&provider_id)?
                .create(common_provider_options.clone())
                .context("failed to create provider")?;
            ensure_capability(
                provider.metadata().capabilities.delete_session,
                "delete_session",
            )?;
            let result = provider
                .delete_session(&chat_session_id)
                .await
                .context("provider delete_session failed")?;
            println!("deleted session {}", result.chat_session_id);
        }
        Command::DeleteCurrent => {
            let mut provider = resolve_provider_definition(&provider_id)?
                .create(common_provider_options.clone())
                .context("failed to create provider")?;
            ensure_capability(
                provider.metadata().capabilities.delete_current,
                "delete_current",
            )?;
            let result = provider
                .delete_current_session()
                .await
                .context("provider delete_current_session failed")?;
            println!("deleted current session {}", result.chat_session_id);
        }
        Command::DeleteAll { force } => {
            if !force {
                return Err(anyhow!("refusing to delete all history without --force"));
            }

            let mut provider = resolve_provider_definition(&provider_id)?
                .create(common_provider_options.clone())
                .context("failed to create provider")?;
            ensure_capability(provider.metadata().capabilities.delete_all, "delete_all")?;
            provider
                .delete_all_history()
                .await
                .context("provider delete_all_history failed")?;
            println!("deleted all provider history");
        }
    }

    Ok(())
}

fn provider_options(cli: &Cli) -> ProviderOptions {
    ProviderOptions {
        base_url: cli.base_url.clone(),
        profile_dir: cli.profile_dir.clone(),
        headed: cli.headed,
    }
}

fn resolve_provider_definition(provider_id: &str) -> Result<ProviderDefinition> {
    provider_definition(provider_id).ok_or_else(|| anyhow!("unknown provider: {provider_id}"))
}

fn resolve_state_paths(provider_id: &str, options: &ProviderOptions) -> Result<StatePaths> {
    StatePaths::resolve(provider_id, options.profile_dir.as_deref()).map_err(anyhow::Error::from)
}

fn resolve_phone_number(
    cli_phone: Option<String>,
    provider_id: &str,
    state_paths: &StatePaths,
) -> Result<Option<String>> {
    if let Some(phone) = cli_phone.as_deref().and_then(normalize_phone_number) {
        return Ok(Some(phone));
    }

    for env_var in phone_env_var_candidates(provider_id) {
        if let Some(phone) = env::var(&env_var)
            .ok()
            .as_deref()
            .and_then(normalize_phone_number)
        {
            return Ok(Some(phone));
        }
    }

    let config = load_provider_config(&state_paths.provider_config_path)?;
    Ok(config.and_then(|value| value.phone_number))
}

fn phone_env_var_candidates(provider_id: &str) -> Vec<String> {
    let normalized_provider_id = provider_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    let mut candidates = vec![format!("IMPERIAL_DESK_{normalized_provider_id}_PHONE")];
    if provider_id == "deepseek-web" {
        candidates.push("IMPERIAL_DESK_DEEPSEEK_PHONE".to_owned());
    }
    candidates
}

fn ensure_capability(enabled: bool, capability: &str) -> Result<()> {
    if enabled {
        return Ok(());
    }

    Err(anyhow!("provider does not support capability {capability}"))
}

fn format_capabilities(capabilities: ProviderCapabilities) -> String {
    let mut items = Vec::new();
    if capabilities.ask {
        items.push("ask");
    }
    if capabilities.agent {
        items.push("agent");
    }
    if capabilities.login {
        items.push("login");
    }
    if capabilities.inspect {
        items.push("inspect");
    }
    if capabilities.delete_session {
        items.push("delete_session");
    }
    if capabilities.delete_current {
        items.push("delete_current");
    }
    if capabilities.delete_all {
        items.push("delete_all");
    }
    items.join(", ")
}

fn read_prompt(prompt: Option<String>) -> Result<String> {
    match prompt {
        Some(value) if !value.trim().is_empty() => Ok(value),
        Some(_) => Err(anyhow!("prompt is empty")),
        None => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .context("failed to read prompt from stdin")?;
            if buffer.trim().is_empty() {
                return Err(anyhow!("prompt is empty"));
            }
            Ok(buffer)
        }
    }
}

fn normalize_phone_number(value: &str) -> Option<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_owned())
    }
}

fn mask_phone(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 7 {
        return value.to_owned();
    }

    let prefix: String = chars.iter().take(3).collect();
    let suffix: String = chars
        .iter()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{prefix}****{suffix}")
}
