use async_trait::async_trait;
use std::sync::Arc;

use anyhow::Result;

use crate::prompt::PromptContext;

mod gemini;

pub use gemini::GeminiProvider;

#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn generate(
        &self,
        context: &PromptContext<'_>,
        options: AiRequestOptions<'_>,
    ) -> Result<String>;
}

#[derive(Clone)]
pub enum Provider {
    Gemini(Arc<GeminiProvider>),
    None,
}

impl Provider {
    pub async fn generate(
        &self,
        context: &PromptContext<'_>,
        options: AiRequestOptions<'_>,
    ) -> Result<String> {
        match self {
            Provider::Gemini(provider) => provider.generate(context, options).await,
            Provider::None => anyhow::bail!("No AI provider configured"),
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct AiRequestOptions<'a> {
    pub api_key: Option<&'a str>,
    pub model: Option<&'a str>,
}

