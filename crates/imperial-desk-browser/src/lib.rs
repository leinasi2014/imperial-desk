//! Browser abstraction for provider-specific web automation.

use std::{env, path::PathBuf};

use async_trait::async_trait;
use chromiumoxide::{
    browser::{Browser, BrowserConfig},
    Page,
};
use futures::StreamExt;
use imperial_desk_core::{ProviderOptions, Result, WebLlmError};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct BrowserLaunchOptions {
    pub base_url: String,
    pub headed: bool,
    pub profile_dir: PathBuf,
}

impl BrowserLaunchOptions {
    #[must_use]
    pub fn from_provider_options(
        options: &ProviderOptions,
        default_base_url: &str,
        profile_dir: PathBuf,
    ) -> Self {
        Self {
            base_url: options
                .base_url
                .clone()
                .unwrap_or_else(|| default_base_url.to_owned()),
            headed: options.headed,
            profile_dir,
        }
    }
}

#[async_trait]
pub trait BrowserBackend: Send {
    async fn goto(&mut self, url: &str) -> Result<()>;
    async fn has_first_visible(&mut self, selectors: &[&str]) -> Result<bool>;
    async fn click_first_visible(&mut self, selectors: &[&str]) -> Result<bool>;
    async fn fill_first_visible(&mut self, selectors: &[&str], text: &str) -> Result<()>;
    async fn press_key_on_first_visible(&mut self, selectors: &[&str], key: &str) -> Result<bool>;
    async fn body_text(&mut self) -> Result<String>;
    async fn evaluate_json(&mut self, script: &str) -> Result<Value>;
    async fn current_url(&mut self) -> Result<String>;
}

pub async fn create_default_browser_backend(
    options: &BrowserLaunchOptions,
) -> Result<Box<dyn BrowserBackend>> {
    Ok(Box::new(ChromiumBrowserBackend::launch(options).await?))
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct VisibleTarget {
    index: usize,
    selector: String,
}

pub struct ChromiumBrowserBackend {
    _browser: Browser,
    handler_task: JoinHandle<()>,
    page: Page,
}

impl ChromiumBrowserBackend {
    pub async fn launch(options: &BrowserLaunchOptions) -> Result<Self> {
        let mut builder = BrowserConfig::builder()
            .user_data_dir(&options.profile_dir)
            .window_size(1440, 1080)
            .viewport(None);

        if options.headed {
            builder = builder.with_head();
        } else {
            builder = builder.new_headless_mode();
        }

        if should_disable_sandbox() {
            builder = builder.no_sandbox();
        }

        let config = builder.build().map_err(WebLlmError::browser)?;
        let (browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|error| WebLlmError::browser(error.to_string()))?;
        let handler_task =
            tokio::spawn(async move { while let Some(_result) = handler.next().await {} });
        let page = browser
            .new_page("about:blank")
            .await
            .map_err(|error| WebLlmError::browser(error.to_string()))?;

        Ok(Self {
            _browser: browser,
            handler_task,
            page,
        })
    }

    async fn evaluate_value<T>(&self, script: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let evaluation = self
            .page
            .evaluate(script)
            .await
            .map_err(|error| WebLlmError::browser(error.to_string()))?;
        let value = evaluation.value().cloned().unwrap_or(Value::Null);
        serde_json::from_value(value).map_err(WebLlmError::from)
    }

    async fn locate_visible_target(&self, selectors: &[&str]) -> Result<Option<VisibleTarget>> {
        let selectors_json = serde_json::to_string(selectors)?;
        let script = format!(
            r#"() => {{
                const selectors = {selectors_json};
                const isVisible = (element) => Boolean(
                    element &&
                    (element.offsetParent !== null || getComputedStyle(element).position === "fixed")
                );

                for (const selector of selectors) {{
                    const elements = Array.from(document.querySelectorAll(selector));
                    for (let index = 0; index < elements.length; index += 1) {{
                        if (isVisible(elements[index])) {{
                            return {{ selector, index }};
                        }}
                    }}
                }}

                return null;
            }}"#,
        );

        self.evaluate_value(&script).await
    }
}

#[async_trait]
impl BrowserBackend for ChromiumBrowserBackend {
    async fn goto(&mut self, url: &str) -> Result<()> {
        self.page
            .goto(url)
            .await
            .map_err(|error| WebLlmError::browser(error.to_string()))?;
        Ok(())
    }

    async fn has_first_visible(&mut self, selectors: &[&str]) -> Result<bool> {
        Ok(self.locate_visible_target(selectors).await?.is_some())
    }

    async fn click_first_visible(&mut self, selectors: &[&str]) -> Result<bool> {
        let Some(target) = self.locate_visible_target(selectors).await? else {
            return Ok(false);
        };
        let mut elements = self
            .page
            .find_elements(target.selector)
            .await
            .map_err(|error| WebLlmError::browser(error.to_string()))?;
        let Some(element) = elements.get_mut(target.index) else {
            return Ok(false);
        };
        element
            .click()
            .await
            .map_err(|error| WebLlmError::browser(error.to_string()))?;
        Ok(true)
    }

