use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct TranslateLineRequest {
    #[serde(default)]
    pub english_line: String,
    #[serde(default)]
    pub code_before: String,
    #[serde(default)]
    pub code_after: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub line_index: usize,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub max_lines: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct TranslateLineResponse {
    pub kind: ResponseKind,
    pub code: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseKind {
    Ok,
    Error,
    Unhandled,
}

impl TranslateLineResponse {
    pub fn ok<S: Into<String>>(code: S) -> Self {
        Self {
            kind: ResponseKind::Ok,
            code: Some(code.into()),
            message: None,
        }
    }

    pub fn error<S: Into<String>>(message: S) -> Self {
        Self {
            kind: ResponseKind::Error,
            code: None,
            message: Some(message.into()),
        }
    }

    pub fn unhandled<S: Into<String>>(placeholder: S) -> Self {
        Self {
            kind: ResponseKind::Unhandled,
            code: Some(placeholder.into()),
            message: Some("Rule-based translator could not interpret this instruction.".into()),
        }
    }
}

