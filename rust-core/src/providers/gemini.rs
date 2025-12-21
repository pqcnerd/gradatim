//! Gemini AI provider implementation.

#![allow(dead_code)]

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::prompt::{PromptContext, SYSTEM_PROMPT};

use super::{AiProvider, AiRequestOptions};

#[derive(Clone)]
pub struct GeminiProvider {
    client: Client,
    default_api_key: Option<String>,
    default_model: String,
    max_tokens: i32,
    temperature: f32,
}

impl GeminiProvider {
    pub fn new(default_api_key: Option<String>, model: &str) -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            default_api_key,
            default_model: model.to_string(),
            max_tokens: 120,
            temperature: 0.2,
        })
    }
}

#[async_trait]
impl AiProvider for GeminiProvider {
    async fn generate(
        &self,
        context: &PromptContext<'_>,
        options: AiRequestOptions<'_>,
    ) -> Result<String> {
        let api_key = options
            .api_key
            .or(self.default_api_key.as_deref())
            .ok_or_else(|| anyhow!("Gemini API key missing"))?;
        let model = options
            .model
            .unwrap_or_else(|| self.default_model.as_str());
        let endpoint = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        );

        let payload = GeminiRequest {
            system_instruction: GeminiContent {
                role: "system".to_string(),
                parts: vec![GeminiPart {
                    text: SYSTEM_PROMPT.to_string(),
                }],
            },
            contents: vec![GeminiContent {
                role: "user".to_string(),
                parts: vec![GeminiPart {
                    text: context.to_user_prompt(),
                }],
            }],
            generation_config: GenerationConfig {
                max_output_tokens: self.max_tokens,
                temperature: self.temperature,
            },
        };

        let response = self
            .client
            .post(endpoint)
            .json(&payload)
            .send()
            .await
            .context("Gemini request failed")?
            .error_for_status()
            .context("Gemini returned an error status")?
            .json::<GeminiResponse>()
            .await
            .context("Gemini JSON payload could not be parsed")?;

        if let Some(err) = response.error {
            return Err(anyhow!(
                "Gemini API error: {}{}",
                err.message,
                err.status
                    .as_ref()
                    .map(|status| format!(" ({status})"))
                    .unwrap_or_default()
            ));
        }

        let text = response
            .candidates
            .unwrap_or_default()
            .into_iter()
            .flat_map(|candidate| candidate.content.parts)
            .find_map(|part| Some(part.text))
            .ok_or_else(|| anyhow!("Gemini returned no candidates"))?;

        info!("Gemini candidate length: {} chars", text.len());

        Ok(text.trim().to_string())
    }
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    #[serde(rename = "systemInstruction")]
    system_instruction: GeminiContent,
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    role: String,
    #[serde(default)]
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: i32,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    error: Option<GeminiErrorBody>,
}

#[derive(Debug, Deserialize)]
struct GeminiErrorBody {
    message: String,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    code: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

