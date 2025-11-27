#![allow(dead_code)]

use anyhow::{anyhow, Result};
use serde::Serialize;

use crate::models::TranslateLineRequest;

pub const SYSTEM_PROMPT: &str = r#"You are a compiler that converts a single
line of natural language into the requested target language (C, Python, etc.).
Output only the minimal code for that instruction—no explanations, comments, or
Markdown fences. Respect indentation and surrounding code, never change other
lines, and keep the response to at most three lines unless absolutely necessary."#;
pub const HARD_MAX_LINES: usize = 5;

#[derive(Debug, Serialize)]
pub struct PromptMessage<'a> {
    pub role: &'a str,
    pub content: String,
}

#[derive(Debug)]
pub struct PromptContext<'a> {
    pub english_line: &'a str,
    pub code_before: String,
    pub code_after: String,
    pub line_index: usize,
    pub language: String,
    pub max_lines: usize,
}

impl<'a> PromptContext<'a> {
    pub fn from_request(
        request: &'a TranslateLineRequest,
        before_budget: usize,
        after_budget: usize,
    ) -> Self {
        let requested_max = request.max_lines.unwrap_or(3);
        let clamped_max = requested_max.max(1).min(HARD_MAX_LINES);
        Self {
            english_line: request.english_line.as_str(),
            code_before: tail(&request.code_before, before_budget),
            code_after: head(&request.code_after, after_budget),
            line_index: request.line_index,
            language: if request.language.is_empty() {
                "c".to_string()
            } else {
                request.language.to_lowercase()
            },
            max_lines: clamped_max,
        }
    }

    pub fn to_messages(&self) -> Vec<PromptMessage<'a>> {
        vec![
            PromptMessage {
                role: "system",
                content: SYSTEM_PROMPT.to_string(),
            },
            PromptMessage {
                role: "user",
                content: self.to_user_prompt(),
            },
        ]
    }

    pub fn to_user_prompt(&self) -> String {
        let language = self.language.to_uppercase();
        format!(
            "TARGET LANGUAGE: {}\nCURSOR LINE NUMBER: {}\nMAX LINES PER RESPONSE: {}\nCURRENT LINE (ENGLISH):\n{}\n\nPRECEDING CODE:\n{}\n\nFOLLOWING CODE:\n{}",
            language, self.line_index, self.max_lines, self.english_line, self.code_before, self.code_after
        )
    }
}

pub fn validate_candidate(candidate: &str, max_lines: usize) -> Result<()> {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("empty response"));
    }

    if trimmed.contains("```") {
        return Err(anyhow!("response contains markdown fences"));
    }

    if trimmed.len() > 240 {
        return Err(anyhow!("response too long"));
    }

    if trimmed.lines().count() > max_lines.min(HARD_MAX_LINES) {
        return Err(anyhow!("response spans too many lines"));
    }

    if trimmed.matches('{').count() != trimmed.matches('}').count() {
        return Err(anyhow!("mismatched braces"));
    }

    let lower = trimmed.to_lowercase();
    if lower.contains("console.log") || lower.contains("def ") {
        return Err(anyhow!("response is not C code"));
    }

    if trimmed.contains("://") {
        return Err(anyhow!("response contains URLs"));
    }

    Ok(())
}

fn tail(input: &str, budget: usize) -> String {
    if budget == 0 || input.is_empty() {
        return String::new();
    }

    let chars: Vec<char> = input.chars().collect();
    if chars.len() <= budget {
        input.to_string()
    } else {
        chars[chars.len().saturating_sub(budget)..].iter().collect()
    }
}

fn head(input: &str, budget: usize) -> String {
    if budget == 0 || input.is_empty() {
        return String::new();
    }

    let chars: Vec<char> = input.chars().collect();
    if chars.len() <= budget {
        input.to_string()
    } else {
        chars[..budget].iter().collect()
    }
}

