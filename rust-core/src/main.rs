//! Gradatim Translation Microservice
//!
//! This service translates natural language instructions into code using a
//! preprocessing pipeline:
//!
//! 1. **Intent Classification**: Reject project-level requests
//! 2. **Hint Extraction**: Parse structured hints from the instruction
//! 3. **Translation**: Use rule-based or AI translation based on hint complexity
//! 4. **Validation**: Ensure output is reasonable code

mod hints;
mod intent;
mod models;
mod prompt;
mod providers;
mod stdlib_db;
mod func_matcher;
mod manpage_parser;
mod synonyms;
mod translate;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use hints::{StatementHint, VariableContext, validate_reads};
use models::{TranslateLineRequest, TranslateLineResponse};
use prompt::PromptContext;
use providers::{AiRequestOptions, GeminiProvider, OpenAIProvider, Provider};
use translate::{translate_from_hint, translate_with_context};
use tracing::{debug, error, info, warn};

/// Application state shared across request handlers.
#[derive(Clone)]
struct AppState {
    gemini_provider: Provider,
    openai_provider: Provider,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,rust_core=debug".into()),
        )
        .compact()
        .init();

    let (gemini_provider, openai_provider) = build_providers_from_env();

    let state = AppState {
        gemini_provider,
        openai_provider,
    };

    let app = Router::new()
        .route("/translate-line", post(handle_translate_line))
        .with_state(state);

    let addr: SocketAddr = std::env::var("RUST_CORE_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:4888".to_string())
        .parse()
        .expect("Unable to parse RUST_CORE_ADDR");

    info!("Starting rust-core server on http://{}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}

/// Main translation endpoint handler.
///
/// Pipeline:
/// 1. Classify intent → reject project-level requests
/// 2. Extract hints → structured understanding of the instruction
/// 3. Analyze context → check for undeclared variables (warnings only)
/// 4. For trivial hints → generate code directly from hints
/// 5. For complex hints → send to AI with hints as context
/// 6. Validate and return with any warnings
async fn handle_translate_line(
    State(state): State<AppState>,
    Json(payload): Json<TranslateLineRequest>,
) -> impl IntoResponse {
    let language = if payload.language.is_empty() {
        "c".to_string()
    } else {
        payload.language.to_lowercase()
    };

    // Step 1: Intent Classification
    let intent_result = intent::classify(&payload.english_line);
    if let Some(rejection) = intent_result.rejection_message() {
        info!(
            "Rejected project-level request: {}",
            payload.english_line.chars().take(50).collect::<String>()
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(TranslateLineResponse::error(rejection)),
        );
    }

    // Step 2: Extract Hints
    let hints = hints::extract(&payload);
    debug!("Extracted hints: {:?}", hints);

    // Step 3: Build Variable Context
    let context = VariableContext::from_code(&payload.code_before);
    debug!("Variable context: {:?}", context.get_declared());

    // Step 4: Validate reads - error if undeclared variables are read
    let validation = validate_reads(&hints, &context);
    if !validation.is_valid {
        // Return error for undeclared reads
        let error_msg = validation.error_message().unwrap_or_else(|| 
            "unknown error".to_string()
        );
        info!("Validation failed: {}", error_msg);
        return (
            StatusCode::BAD_REQUEST,
            Json(TranslateLineResponse::error(format!("ERROR: {}", error_msg))),
        );
    }

    // Step 5: For trivial hints, generate code with context-aware declarations
    if hints.is_trivial() {
        debug!("Using context-aware rule-based translation for trivial hint");
        return respond_with_context(&hints, &validation, &context, &language);
    }

    // Step 6: Try AI translation with hints
    if let Some(ai_code) = try_ai_translation_with_hints(&state, &payload, &hints, &language).await
    {
        return (
            StatusCode::OK,
            Json(TranslateLineResponse::ok(ai_code)),
        );
    }

    // Step 7: Fallback to context-aware rule-based if AI fails
    respond_with_context(&hints, &validation, &context, &language)
}

/// Generate code from a hint using the context-aware translator.
fn respond_with_context(
    hint: &StatementHint,
    validation: &hints::ValidationResult,
    context: &VariableContext,
    language: &str,
) -> (StatusCode, Json<TranslateLineResponse>) {
    match translate_with_context(hint, validation, context, language) {
        Ok(code) => (
            StatusCode::OK,
            Json(TranslateLineResponse::ok(code)),
        ),
        Err(err) => {
            let message = err.to_string();
            if message.starts_with("UNHANDLED:") {
                // Extract the original instruction from the hint
                let original = match hint {
                    StatementHint::Unknown { original } => original.clone(),
                    _ => message.replace("UNHANDLED: ", ""),
                };
                let placeholder = todo_placeholder(language, &original);
                (
                    StatusCode::OK,
                    Json(TranslateLineResponse::unhandled(placeholder)),
                )
            } else {
                error!("translate_with_context failed: {err:?}");
                (
                    StatusCode::BAD_REQUEST,
                    Json(TranslateLineResponse::error(err.to_string())),
                )
            }
        }
    }
}

/// Generate code from a hint using the rule-based translator (legacy, without context).
#[allow(dead_code)]
fn respond_with_hint_based(
    hint: &StatementHint,
    language: &str,
    warnings: Vec<String>,
) -> (StatusCode, Json<TranslateLineResponse>) {
    match translate_from_hint(hint, language) {
        Ok(code) => (
            StatusCode::OK,
            Json(TranslateLineResponse::ok_with_warnings(code, warnings)),
        ),
        Err(err) => {
            let message = err.to_string();
            if message.starts_with("UNHANDLED:") {
                // Extract the original instruction from the hint
                let original = match hint {
                    StatementHint::Unknown { original } => original.clone(),
                    _ => message.replace("UNHANDLED: ", ""),
                };
                let placeholder = todo_placeholder(language, &original);
                (
                    StatusCode::OK,
                    Json(TranslateLineResponse::unhandled(placeholder)),
                )
            } else {
                error!("translate_from_hint failed: {err:?}");
                (
                    StatusCode::BAD_REQUEST,
                    Json(TranslateLineResponse::error(err.to_string())),
                )
            }
        }
    }
}

/// Build all supported AI providers from environment variables.
fn build_providers_from_env() -> (Provider, Provider) {
    let gemini_api_key = std::env::var("GEMINI_API_KEY").ok().filter(|k| !k.is_empty());
    let gemini_model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string());
    let openai_api_key = std::env::var("OPENAI_API_KEY").ok().filter(|k| !k.is_empty());
    let openai_model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4.1-nano".to_string());

    let gemini_provider = match GeminiProvider::new(gemini_api_key.clone(), &gemini_model) {
        Ok(provider) => {
            if gemini_api_key.is_some() {
                info!("Gemini provider enabled using model {}", gemini_model);
            } else {
                info!("Gemini provider initialized without API key; expecting per-request credentials.");
            }
            Provider::Gemini(Arc::new(provider))
        }
        Err(err) => {
            warn!("Failed to initialize Gemini provider: {err:?}");
            Provider::None
        }
    };

    let openai_provider = match OpenAIProvider::new(openai_api_key.clone(), &openai_model) {
        Ok(provider) => {
            if openai_api_key.is_some() {
                info!("OpenAI provider enabled using model {}", openai_model);
            } else {
                info!("OpenAI provider initialized without API key; expecting per-request credentials.");
            }
            Provider::OpenAI(Arc::new(provider))
        }
        Err(err) => {
            warn!("Failed to initialize OpenAI provider: {err:?}");
            Provider::None
        }
    };

    (gemini_provider, openai_provider)
}

