use imperial_desk_browser::BrowserBackend;
use imperial_desk_core::{Result, WebLlmError};
use serde_json::Value;

use crate::providers::deepseek::common::models::AuthorizedMutationResult;

pub async fn run_authorized_mutation(
    browser: &mut dyn BrowserBackend,
    base_url: &str,
    endpoint_path: &str,
    payload: Value,
    fallback_error: &str,
) -> Result<AuthorizedMutationResult> {
    browser.goto(base_url).await?;
    let result = post_authorized_json(browser, endpoint_path, &payload).await?;
    if result.ok {
        return Ok(result);
    }

    Err(WebLlmError::browser(
        result.error.unwrap_or_else(|| fallback_error.to_owned()),
    ))
}

async fn post_authorized_json(
    browser: &mut dyn BrowserBackend,
    endpoint_path: &str,
    payload: &Value,
) -> Result<AuthorizedMutationResult> {
    let endpoint_json = serde_json::to_string(endpoint_path)?;
    let payload_json = serde_json::to_string(payload)?;
    let script = format!(
        r#"async () => {{
            const endpointPathValue = {endpoint_json};
            const payloadValue = {payload_json};
            const rawToken = localStorage.getItem("userToken");
            const tokenValue = rawToken ? JSON.parse(rawToken)?.value ?? null : null;
            if (!tokenValue) {{
                return {{
                    ok: false,
                    status: 401,
                    data: null,
                    error: "missing DeepSeek userToken in localStorage"
                }};
            }}

            const response = await fetch(endpointPathValue, {{
                method: "POST",
                headers: {{
                    authorization: `Bearer ${{tokenValue}}`,
                    "content-type": "application/json"
                }},
                body: JSON.stringify(payloadValue),
                credentials: "include"
            }});

            let data = null;
            try {{
                data = await response.json();
            }} catch (_error) {{
                data = null;
            }}

            const success = Boolean(
                response.ok &&
                data?.code === 0 &&
                data?.data?.biz_code === 0
            );
            return {{
                ok: success,
                status: response.status,
                data,
                error: success
                    ? null
                    : data?.msg ?? `unexpected API response (HTTP ${{response.status}})`
            }};
        }}"#,
    );

    let result = browser.evaluate_json(&script).await?;
    serde_json::from_value(result).map_err(WebLlmError::from)
}
