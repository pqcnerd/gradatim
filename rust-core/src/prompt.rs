//! AI prompt construction and response validation.
//!
//! This module builds prompts for the AI that include:
//! - The original English instruction
//! - Extracted structured hints (intent, variables, etc.)
//! - Surrounding code context
//!
//! It also validates AI responses to ensure they're reasonable code.

#![allow(dead_code)]

use anyhow::{anyhow, Result};
use serde::Serialize;

use crate::hints::StatementHint;
use crate::models::TranslateLineRequest;

/// System prompt that instructs the AI on its role.
/// Now emphasizes using the extracted hints to generate accurate code.
pub const SYSTEM_PROMPT: &str = r#"You are a precise code translator that converts natural language instructions into code.

RULES:
1. Output ONLY the code for the given instruction - no explanations, comments, or markdown.
2. Use the EXTRACTED HINTS to understand the user's intent precisely.
3. Generate a single logical statement or construct (one declaration, one loop, one condition, etc.).
4. Match the style and indentation of the surrounding code.
5. Never generate entire programs, multiple functions, or project-level code.
6. If the instruction is ambiguous, prefer the simplest interpretation.
7. Respect the target language syntax exactly."#;

/// Soft limit for response length - used for guidance, not strict rejection.
pub const SOFT_MAX_LINES: usize = 8;

/// Character limit for responses to prevent runaway generation.
pub const MAX_RESPONSE_CHARS: usize = 500;

#[derive(Debug, Serialize)]
pub struct PromptMessage<'a> {
    pub role: &'a str,
    pub content: String,
}

/// Context for building an AI prompt, now including extracted hints.
#[derive(Debug)]
pub struct PromptContext<'a> {
    pub english_line: &'a str,
    pub hints: Option<StatementHint>,
    pub code_before: String,
    pub code_after: String,
    pub line_index: usize,
    pub language: String,
}

impl<'a> PromptContext<'a> {
    /// Create a prompt context from a request (without hints - for backward compatibility).
    pub fn from_request(
        request: &'a TranslateLineRequest,
        before_budget: usize,
        after_budget: usize,
    ) -> Self {
        Self {
            english_line: request.english_line.as_str(),
            hints: None,
            code_before: tail(&request.code_before, before_budget),
            code_after: head(&request.code_after, after_budget),
            line_index: request.line_index,
            language: if request.language.is_empty() {
                "c".to_string()
            } else {
                request.language.to_lowercase()
            },
        }
    }

