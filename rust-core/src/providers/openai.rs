//! OpenAI provider implementation.

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::prompt::{PromptContext, SYSTEM_PROMPT};

use super::{AiProvider, AiRequestOptions};

#[derive(Clone)]
pub struct OpenAIProvider {
    client: Client,
    default_api_key: Option<String>,
    default_model: String,
    max_tokens: i32,
    temperature: f32,
}

impl OpenAIProvider {
    pub fn new(default_api_key: Option<String>, model: &str) -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            default_api_key,
            default_model: model.to_string(),
            max_tokens: 240,
            temperature: 0.2,
        })
    }
}

#[async_trait]
impl AiProvider for OpenAIProvider {
    async fn generate(
        &self,
        context: &PromptContext<'_>,
        options: AiRequestOptions<'_>,
    ) -> Result<String> {
        let api_key = options
            .api_key
            .or(self.default_api_key.as_deref())
            .ok_or_else(|| anyhow!("OpenAI API key missing"))?;
        let model = options
            .model
            .unwrap_or_else(|| self.default_model.as_str());

        let payload = OpenAIRequest {
            model: model.to_string(),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            messages: vec![
                OpenAIMessage {
                    role: "system".to_string(),
                    content: SYSTEM_PROMPT.to_string(),
                },
                OpenAIMessage {
                    role: "user".to_string(),
                    content: context.to_user_prompt(),
                },
            ],
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(api_key)
            .json(&payload)
            .send()
            .await
            .context("OpenAI request failed")?
            .error_for_status()
            .context("OpenAI returned an error status")?
            .json::<OpenAIResponse>()
            .await
            .context("OpenAI JSON payload could not be parsed")?;

        let text = response
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content.trim().to_string())
            .filter(|content| !content.is_empty())
            .ok_or_else(|| anyhow!("OpenAI returned no completion text"))?;

        info!("OpenAI candidate length: {} chars", text.len());
        Ok(text)
    }
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f32,
    max_tokens: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}
