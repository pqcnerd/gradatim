mod models;
mod prompt;
mod providers;
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
use models::{TranslateLineRequest, TranslateLineResponse};
use prompt::PromptContext;
use providers::{AiRequestOptions, GeminiProvider, Provider};
use translate::translate_line;
use tracing::{error, info, warn};

#[derive(Clone)]
struct AppState {
    language: String,
    provider: Provider,
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

    let provider = build_provider_from_env();

    let state = AppState {
        language: "c".to_string(),
        provider,
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

async fn handle_translate_line(
    State(state): State<AppState>,
    Json(payload): Json<TranslateLineRequest>,
) -> impl IntoResponse {
    if payload.language.to_lowercase() != state.language {
        return (
            StatusCode::BAD_REQUEST,
            Json(TranslateLineResponse::error("Unsupported language.")),
        );
    }

    let max_lines = payload
        .max_lines
        .unwrap_or(3)
        .clamp(1, prompt::HARD_MAX_LINES);

    if let Some(ai_code) = try_ai_translation(&state, &payload, max_lines).await {
        return (StatusCode::OK, Json(TranslateLineResponse::ok(ai_code)));
    }

    respond_with_rule_based(payload, max_lines)
}

fn respond_with_rule_based(
    payload: TranslateLineRequest,
    max_lines: usize,
) -> (StatusCode, Json<TranslateLineResponse>) {
    match translate_line(&payload) {
        Ok(code) => {
            if snippet_exceeds_limit(&code, max_lines) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(TranslateLineResponse::error(format!(
                        "Generated code exceeds the configured line limit ({}).",
                        max_lines
                    ))),
                );
            }
            (StatusCode::OK, Json(TranslateLineResponse::ok(code)))
        }
        Err(err) => {
            let message = err.to_string();
            if message.starts_with("UNHANDLED:") {
                let placeholder = todo_placeholder(&payload.language, &payload.english_line);
                return (
                    StatusCode::OK,
                    Json(TranslateLineResponse::unhandled(placeholder)),
                );
            }

            error!("translate_line failed: {err:?}");
            (
                StatusCode::BAD_REQUEST,
                Json(TranslateLineResponse::error(err.to_string())),
            )
        }
    }
}

fn build_provider_from_env() -> Provider {
    let api_key = std::env::var("GEMINI_API_KEY").ok().filter(|k| !k.is_empty());
    let model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string());

    match GeminiProvider::new(api_key.clone(), &model) {
        Ok(provider) => {
            if api_key.is_some() {
                info!("Gemini provider enabled using model {}", model);
            } else {
                info!("Gemini provider initialized without API key; expecting per-request credentials.");
            }
            Provider::Gemini(Arc::new(provider))
        }
        Err(err) => {
            warn!("Failed to initialize Gemini provider: {err:?}");
            Provider::None
        }
    }
}

async fn try_ai_translation(
    state: &AppState,
    payload: &TranslateLineRequest,
    max_lines: usize,
) -> Option<String> {
    match state.provider.clone() {
        Provider::None => None,
        provider => {
            let context = PromptContext::from_request(payload, 1200, 600);
            let ai_options = AiRequestOptions {
                api_key: payload.api_key.as_deref(),
                model: payload.model.as_deref(),
            };

            let result =
                provider
                    .generate(&context, ai_options)
                    .await
                    .and_then(|candidate| {
                        prompt::validate_candidate(&candidate, context.max_lines.min(max_lines))?;
                        Ok(candidate)
                    });

            match result {
                Ok(candidate) => Some(candidate),
                Err(err) => {
                    warn!("AI translation failed, falling back to rule-based: {err:?}");
                    None
                }
            }
        }
    }
}

fn snippet_exceeds_limit(snippet: &str, max_lines: usize) -> bool {
    let capped = max_lines.max(1);
    let mut lines: Vec<&str> = snippet.split('\n').collect();
    if lines.is_empty() {
        return false;
    }
    if lines.len() == 1 && lines[0].is_empty() {
        return false;
    }
    if let Some(last) = lines.last() {
        if last.is_empty() {
            lines.pop();
        }
    }
    let effective_lines = lines.len().max(1);
    effective_lines > capped
}

fn todo_placeholder(language: &str, instruction: &str) -> String {
    if language.eq_ignore_ascii_case("python") {
        format!("# TODO: {}", instruction.trim())
    } else {
        format!("// TODO: {}", instruction.trim())
    }
}