    /// Create a prompt context with extracted hints.
    pub fn with_hints(
        request: &'a TranslateLineRequest,
        hints: StatementHint,
        before_budget: usize,
        after_budget: usize,
    ) -> Self {
        Self {
            english_line: request.english_line.as_str(),
            hints: Some(hints),
            code_before: tail(&request.code_before, before_budget),
            code_after: head(&request.code_after, after_budget),
            line_index: request.line_index,
            language: if request.language.is_empty() {
                "c".to_string()
            } else {
                request.language.to_lowercase()
            },
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

    /// Generate the user prompt, including hints if available.
    pub fn to_user_prompt(&self) -> String {
        let language = self.language.to_uppercase();
        
        let hints_section = if let Some(ref hints) = self.hints {
            format!("\n\nEXTRACTED HINTS:\n{}", hints.to_prompt_hints())
        } else {
            String::new()
        };

        format!(
            "TARGET LANGUAGE: {lang}\nLINE NUMBER: {line}{hints}\n\nINSTRUCTION:\n{english}\n\nPRECEDING CODE:\n{before}\n\nFOLLOWING CODE:\n{after}",
            lang = language,
            line = self.line_index,
            hints = hints_section,
            english = self.english_line,
            before = self.code_before,
            after = self.code_after
        )
    }
}

/// Validate an AI-generated code candidate.
/// 
/// Instead of arbitrary line limits, this uses semantic validation:
/// - No markdown fences
/// - No URLs
/// - No multiple function definitions
/// - Reasonable length
/// - Balanced braces
pub fn validate_candidate(candidate: &str, language: &str) -> Result<()> {
    let trimmed = candidate.trim();
    
    if trimmed.is_empty() {
        return Err(anyhow!("empty response"));
    }

    // No markdown fences allowed
    if trimmed.contains("```") {
        return Err(anyhow!("response contains markdown fences"));
    }

    // Reasonable length limit
    if trimmed.len() > MAX_RESPONSE_CHARS {
        return Err(anyhow!("response exceeds {} characters", MAX_RESPONSE_CHARS));
    }

    // Check for URLs (likely hallucination)
    if trimmed.contains("://") {
        return Err(anyhow!("response contains URLs"));
    }

    // Language-specific validation
    match language.to_lowercase().as_str() {
        "python" => validate_python(trimmed)?,
        _ => validate_c(trimmed)?,
    }

    Ok(())
}

/// Validate C code response.
fn validate_c(code: &str) -> Result<()> {
    // Check for balanced braces
    let open_braces = code.matches('{').count();
    let close_braces = code.matches('}').count();
    if open_braces != close_braces {
        return Err(anyhow!("mismatched braces ({} open, {} close)", open_braces, close_braces));
    }

    // Reject JavaScript-style code
    let lower = code.to_lowercase();
    if lower.contains("console.log") || lower.contains("console.error") {
        return Err(anyhow!("response appears to be JavaScript, not C"));
    }

    // Reject Python-style code
    if lower.contains("def ") && !code.contains("#define") {
        return Err(anyhow!("response appears to be Python, not C"));
    }

    // Check for multiple function definitions (project-level output)
    let function_pattern_count = count_function_definitions(code);
    if function_pattern_count > 1 {
        return Err(anyhow!(
            "response contains {} function definitions; expected at most 1",
            function_pattern_count
        ));
    }

    // Check for multiple struct definitions
    let struct_count = code.matches("struct ").count();
    // Allow "struct X {" but reject multiple separate struct definitions
    if struct_count > 1 && code.matches("};").count() > 1 {
        return Err(anyhow!("response contains multiple struct definitions"));
    }

    Ok(())
}

/// Validate Python code response.
fn validate_python(code: &str) -> Result<()> {
    // Check for C-style code in Python response
    if code.contains("printf(") || code.contains("int main(") {
        return Err(anyhow!("response appears to be C, not Python"));
    }

    // Check for multiple class/function definitions
    let def_count = code.matches("\ndef ").count() + if code.starts_with("def ") { 1 } else { 0 };
    let class_count = code.matches("\nclass ").count() + if code.starts_with("class ") { 1 } else { 0 };
    
    if def_count > 1 {
        return Err(anyhow!(
            "response contains {} function definitions; expected at most 1",
            def_count
        ));
    }

    if class_count > 1 {
        return Err(anyhow!(
            "response contains {} class definitions; expected at most 1",
            class_count
        ));
    }

    Ok(())
}

/// Count C function definitions in code.
/// Looks for patterns like "type name(..." followed by "{"
fn count_function_definitions(code: &str) -> usize {
    let lines: Vec<&str> = code.lines().collect();
    let mut count = 0;
    
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // Skip preprocessor directives
        if trimmed.starts_with('#') {
            continue;
        }
        
        // Look for function signature pattern: something ending with ) or ) {
        if trimmed.contains('(') && trimmed.contains(')') {
            // Check if this looks like a function definition (not a call)
            // Function definitions typically have a type before the name
            let before_paren = trimmed.split('(').next().unwrap_or("");
            let tokens: Vec<&str> = before_paren.split_whitespace().collect();
            
            if tokens.len() >= 2 {
                // Check if followed by { (on same line or next)
                if trimmed.ends_with('{') {
                    count += 1;
                } else if trimmed.ends_with(')') {
                    // Check next line for {
                    if i + 1 < lines.len() && lines[i + 1].trim().starts_with('{') {
                        count += 1;
                    }
                }
            }
        }
    }
    
    count
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_simple_c() {
        assert!(validate_candidate("int x = 5;", "c").is_ok());
        assert!(validate_candidate("for (int i = 0; i < 10; i++) {\n    printf(\"%d\\n\", i);\n}", "c").is_ok());
    }

    #[test]
    fn test_reject_multiple_functions() {
        let code = "int foo() {\n    return 1;\n}\n\nint bar() {\n    return 2;\n}";
        assert!(validate_candidate(code, "c").is_err());
    }

    #[test]
    fn test_reject_markdown() {
        assert!(validate_candidate("```c\nint x = 5;\n```", "c").is_err());
    }

    #[test]
    fn test_validate_python() {
        assert!(validate_candidate("x = 5", "python").is_ok());
        assert!(validate_candidate("for i in range(10):\n    print(i)", "python").is_ok());
    }
}