    async fn fill_first_visible(&mut self, selectors: &[&str], text: &str) -> Result<()> {
        let selectors_json = serde_json::to_string(selectors)?;
        let text_json = serde_json::to_string(text)?;
        let script = format!(
            r#"() => {{
                const selectors = {selectors_json};
                const text = {text_json};
                const isVisible = (element) => Boolean(
                    element &&
                    (element.offsetParent !== null || getComputedStyle(element).position === "fixed")
                );
                const setValue = (element, value) => {{
                    if (element instanceof HTMLTextAreaElement || element instanceof HTMLInputElement) {{
                        const prototype = element instanceof HTMLTextAreaElement
                            ? HTMLTextAreaElement.prototype
                            : HTMLInputElement.prototype;
                        const descriptor = Object.getOwnPropertyDescriptor(prototype, "value");
                        if (descriptor && descriptor.set) {{
                            descriptor.set.call(element, value);
                        }} else {{
                            element.value = value;
                        }}
                        element.dispatchEvent(new Event("input", {{ bubbles: true }}));
                        element.dispatchEvent(new Event("change", {{ bubbles: true }}));
                        return true;
                    }}
                    if (element instanceof HTMLElement && element.isContentEditable) {{
                        element.textContent = value;
                        element.dispatchEvent(new Event("input", {{ bubbles: true }}));
                        return true;
                    }}
                    return false;
                }};

                for (const selector of selectors) {{
                    const elements = Array.from(document.querySelectorAll(selector));
                    for (const element of elements) {{
                        if (!isVisible(element)) {{
                            continue;
                        }}
                        element.focus();
                        if (typeof element.select === "function") {{
                            element.select();
                        }}
                        return setValue(element, text);
                    }}
                }}

                return false;
            }}"#,
        );

        let filled: bool = self.evaluate_value(&script).await?;
        if !filled {
            return Err(WebLlmError::browser("chat input box was not found"));
        }

        Ok(())
    }

    async fn press_key_on_first_visible(&mut self, selectors: &[&str], key: &str) -> Result<bool> {
        let Some(target) = self.locate_visible_target(selectors).await? else {
            return Ok(false);
        };
        let mut elements = self
            .page
            .find_elements(target.selector)
            .await
            .map_err(|error| WebLlmError::browser(error.to_string()))?;
        let Some(element) = elements.get_mut(target.index) else {
            return Ok(false);
        };
        element
            .click()
            .await
            .map_err(|error| WebLlmError::browser(error.to_string()))?;
        element
            .press_key(key)
            .await
            .map_err(|error| WebLlmError::browser(error.to_string()))?;
        Ok(true)
    }

    async fn body_text(&mut self) -> Result<String> {
        self.evaluate_value::<String>("() => document.body?.innerText ?? \"\"")
            .await
    }

    async fn evaluate_json(&mut self, script: &str) -> Result<Value> {
        self.evaluate_value(script).await
    }

    async fn current_url(&mut self) -> Result<String> {
        match self
            .page
            .url()
            .await
            .map_err(|error| WebLlmError::browser(error.to_string()))?
        {
            Some(url) => Ok(url),
            None => self.evaluate_value("() => location.href").await,
        }
    }
}

impl Drop for ChromiumBrowserBackend {
    fn drop(&mut self) {
        self.handler_task.abort();
    }
}

#[derive(Debug, Default)]
pub struct NoopBrowserBackend;

#[async_trait]
impl BrowserBackend for NoopBrowserBackend {
    async fn goto(&mut self, _url: &str) -> Result<()> {
        Err(WebLlmError::not_implemented("chromium browser backend"))
    }

    async fn has_first_visible(&mut self, _selectors: &[&str]) -> Result<bool> {
        Err(WebLlmError::not_implemented("chromium browser backend"))
    }

    async fn click_first_visible(&mut self, _selectors: &[&str]) -> Result<bool> {
        Err(WebLlmError::not_implemented("chromium browser backend"))
    }

    async fn fill_first_visible(&mut self, _selectors: &[&str], _text: &str) -> Result<()> {
        Err(WebLlmError::not_implemented("chromium browser backend"))
    }

    async fn press_key_on_first_visible(
        &mut self,
        _selectors: &[&str],
        _key: &str,
    ) -> Result<bool> {
        Err(WebLlmError::not_implemented("chromium browser backend"))
    }

    async fn body_text(&mut self) -> Result<String> {
        Err(WebLlmError::not_implemented("chromium browser backend"))
    }

    async fn evaluate_json(&mut self, _script: &str) -> Result<Value> {
        Err(WebLlmError::not_implemented("chromium browser backend"))
    }

    async fn current_url(&mut self) -> Result<String> {
        Err(WebLlmError::not_implemented("chromium browser backend"))
    }
}

fn should_disable_sandbox() -> bool {
    cfg!(target_os = "linux") || env::var_os("WSL_DISTRO_NAME").is_some()
}
