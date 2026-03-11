pub const INPUT_SELECTORS: &[&str] = &[
    r#"textarea[placeholder*="Message"]"#,
    r#"textarea[placeholder*="message"]"#,
    "textarea",
    r#"[contenteditable="true"][role="textbox"]"#,
    r#"[contenteditable="true"]"#,
];

pub const SEND_BUTTON_SELECTORS: &[&str] = &[
    r#"button[type="submit"]"#,
    r#"button[aria-label*="Send"]"#,
    r#"button[aria-label*="send"]"#,
];

pub const INSPECT_NEW_CHAT_SELECTORS: &[&str] = &["button", "a", r#"div[role="button"]"#];
pub const THINKING_LABELS: &[&str] = &["深度思考", "DeepThink", "Deep Thinking"];
pub const SEARCH_LABELS: &[&str] = &["智能搜索", "Search", "Web Search"];
