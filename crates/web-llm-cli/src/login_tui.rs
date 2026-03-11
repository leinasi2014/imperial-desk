use std::time::Duration;

use anyhow::{anyhow, Result};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    layout::{Alignment, Constraint, Flex, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph},
    Frame,
};

const ACCENT: Color = Color::Rgb(255, 92, 0);
const BG_PRIMARY: Color = Color::Rgb(15, 14, 14);
const BG_CARD: Color = Color::Rgb(31, 29, 28);
const TEXT_PRIMARY: Color = Color::Rgb(240, 239, 238);
const TEXT_SECONDARY: Color = Color::Rgb(168, 162, 158);
const TEXT_TERTIARY: Color = Color::Rgb(120, 113, 108);
const BORDER: Color = Color::Rgb(63, 59, 56);

pub fn prompt_verification_code(phone: &str) -> Result<String> {
    let mut terminal = ratatui::init();
    let result = run_prompt(&mut terminal, phone);
    ratatui::restore();
    result
}

fn run_prompt(terminal: &mut ratatui::DefaultTerminal, phone: &str) -> Result<String> {
    let mut code = String::new();

    loop {
        terminal.draw(|frame| render(frame, phone, &code))?;

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Enter => {
                    let trimmed = code.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    return Ok(trimmed.to_owned());
                }
                KeyCode::Esc => return Err(anyhow!("verification canceled")),
                KeyCode::Backspace => {
                    code.pop();
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Err(anyhow!("verification canceled"));
                }
                KeyCode::Char(ch) => {
                    if !ch.is_control() {
                        code.push(ch);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn render(frame: &mut Frame, phone: &str, code: &str) {
    let area = centered_rect(frame.area());
    frame.render_widget(Clear, area);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(BG_CARD).fg(BORDER))
        .title(
            Line::from(vec![Span::styled(
                " DeepSeek Login ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )])
            .alignment(Alignment::Center),
        );
    frame.render_widget(block, area);

    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 2,
    });
    let [title_area, info_area, input_area, hint_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(2),
    ])
    .areas(inner);

    let title =
        Paragraph::new(vec![
        Line::from(Span::styled(
            "Verification Code",
            Style::default().fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "Browser has sent or is about to send an SMS code. Enter it here to complete login.",
            Style::default().fg(TEXT_SECONDARY),
        )),
    ])
        .alignment(Alignment::Center)
        .style(Style::default().bg(BG_CARD));
    frame.render_widget(title, title_area);

    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Phone: ", Style::default().fg(TEXT_TERTIARY)),
            Span::styled(
                mask_phone(phone),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            "DeepSeek uses phone SMS login. The code is not stored on disk.",
            Style::default().fg(TEXT_SECONDARY),
        )),
    ])
    .alignment(Alignment::Left)
    .style(Style::default().bg(BG_CARD));
    frame.render_widget(info, info_area);

    let input = Paragraph::new(code.to_owned())
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .style(Style::default().bg(BG_PRIMARY).fg(BORDER))
                .title(Span::styled(
                    " SMS Code ",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                )),
        )
        .style(Style::default().fg(TEXT_PRIMARY).bg(BG_PRIMARY));
    frame.render_widget(input, input_area);

    let hint = Paragraph::new(vec![Line::from(vec![
        Span::styled(
            "Enter",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" submit  ", Style::default().fg(TEXT_SECONDARY)),
        Span::styled(
            "Esc",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" cancel", Style::default().fg(TEXT_SECONDARY)),
    ])])
    .alignment(Alignment::Center)
    .style(Style::default().bg(BG_CARD));
    frame.render_widget(hint, hint_area);
}

fn centered_rect(area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let [vertical] = Layout::vertical([Constraint::Length(12)])
        .flex(Flex::Center)
        .areas(area);
    let [horizontal] = Layout::horizontal([Constraint::Length(72)])
        .flex(Flex::Center)
        .areas(vertical);
    horizontal
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