/// Attempt AI translation with extracted hints.
///
/// The hints are included in the prompt to give the AI structured context
/// about what the user wants.
async fn try_ai_translation_with_hints(
    state: &AppState,
    payload: &TranslateLineRequest,
    hints: &StatementHint,
    language: &str,
) -> Option<String> {
    let selected_provider = payload
        .provider
        .as_deref()
        .map(|provider| provider.to_ascii_lowercase())
        .or_else(|| {
            payload.model.as_deref().map(|model| {
                if model.to_ascii_lowercase().starts_with("gpt-") {
                    "openai".to_string()
                } else {
                    "gemini".to_string()
                }
            })
        })
        .unwrap_or_else(|| "gemini".to_string());

    let provider = match selected_provider.as_str() {
        "openai" | "deepseek" | "other" => state.openai_provider.clone(),
        _ => state.gemini_provider.clone(),
    };

    match provider {
        Provider::None => {
            debug!("No AI provider configured for `{selected_provider}`, skipping AI translation");
            None
        }
        active_provider => {
            // Build prompt context with hints
            let context = PromptContext::with_hints(payload, hints.clone(), 1200, 600);
            let ai_options = AiRequestOptions {
                api_key: payload.api_key.as_deref(),
                model: payload.model.as_deref(),
            };

            let result = active_provider
                .generate(&context, ai_options)
                .await
                .and_then(|candidate| {
                    prompt::validate_candidate(&candidate, language)?;
                    Ok(candidate)
                });

            match result {
                Ok(candidate) => {
                    info!(
                        "AI translation succeeded: {} chars",
                        candidate.len()
                    );
                    Some(candidate)
                }
                Err(err) => {
                    warn!("AI translation failed, falling back to rule-based: {err:?}");
                    None
                }
            }
        }
    }
}

/// Generate a TODO placeholder comment for unhandled instructions.
fn todo_placeholder(language: &str, instruction: &str) -> String {
    if language.eq_ignore_ascii_case("python") {
        format!("# TODO: {}", instruction.trim())
    } else {
        format!("// TODO: {}", instruction.trim())
    }
}
