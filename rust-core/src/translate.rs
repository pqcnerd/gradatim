//! Code generation from English instructions and structured hints.
//!
//! This module contains both the new hint-based translation functions
//! and the legacy string-based translation functions (kept for potential
//! future use or reference).

#![allow(dead_code)]

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::hints::{ArithmeticOp as HintArithmeticOp, StatementHint};
use crate::models::TranslateLineRequest;

#[derive(Debug, Clone)]
pub struct CodeStyle {
    pub indent_style: IndentStyle,
    pub indent_width: usize,
    pub brace_style: BraceStyle,
    pub max_line_length: usize,
    pub naming_convention: NamingConvention,
}

#[derive(Debug, Clone, Copy)]
pub enum IndentStyle {
    Tabs,
    Spaces,
}

#[derive(Debug, Clone, Copy)]
pub enum BraceStyle {
    KAndR,
    Allman,
    GNU,
    Whitesmiths,
}

#[derive(Debug, Clone, Copy)]
pub enum NamingConvention {
    CamelCase,
    SnakeCase,
    PascalCase,
}

impl Default for CodeStyle {
    fn default() -> Self {
        Self {
            indent_style: IndentStyle::Spaces,
            indent_width: 4,
            brace_style: BraceStyle::KAndR,
            max_line_length: 100,
            naming_convention: NamingConvention::SnakeCase,
        }
    }
}

static DEFAULT_STYLE: Lazy<CodeStyle> = Lazy::new(CodeStyle::default);

fn indent(level: usize) -> String {
    let style = &*DEFAULT_STYLE;
    match style.indent_style {
        IndentStyle::Tabs => "\t".repeat(level),
        IndentStyle::Spaces => " ".repeat(style.indent_width * level),
    }
}

/// Translate an inline action string (like "print a") into C code
fn translate_inline_action(action: &str) -> String {
    let trimmed = action.trim();
    let lower = trimmed.to_lowercase();
    
    // Handle "print X"
    if lower.starts_with("print ") || lower == "print" {
        let content = trimmed.get(6..).unwrap_or("").trim();
        if content.is_empty() {
            return "printf(\"\\n\");".to_string();
        }
        // Check if content is quoted
        if content.starts_with('"') || content.starts_with('\'') {
            let inner = content.trim_matches(|c| c == '"' || c == '\'');
            return format!("printf(\"{}\\n\");", inner);
        }
        // Assume it's a variable
        return format!("printf(\"%d\\n\", {});", content);
    }
    
    // Handle "return X"
    if lower.starts_with("return ") || lower == "return" {
        let value = trimmed.get(7..).unwrap_or("").trim();
        if value.is_empty() {
            return "return;".to_string();
        }
        return format!("return {};", value);
    }
    
    // Handle "set X to Y" or "set X Y"
    if lower.starts_with("set ") {
        let rest = trimmed.get(4..).unwrap_or("").trim();
        let parts: Vec<&str> = rest.splitn(3, ' ').collect();
        if parts.len() >= 2 {
            let var = parts[0];
            let value = if parts.len() >= 3 && parts[1].to_lowercase() == "to" {
                parts[2]
            } else {
                parts[1]
            };
            return format!("{} = {};", var, value);
        }
    }
    
    // Handle "increment X" or "decrement X"
    if lower.starts_with("increment ") {
        let var = trimmed.get(10..).unwrap_or("").trim();
        return format!("{}++;", var);
    }
    if lower.starts_with("decrement ") {
        let var = trimmed.get(10..).unwrap_or("").trim();
        return format!("{}--;", var);
    }
    
    // Handle "add X to Y" -> "Y += X;"
    if lower.starts_with("add ") {
        let rest = trimmed.get(4..).unwrap_or("").trim();
        if let Some(to_idx) = rest.to_lowercase().find(" to ") {
            let value = rest[..to_idx].trim();
            let target = rest[to_idx + 4..].trim();
            return format!("{} += {};", target, value);
        }
    }
    
    // Handle "call X" or "call X(args)"
    if lower.starts_with("call ") {
        let func = trimmed.get(5..).unwrap_or("").trim();
        if func.contains('(') {
            return format!("{};", func);
        }
        return format!("{}();", func);
    }
    
    // Handle "break" and "continue"
    if lower == "break" {
        return "break;".to_string();
    }
    if lower == "continue" {
        return "continue;".to_string();
    }
    
    // Fallback: just add a semicolon if it doesn't have one
    if trimmed.ends_with(';') {
        trimmed.to_string()
    } else {
        format!("{};", trimmed)
    }
}

/// Strip a word prefix and return the rest if it matches
fn strip_prefix_word<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if text.starts_with(prefix) {
        let rest = &text[prefix.len()..];
        if rest.is_empty() || rest.starts_with(' ') || rest.starts_with(':') {
            return Some(rest.trim_start());
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArithmeticOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone)]
struct ArithmeticInstruction {
    op: ArithmeticOp,
    left: String,
    right: String,
    target: Option<String>,
}

const DECLARE_KEYWORDS: &[&str] = &[
    "declare",
    "define",
    "create",
    "make",
    "set up",
    "setup",
    "make sure",
    "initialize",
    "init",
    "turn",
    "convert",
    "build",
    "form",
];

const LOOP_KEYWORDS: &[&str] = &[
    "loop",
    "for loop",
    "for each",
    "foreach",
    "for every",
    "for all",
    "iterate",
    "iterate over",
    "iterate through",
    "loop over",
    "loop through",
    "go through",
    "go over",
    "cycle through",
    "traverse",
    "walk through",
];

const IF_KEYWORDS: &[&str] = &[
    "if",
    "when",
    "whenever",
    "in case",
    "provided that",
    "assuming",
    "in the event",
    "should",
];

const ELSE_IF_KEYWORDS: &[&str] = &["else if", "otherwise if", "elsewhen", "else when"];

const LIST_STARTERS: &[&str] = &["list", "array", "vector", "arr", "collection"];

const ADD_KEYWORDS: &[&str] = &["add", "sum", "total", "combine", "plus", "tally", "sum up"];
const SUBTRACT_KEYWORDS: &[&str] = &[
    "subtract",
    "minus",
    "difference",
    "difference between",
    "difference of",
    "remove",
    "take away",
    "decrease",
];
const MULTIPLY_KEYWORDS: &[&str] = &["multiply", "product", "product of", "times"];
const DIVIDE_KEYWORDS: &[&str] = &["divide", "quotient", "quotient of", "split", "divide by"];

pub fn translate_line(request: &TranslateLineRequest) -> Result<String> {
    match request.language.to_lowercase().as_str() {
        "python" => translate_python(request),
        _ => translate_c(request),
    }
}

fn translate_c(request: &TranslateLineRequest) -> Result<String> {
    let line = request.english_line.trim();

    if line.is_empty() {
        return Err(anyhow!("Line is empty"));
    }

    if let Some(rest) = strip_declare_keyword(line) {
        return handle_declare(rest);
    }

    if starts_with_list_keyword(line) {
        return handle_declare(line);
    }

    if let Some(rest) = strip_keyword(line, "set") {
        return handle_assignment(rest);
    }

    if let Some(arith) = parse_arithmetic_instruction(line) {
        return build_c_arithmetic(&arith);
    }

    if let Some(rest) = strip_keyword(line, "increment") {
        return Ok(format!("{}++;", sanitize_identifier(rest)));
    }

    if let Some(rest) = strip_keyword(line, "decrement") {
        return Ok(format!("{}--;", sanitize_identifier(rest)));
    }

    if let Some(rest) = strip_keyword(line, "print") {
        return handle_print(rest);
    }

    if let Some(rest) = strip_keyword(line, "return") {
        return Ok(format!("return {};", rest.trim()));
    }

    if let Some(rest) = strip_if_keyword(line) {
        return handle_if(rest);
    }

    if let Some(rest) = strip_keyword(line, "struct") {
        return handle_struct_definition(rest);
    }

    if looks_like_main_request(line) {
        return Ok(generate_main_skeleton());
    }

    if let Some(rest) = strip_keyword(line, "function") {
        return handle_function_skeleton(rest);
    }
    if let Some(rest) = strip_keyword(line, "create function") {
        return handle_function_skeleton(rest);
    }
    if let Some(rest) = strip_keyword(line, "make function") {
        return handle_function_skeleton(rest);
    }

    if {
        let trimmed = line.trim();
        trimmed.eq_ignore_ascii_case("else") || trimmed.eq_ignore_ascii_case("otherwise")
    } {
        return Ok("else {\n    \n}".to_string());
    }

    if let Some(rest) = strip_else_if_keyword(line) {
        let condition = rest.trim();
        let c_expr = normalize_condition(condition);
        return Ok(format!("else if ({c_expr}) {{\n    \n}}"));
    }

    if line.to_lowercase().starts_with("end") {
        return Ok("}".to_string());
    }

    if let Some(rest) = strip_loop_keyword(line) {
        return handle_loop(rest);
    }

    Err(anyhow!("UNHANDLED: {}", line))
}

fn translate_python(request: &TranslateLineRequest) -> Result<String> {
    let line = request.english_line.trim();

    if line.is_empty() {
        return Err(anyhow!("Line is empty"));
    }

    if let Some(rest) = strip_keyword(line, "declare") {
        return handle_py_declare(rest);
    }

    if starts_with_list_keyword(line) {
        return handle_py_declare(line);
    }

    if let Some(rest) = strip_keyword(line, "set") {
        return handle_py_assignment(rest);
    }

    if let Some(rest) = strip_keyword(line, "print") {
        return Ok(format!("print({})", rest.trim()));
    }

    if let Some(rest) = strip_keyword(line, "return") {
        return Ok(format!("return {}", rest.trim()));
    }

    if let Some(arith) = parse_arithmetic_instruction(line) {
        return build_python_arithmetic(&arith);
    }

    if let Some(rest) = strip_keyword(line, "if") {
        return handle_py_if(rest);
    }

    if line.trim().eq_ignore_ascii_case("else") {
        return Ok("else:\n    pass".to_string());
    }

    if line.to_lowercase().starts_with("elif ") {
        let cond = normalize_condition(line[4..].trim());
        return Ok(format!("elif {}:\n    pass", cond));
    }

    if line.to_lowercase().starts_with("end") {
        return Ok(String::new());
    }

    if let Some(rest) = strip_keyword(line, "loop")
        .or_else(|| strip_keyword(line, "for loop"))
        .or_else(|| strip_keyword(line, "for"))
    {
        return handle_py_loop(rest);
    }

    if let Some(rest) = strip_keyword(line, "increment") {
        return Ok(format!("{} += 1", sanitize_identifier(rest)));
    }

    if let Some(rest) = strip_keyword(line, "decrement") {
        return Ok(format!("{} -= 1", sanitize_identifier(rest)));
    }

    Err(anyhow!("UNHANDLED: {}", line))
}

fn handle_declare(remainder: &str) -> Result<String> {
    let remainder = remainder.trim();
    if remainder.is_empty() {
        return Err(anyhow!("Nothing to declare"));
    }

    let (normalized, list_kind) = normalize_declare_input(remainder);
    let (names_part, value) = split_value(&normalized);
    let names: Vec<_> = names_part
        .split(',')
        .map(|segment| segment.trim())
        .filter(|segment| !segment.is_empty())
        .collect();

    if names.is_empty() {
        return Err(anyhow!("Could not parse variable names"));
    }

    if list_kind {
        let size_expr = value.as_deref().unwrap_or("10");
        let decls: Vec<String> = names
            .into_iter()
            .map(|name| {
                let ident = sanitize_identifier(name);
                format!("int {ident}[{size}];", size = size_expr)
            })
            .collect();
        if decls.is_empty() {
            return Err(anyhow!("List declaration missing identifier"));
        }
        return Ok(decls.join("\n"));
    }

    let declarations: Vec<String> = names
        .into_iter()
        .map(|name| {
            if let Some(value) = value.as_deref() {
                format!("{} = {}", sanitize_identifier(name), value)
            } else {
                sanitize_identifier(name).to_string()
            }
        })
        .collect();

    Ok(format!("int {};", declarations.join(", ")))
}

fn handle_assignment(remainder: &str) -> Result<String> {
    let rem = remainder.trim();
    if rem.is_empty() {
        return Err(anyhow!("Assignment target missing"));
    }

    let lower = rem.to_lowercase();
    if let Some(idx) = lower.find(" to ") {
        let (lhs, rhs) = rem.split_at(idx);
        let rhs = &rhs[4..]; // Skip " to "
        return Ok(format!("{} = {};", sanitize_identifier(lhs.trim()), rhs.trim()));
    }

    Err(anyhow!("Use the `set <name> to <value>` phrasing for assignments"))
}

fn handle_print(remainder: &str) -> Result<String> {
    let message = remainder.trim();
    if message.is_empty() {
        return Err(anyhow!("Print statement requires content"));
    }

    if message.starts_with('"') && message.ends_with('"') && message.len() > 1 {
        let literal = &message[1..message.len() - 1];
        Ok(format!("printf(\"{}\\n\");", literal))
    } else {
        Ok(format!("printf(\"%s\\n\", {message});"))
    }
}

fn handle_if(segment: &str) -> Result<String> {
    let (condition_text, then_action, else_action) = parse_conditional_actions(segment);

    let cond = normalize_condition(condition_text.trim());
    if cond.is_empty() {
        return Err(anyhow!("Missing if condition"));
    }

    let mut lines = Vec::new();
    lines.push(format!("if ({cond}) {{"));
    lines.push(build_c_body_line(then_action.as_deref(), None));
    lines.push("}".to_string());

    if let Some(else_act) = else_action {
        lines.push("else {".to_string());
        lines.push(build_c_body_line(Some(else_act.as_str()), None));
        lines.push("}".to_string());
    }

    Ok(lines.join("\n"))
}

fn handle_loop(remainder: &str) -> Result<String> {
    let lower = remainder.to_lowercase();
    if !lower.contains("to") {
        return handle_collection_loop_c(remainder);
    }

    let (iterator, range_section) = extract_iterator_and_range(remainder);
    let iter_name = iterator.as_str();
    let mut range_segment = range_section.trim();
    if range_segment.is_empty() {
        return Err(anyhow!("Loop bounds are missing"));
    }
    if range_segment.to_lowercase().starts_with("from ") {
        range_segment = range_segment[4..].trim_start();
    }

    let lower_range = range_segment.to_lowercase();
    let to_rel = lower_range.find("to").ok_or_else(|| {
        anyhow!("Loop sentences should include `to <end>` after the starting expression")
    })?;

    let start = range_segment[..to_rel].trim();
    let tail = range_segment[to_rel + 2..].trim();
    if start.is_empty() || tail.is_empty() {
        return Err(anyhow!("Loop bounds are incomplete"));
    }

    let (end_expr, action) = split_range_and_action(tail);
    if end_expr.is_empty() {
        return Err(anyhow!("Loop end expression is missing"));
    }

    let start_expr = extract_range_value(start);
    let mut lines = Vec::with_capacity(3);
    lines.push(format!(
        "for (int {iter} = {start}; {iter} < {end}; {iter}++) {{",
        iter = iter_name,
        start = start_expr,
        end = end_expr
    ));
    lines.push(build_c_body_line(action.as_deref(), Some(iter_name)));
    lines.push("}".to_string());

    Ok(lines.join("\n"))
}

fn handle_py_declare(remainder: &str) -> Result<String> {
    let (normalized, list_kind) = normalize_declare_input(remainder);
    let (names_part, value) = split_value(&normalized);
    let names: Vec<_> = names_part
        .split(',')
        .map(|segment| sanitize_identifier(segment))
        .filter(|segment| !segment.is_empty())
        .collect();

    if names.is_empty() {
        return Err(anyhow!("Nothing to declare"));
    }

    if list_kind {
        let size_expr = value.as_deref().unwrap_or("10");
        let decls: Vec<String> = names
            .into_iter()
            .map(|name| format!("{name} = [0] * {size}", size = size_expr))
            .collect();
        return Ok(decls.join("\n"));
    }

    let rhs = value.as_deref().unwrap_or("None");

    if names.len() == 1 {
        Ok(format!("{} = {}", names[0], rhs))
    } else {
        Ok(format!("{} = {}", names.join(" = "), rhs))
    }
}

fn handle_py_assignment(remainder: &str) -> Result<String> {
    let rem = remainder.trim();
    if rem.is_empty() {
        return Err(anyhow!("Assignment target missing"));
    }

    let lower = rem.to_lowercase();
    if let Some(idx) = lower.find(" to ") {
        let (lhs, rhs) = rem.split_at(idx);
        let rhs = &rhs[4..];
        return Ok(format!("{} = {}", sanitize_identifier(lhs.trim()), rhs.trim()));
    }

    Err(anyhow!("Use the `set <name> to <value>` phrasing for assignments"))
}

fn handle_py_loop(remainder: &str) -> Result<String> {
    let lower = remainder.to_lowercase();
    if !lower.contains("to") {
        return handle_python_collection_loop(remainder);
    }

    let (iterator, range_section) = extract_iterator_and_range(remainder);
    let iter_name = iterator.as_str();
    let mut range_segment = range_section.trim();
    if range_segment.is_empty() {
        return Err(anyhow!("Loop bounds are missing"));
    }
    if range_segment.to_lowercase().starts_with("from ") {
        range_segment = range_segment[4..].trim_start();
    }

    let lower_range = range_segment.to_lowercase();
    let to_rel = lower_range.find("to").ok_or_else(|| {
        anyhow!("Loop sentences should include `to <end>` after the starting expression")
    })?;

    let start_raw = range_segment[..to_rel].trim();
    let tail = range_segment[to_rel + 2..].trim();
    if start_raw.is_empty() || tail.is_empty() {
        return Err(anyhow!("Loop bounds are incomplete"));
    }

    let (end_expr, action) = split_range_and_action(tail);
    if end_expr.is_empty() {
        return Err(anyhow!("Loop end expression is missing"));
    }

    let start_expr = extract_range_value(start_raw);
    let mut lines = Vec::new();
    lines.push(format!(
        "for {iter} in range({start}, {end}):",
        iter = iter_name,
        start = start_expr,
        end = end_expr
    ));
    lines.push(build_python_body_line(action.as_deref(), Some(iter_name)));

    Ok(lines.join("\n"))
}

fn split_value(input: &str) -> (String, Option<String>) {
    let mut tokens: Vec<&str> = input
        .split_whitespace()
        .map(|token| token.trim_matches(|c: char| c == ',' || c == ';'))
        .collect();
    if tokens.is_empty() {
        return (String::new(), None);
    }

    let mut value: Option<String> = None;
    let mut idx = tokens.len();
    while idx > 0 {
        let token = tokens[idx - 1];
        if looks_like_value(token) {
            value = Some(token.to_string());
            tokens.truncate(idx - 1);
            remove_trailing_size_keywords(&mut tokens);
            break;
        }

        let lower = token.to_lowercase();
        if matches!(
            lower.as_str(),
            "ints" | "integers" | "numbers" | "values" | "elements" | "items"
        ) && idx >= 2
        {
            let candidate = tokens[idx - 2];
            if looks_like_value(candidate) {
                value = Some(candidate.to_string());
                tokens.truncate(idx - 2);
                remove_trailing_size_keywords(&mut tokens);
                break;
            }
        }
        idx -= 1;
    }

    (tokens.join(" "), value)
}

fn remove_trailing_size_keywords(tokens: &mut Vec<&str>) {
    while let Some(last) = tokens.last() {
        let lower = last.to_lowercase();
        if matches!(
            lower.as_str(),
            "of"
                | "size"
                | "sized"
                | "length"
                | "slots"
                | "elements"
                | "values"
                | "items"
                | "ints"
                | "integers"
                | "numbers"
                | "count"
        ) {
            tokens.pop();
        } else {
            break;
        }
    }
}

fn looks_like_value(segment: &str) -> bool {
    segment.parse::<f64>().is_ok()
        || matches!(
            segment.to_lowercase().as_str(),
            "true" | "false" | "null" | "nullptr"
        )
        || segment.starts_with('"') && segment.ends_with('"')
}

fn strip_keyword<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let keyword_lower = keyword.to_lowercase();
    if line.len() < keyword_lower.len() {
        return None;
    }

    let head = &line[..keyword_lower.len()];
    if head.eq_ignore_ascii_case(&keyword_lower) {
        Some(line[keyword_lower.len()..].trim_start())
    } else {
        None
    }
}

fn strip_any_keyword<'a>(line: &'a str, keywords: &[&str]) -> Option<&'a str> {
    for keyword in keywords {
        if let Some(rest) = strip_keyword(line, keyword) {
            return Some(rest);
        }
    }
    None
}

fn strip_declare_keyword(line: &str) -> Option<&str> {
    let rest = strip_any_keyword(line, DECLARE_KEYWORDS)?;
    let next = rest.trim_start().to_lowercase();
    if next.starts_with("function")
        || next.starts_with("main")
        || next.starts_with("entry")
        || next.starts_with("program")
    {
        return None;
    }
    Some(rest)
}

fn strip_loop_keyword(line: &str) -> Option<&str> {
    strip_any_keyword(line, LOOP_KEYWORDS)
}

fn strip_if_keyword(line: &str) -> Option<&str> {
    strip_any_keyword(line, IF_KEYWORDS)
}

fn strip_else_if_keyword(line: &str) -> Option<&str> {
    strip_any_keyword(line, ELSE_IF_KEYWORDS)
}

fn sanitize_identifier(input: &str) -> String {
    input
        .trim()
        .split_whitespace()
        .last()
        .unwrap_or(input)
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

fn normalize_condition(condition: &str) -> String {
    condition
        .replace(" is greater than ", " > ")
        .replace(" is less than ", " < ")
        .replace(" equals ", " == ")
        .replace(" is equal to ", " == ")
        .replace(" is not equal to ", " != ")
}

fn split_range_and_action(segment: &str) -> (String, Option<String>) {
    let lower = segment.to_lowercase();
    if let Some(idx) = lower.find("printing") {
        let range_part = segment[..idx].trim().to_string();
        let action_part = clean_action_segment(&segment[idx + "printing".len()..]);
        return (range_part, if action_part.is_empty() { None } else { Some(action_part) });
    }
    if let Some(idx) = lower.find("print") {
        let range_part = segment[..idx].trim().to_string();
        let action_part = clean_action_segment(&segment[idx + "print".len()..]);
        return (range_part, if action_part.is_empty() { None } else { Some(action_part) });
    }
    for key in [" in ", " over "] {
        if let Some(idx) = lower.find(key) {
            let range_part = segment[..idx].trim().to_string();
            let action_part = clean_action_segment(&segment[idx + key.len()..]);
            let formatted = if action_part.is_empty() {
                None
            } else {
                Some(format!("in {}", action_part))
            };
            return (range_part, formatted);
        }
    }
    (segment.trim().to_string(), None)
}

fn parse_arithmetic_instruction(line: &str) -> Option<ArithmeticInstruction> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(rest) = strip_any_keyword(trimmed, ADD_KEYWORDS) {
        return parse_add_mul(rest, ArithmeticOp::Add, true);
    }

    if let Some(rest) = strip_any_keyword(trimmed, MULTIPLY_KEYWORDS) {
        return parse_add_mul(rest, ArithmeticOp::Multiply, true);
    }

    if let Some(rest) = strip_any_keyword(trimmed, DIVIDE_KEYWORDS) {
        return parse_divide(rest);
    }

    if let Some(rest) = strip_any_keyword(trimmed, SUBTRACT_KEYWORDS) {
        return parse_subtract(rest);
    }

    None
}

fn parse_add_mul(remainder: &str, op: ArithmeticOp, allow_and: bool) -> Option<ArithmeticInstruction> {
    let lower = remainder.to_lowercase();

    if allow_and {
        if let Some(idx) = lower.find(" and ") {
            let left = remainder[..idx].trim();
            let after = remainder[idx + 5..].trim();
            let (right, target) = split_operand_and_target(after);
            if left.is_empty() || right.is_empty() {
                return None;
            }
            return Some(ArithmeticInstruction {
                op,
                left: left.to_string(),
                right,
                target,
            });
        }
    }

    if op == ArithmeticOp::Multiply || op == ArithmeticOp::Divide {
        if let Some(idx) = lower.find(" by ") {
            let left = remainder[..idx].trim();
            let after = remainder[idx + 4..].trim();
            let (right, target) = split_operand_and_target(after);
            if left.is_empty() || right.is_empty() {
                return None;
            }
            return Some(ArithmeticInstruction {
                op,
                left: left.to_string(),
                right,
                target,
            });
        }
    }

    None
}

fn parse_divide(remainder: &str) -> Option<ArithmeticInstruction> {
    parse_add_mul(remainder, ArithmeticOp::Divide, true)
}

fn parse_subtract(remainder: &str) -> Option<ArithmeticInstruction> {
    let lower = remainder.to_lowercase();
    if let Some(idx) = lower.find(" from ") {
        let subtrahend = remainder[..idx].trim();
        let after = remainder[idx + 6..].trim();
        let (minuend, target) = split_operand_and_target(after);
        if subtrahend.is_empty() || minuend.is_empty() {
            return None;
        }
        return Some(ArithmeticInstruction {
            op: ArithmeticOp::Subtract,
            left: minuend,
            right: subtrahend.to_string(),
            target,
        });
    }

    if let Some(instr) = parse_add_mul(remainder, ArithmeticOp::Subtract, true) {
        return Some(instr);
    }

    None
}

fn split_operand_and_target(segment: &str) -> (String, Option<String>) {
    let trimmed = segment.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }
    let lower = trimmed.to_lowercase();
    for key in [" into ", " to ", " store in ", " store into ", " as "] {
        if let Some(idx) = lower.find(key) {
            let value = trimmed[..idx].trim().to_string();
            let target = trimmed[idx + key.len()..].trim();
            if !target.is_empty() {
                return (value, Some(target.to_string()));
            }
        }
    }
    (trimmed.to_string(), None)
}

fn build_c_arithmetic(instr: &ArithmeticInstruction) -> Result<String> {
    let op_symbol = match instr.op {
        ArithmeticOp::Add => "+",
        ArithmeticOp::Subtract => "-",
        ArithmeticOp::Multiply => "*",
        ArithmeticOp::Divide => "/",
    };

    let expression = format!("{} {} {}", instr.left.trim(), op_symbol, instr.right.trim());

    if let Some(target_raw) = instr.target.as_ref() {
        let target = sanitize_identifier(target_raw);
        if !target.is_empty() {
            return Ok(format!("{target} = {expression};"));
        }
    }

    let default_name = match instr.op {
        ArithmeticOp::Add => "sum",
        ArithmeticOp::Subtract => "difference",
        ArithmeticOp::Multiply => "product",
        ArithmeticOp::Divide => "quotient",
    };

    Ok(format!("int {default_name} = {expression};"))
}

fn build_python_arithmetic(instr: &ArithmeticInstruction) -> Result<String> {
    let op_symbol = match instr.op {
        ArithmeticOp::Add => "+",
        ArithmeticOp::Subtract => "-",
        ArithmeticOp::Multiply => "*",
        ArithmeticOp::Divide => "/",
    };

    let expression = format!("{} {} {}", instr.left.trim(), op_symbol, instr.right.trim());

    if let Some(target_raw) = instr.target.as_ref() {
        let target = sanitize_identifier(target_raw);
        if !target.is_empty() {
            return Ok(format!("{target} = {expression}"));
        }
    }

    let default_name = match instr.op {
        ArithmeticOp::Add => "sum_result",
        ArithmeticOp::Subtract => "difference_result",
        ArithmeticOp::Multiply => "product_result",
        ArithmeticOp::Divide => "quotient_result",
    };

    Ok(format!("{default_name} = {expression}"))
}

fn clean_action_segment(action: &str) -> String {
    action
        .trim()
        .trim_start_matches(|c: char| c.is_whitespace() || c == ':' || c == ',')
        .trim()
        .trim_matches('.')
        .to_string()
}

fn extract_range_value(segment: &str) -> String {
    segment
        .split('=')
        .last()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .unwrap_or(segment.trim())
        .to_string()
}

fn parse_conditional_actions(segment: &str) -> (String, Option<String>, Option<String>) {
    let mut main = segment;
    let mut else_action: Option<String> = None;

    let lower = segment.to_lowercase();
    for key in [" otherwise ", " else "] {
        if let Some(idx) = lower.find(key) {
            let action_raw = clean_action_segment(&segment[idx + key.len()..]);
            else_action = if action_raw.is_empty() { None } else { Some(action_raw) };
            main = &segment[..idx];
            break;
        }
    }

    let mut then_action: Option<String> = None;
    let lower_main = main.to_lowercase();
    for key in [" then ", " print ", " do "] {
        if let Some(idx) = lower_main.find(key) {
            let action_raw = clean_action_segment(&main[idx + key.len()..]);
            then_action = if action_raw.is_empty() { None } else { Some(action_raw) };
            main = &main[..idx];
            break;
        }
    }

    let condition = main.trim().trim_matches(|c| c == '(' || c == ')').trim().to_string();
    (condition, then_action, else_action)
}

enum Action {
    Literal(String),
    Identifier(String),
    Printf(String),
    Raw(String),
    CollectionIndex(String),
}

fn normalize_action(action: Option<&str>) -> Option<Action> {
    let cleaned = action?.trim();
    if cleaned.is_empty() {
        return None;
    }

    if cleaned.starts_with('"') && cleaned.ends_with('"') && cleaned.len() >= 2 {
        return Some(Action::Literal(
            cleaned.trim_matches('"').replace("\\\"", "\""),
        ));
    }

    let lower = cleaned.to_lowercase();
    if lower.starts_with("in ") || lower.starts_with("over ") {
        let ident = cleaned.split_whitespace().last().unwrap_or("");
        let sanitized = sanitize_identifier(ident);
        if sanitized.is_empty() {
            return None;
        }
        return Some(Action::CollectionIndex(sanitized));
    }

    if lower.starts_with("printf") || lower.starts_with("puts") {
        return Some(Action::Printf(cleaned.to_string()));
    }

    if lower.starts_with("return") || lower.starts_with("break") || lower.starts_with("continue") {
        return Some(Action::Raw(cleaned.to_string()));
    }

    if cleaned.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') {
        return Some(Action::Identifier(sanitize_identifier(cleaned)));
    }

    Some(Action::Raw(cleaned.to_string()))
}

fn build_c_body_line(action: Option<&str>, iterator: Option<&str>) -> String {
    let iter_name = iterator.unwrap_or("i");
    let pad = indent(1);
    match normalize_action(action) {
        Some(Action::Literal(expr)) => format!("{pad}printf(\"{}\\n\");", expr),
        Some(Action::Identifier(expr)) => format!("{pad}printf(\"%s\\n\", {expr});"),
        Some(Action::CollectionIndex(expr)) => format!("{pad}printf(\"%d\\n\", {expr}[{iter_name}]);"),
        Some(Action::Printf(expr)) => {
            if expr.trim_end().ends_with(';') {
                format!("{pad}{expr}")
            } else {
                format!("{pad}{expr};")
            }
        }
        Some(Action::Raw(expr)) => {
            if expr.trim_end().ends_with(';') {
                format!("{pad}{expr}")
            } else {
                format!("{pad}{expr};")
            }
        }
        None => pad,
    }
}

fn build_python_body_line(action: Option<&str>, iterator: Option<&str>) -> String {
    let iter_name = iterator.unwrap_or("i");
    let pad = indent(1);
    match normalize_action(action) {
        Some(Action::Literal(expr)) => format!("{pad}print(\"{}\")", expr),
        Some(Action::Identifier(expr)) => format!("{pad}print({expr})"),
        Some(Action::CollectionIndex(expr)) => format!("{pad}print({expr}[{iter_name}])"),
        Some(Action::Printf(expr)) => format!("{pad}print({expr})"),
        Some(Action::Raw(expr)) => format!("{pad}{expr}"),
        None => format!("{pad}pass"),
    }
}

fn handle_collection_loop_c(text: &str) -> Result<String> {
    let keywords = [(" in ", 4usize), (" over ", 6usize)];
    let lower = text.to_lowercase();
    let mut match_idx = None;
    let mut key_len = 0;
    for (key, len) in keywords {
        if let Some(idx) = lower.find(key) {
            match_idx = Some(idx);
            key_len = len;
            break;
        }
    }

    let idx = match_idx.ok_or_else(|| {
        anyhow!("Loop sentences should specify a range (`to`) or a collection using `in <list>`")
    })?;

    let after = &text[idx + key_len..];
    let (collection, remainder) = extract_collection_identifier(after);
    if collection.is_empty() {
        return Err(anyhow!("Could not determine the list/array name for the loop"));
    }

    let action_raw = clean_action_segment(remainder);
    let default_action = if action_raw.is_empty() {
        None
    } else {
        Some(action_raw)
    };

    let mut lines = Vec::new();
    lines.push(format!(
        "for (int i = 0; i < sizeof({col}) / sizeof({col}[0]); i++) {{",
        col = collection
    ));
    lines.push(build_c_body_line(default_action.as_deref(), Some("i")));
    lines.push("}".to_string());

    Ok(lines.join("\n"))
}

fn handle_python_collection_loop(text: &str) -> Result<String> {
    let keywords = [(" in ", 4usize), (" over ", 6usize)];
    let lower = text.to_lowercase();
    let mut match_idx = None;
    let mut key_len = 0;
    for (key, len) in keywords {
        if let Some(idx) = lower.find(key) {
            match_idx = Some(idx);
            key_len = len;
            break;
        }
    }

    let idx = match_idx.ok_or_else(|| {
        anyhow!("Loop sentences should specify a range (`to`) or a collection using `in <list>`")
    })?;

    let after = &text[idx + key_len..];
    let (collection, remainder) = extract_collection_identifier(after);
    if collection.is_empty() {
        return Err(anyhow!("Could not determine the list/array name for the loop"));
    }

    let action_raw = clean_action_segment(remainder);
    let iter_name = "item";
    let default_action = if action_raw.is_empty() {
        None
    } else {
        Some(action_raw)
    };

    let mut lines = Vec::new();
    lines.push(format!("for {iter} in {col}:", iter = iter_name, col = collection));
    lines.push(build_python_body_line(default_action.as_deref(), None));

    Ok(lines.join("\n"))
}

fn extract_iterator_and_range<'a>(input: &'a str) -> (String, &'a str) {
    let trimmed = input.trim_start();
    if trimmed.is_empty() {
        return ("i".to_string(), trimmed);
    }

    let lower = trimmed.to_lowercase();
    if lower.starts_with("from ") {
        return ("i".to_string(), trimmed[4..].trim_start());
    }

    let (first_token, tail) = split_first_token(trimmed);
    if first_token.is_empty() {
        return ("i".to_string(), trimmed);
    }

    let first_char_is_alpha = first_token
        .chars()
        .next()
        .map(|c| c.is_ascii_alphabetic() || c == '_')
        .unwrap_or(false);
    let candidate = sanitize_identifier(first_token);
    if !first_char_is_alpha || candidate.is_empty() {
        return ("i".to_string(), trimmed);
    }

    let tail_trim = tail.trim_start();
    if tail_trim.to_lowercase().starts_with("from ") {
        return (candidate, tail_trim[4..].trim_start());
    }
    if tail_trim.starts_with(":=") {
        return (candidate, tail_trim[2..].trim_start());
    }
    if tail_trim.starts_with('=') {
        return (candidate, tail_trim[1..].trim_start());
    }
    for prefix in ["equals", "equal to", "starts at", "starting at", "begin at", "begins at"] {
        let lower_tail = tail_trim.to_lowercase();
        if lower_tail.starts_with(prefix) {
            let offset = prefix.len();
            return (
                candidate,
                tail_trim[offset..].trim_start_matches(|c: char| c == ' '),
            );
        }
    }
    let first_tail_char = tail_trim.chars().next().unwrap_or(' ');
    if first_tail_char.is_ascii_digit() || matches!(first_tail_char, '-' | '+') {
        return (candidate, tail_trim);
    }

    ("i".to_string(), trimmed)
}

fn extract_collection_identifier(fragment: &str) -> (String, &str) {
    let mut remainder = fragment.trim_start();
    loop {
        if remainder.is_empty() {
            return (String::new(), remainder);
        }
        let (token, rest) = split_first_token(remainder);
        if token.is_empty() {
            return (String::new(), remainder);
        }
        let lower = token.to_lowercase();
        if matches!(lower.as_str(), "list" | "array" | "vector" | "arr" | "of") {
            remainder = rest.trim_start();
            continue;
        }
        let ident = sanitize_identifier(token);
        return (ident, rest);
    }
}

fn split_first_token(input: &str) -> (&str, &str) {
    let trimmed = input.trim_start();
    if trimmed.is_empty() {
        return ("", trimmed);
    }
    for (idx, ch) in trimmed.char_indices() {
        if ch.is_whitespace() {
            let (head, tail) = trimmed.split_at(idx);
            return (head, tail);
        }
    }
    (trimmed, "")
}

fn starts_with_list_keyword(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_lowercase();
    for keyword in LIST_STARTERS {
        if lower.starts_with(keyword) {
            let boundary = keyword.len();
            if lower.len() == boundary || lower.as_bytes().get(boundary).map(|b| *b == b' ').unwrap_or(false) {
                return true;
            }
        }
    }
    false
}

fn handle_struct_definition(rest: &str) -> Result<String> {
    let trimmed = rest.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Struct name is missing"));
    }

    let lower = trimmed.to_lowercase();
    let mut fields_text = "";
    let mut name_part = trimmed;
    for key in [" with ", " containing ", " having "] {
        if let Some(idx) = lower.find(key) {
            name_part = trimmed[..idx].trim();
            fields_text = trimmed[idx + key.len()..].trim();
            break;
        }
    }

    let struct_name = sanitize_identifier(name_part);
    if struct_name.is_empty() {
        return Err(anyhow!("Struct name is missing"));
    }

    let fields = parse_struct_fields(fields_text);
    let body = if fields.is_empty() {
        "    int value;".to_string()
    } else {
        fields
            .into_iter()
            .map(|(ty, name)| format!("    {ty} {name};"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    Ok(format!("struct {name} {{\n{body}\n}};", name = struct_name))
}

fn parse_struct_fields(text: &str) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    for chunk in text
        .split(|c| matches!(c, ',' | ';'))
        .flat_map(|part| part.split(" and "))
    {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }
        let words: Vec<&str> = chunk.split_whitespace().collect();
        if words.is_empty() {
            continue;
        }

        let mut field_type = "int";
        let mut field_name = "";
        for word in words.iter().rev() {
            if let Some(mapped) = map_c_type(word) {
                field_type = mapped;
                continue;
            }
            field_name = word;
            break;
        }

        let sanitized = sanitize_identifier(field_name);
        if sanitized.is_empty() {
            continue;
        }
        fields.push((field_type.to_string(), sanitized));
    }
    fields
}

fn handle_function_skeleton(rest: &str) -> Result<String> {
    let trimmed = rest.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Function name is missing"));
    }

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let name_token = parts.next().unwrap_or("func");
    let remainder = parts.next().unwrap_or("").trim_start();

    let func_name = sanitize_identifier(name_token);
    if func_name.is_empty() {
        return Err(anyhow!("Function name is missing"));
    }

    let lower_remainder = remainder.to_lowercase();
    let returning_idx = lower_remainder.find("returning");
    let (params_segment, return_segment) = if let Some(idx) = returning_idx {
        let (pre, post) = remainder.split_at(idx);
        let ret = post["returning".len()..].trim();
        (pre.trim(), ret)
    } else {
        (remainder, "")
    };

    let return_type = if return_segment.is_empty() {
        "int"
    } else {
        let first_word = return_segment.split_whitespace().next().unwrap_or("int");
        map_c_type(first_word).unwrap_or("int")
    };

    let params = parse_params_segment(params_segment);
    let params_str = if params.is_empty() {
        "void".to_string()
    } else {
        params
            .into_iter()
            .map(|(ty, name)| format!("{ty} {name}"))
            .collect::<Vec<_>>()
            .join(", ")
    };

    Ok(format!(
        "{ret} {name}({params}) {{\n    \n}}",
        ret = return_type,
        name = func_name,
        params = params_str
    ))
}

fn parse_params_segment(segment: &str) -> Vec<(String, String)> {
    let mut params = Vec::new();
    let tokens: Vec<&str> = segment.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i].trim_matches(|c: char| c == ',' || c == ';');
        if token.is_empty() || is_param_filler(token) {
            i += 1;
            continue;
        }

        if let Some(mapped) = map_c_type(token) {
            if i + 1 < tokens.len() {
                let ident_token =
                    tokens[i + 1].trim_matches(|c: char| c == ',' || c == ';' || c == '.');
                if is_param_filler(ident_token) {
                    i += 1;
                    continue;
                }
                let ident = sanitize_identifier(ident_token);
                if !ident.is_empty() {
                    params.push((mapped.to_string(), ident));
                    i += 2;
                    continue;
                }
            }
        }

        i += 1;
    }
    params
}

fn map_c_type(word: &str) -> Option<&'static str> {
    match word.to_lowercase().as_str() {
        "int" | "ints" | "integer" | "integers" => Some("int"),
        "float" | "floats" => Some("float"),
        "double" | "doubles" => Some("double"),
        "char" | "chars" => Some("char"),
        "string" | "strings" => Some("char *"),
        "bool" | "boolean" | "booleans" => Some("bool"),
        "void" => Some("void"),
        _ => None,
    }
}

fn is_param_filler(word: &str) -> bool {
    matches!(
        word.to_lowercase().as_str(),
        "" | "taking"
            | "parameters"
            | "parameter"
            | "args"
            | "arguments"
            | "and"
            | "with"
            | "named"
            | "called"
            | "each"
            | "every"
            | "comma"
            | "two"
            | "three"
            | "four"
    )
}

fn looks_like_main_request(line: &str) -> bool {
    let lower = line.trim().to_lowercase();
    lower.starts_with("create main")
        || lower.starts_with("make main")
        || lower.starts_with("write main")
        || lower.starts_with("build main")
        || lower.contains("entry point")
        || lower == "main"
}

fn generate_main_skeleton() -> String {
    "int main(void) {\n    return 0;\n}".to_string()
}

fn normalize_declare_input(input: &str) -> (String, bool) {
    let mut is_list = false;
    let mut filtered = Vec::new();
    for token in input.split_whitespace() {
        let clean = token.trim_matches(|c: char| c == ',' || c == ';');
        let lower = clean.to_lowercase();
        if matches!(
            lower.as_str(),
            "list" | "array" | "vector" | "arr" | "collection" | "table"
        ) {
            is_list = true;
            continue;
        }
        if matches!(
            lower.as_str(),
            "of" | "a" | "an" | "with" | "the" | "this" | "that" | "be" | "is" | "are" | "into"
                | "in" | "as" | "named" | "called" | "should" | "to"
        ) {
            continue;
        }
        filtered.push(clean);
    }
    (filtered.join(" "), is_list)
}

fn handle_py_if(segment: &str) -> Result<String> {
    let (condition_text, then_action, else_action) = parse_conditional_actions(segment);

    let cond = normalize_condition(condition_text.trim());
    if cond.is_empty() {
        return Err(anyhow!("Missing if condition"));
    }

    let mut lines = Vec::new();
    lines.push(format!("if {}:", cond));
    lines.push(build_python_body_line(then_action.as_deref(), None));

    if let Some(else_act) = else_action {
        lines.push("else:".to_string());
        lines.push(build_python_body_line(Some(else_act.as_str()), None));
    }

    Ok(lines.join("\n"))
}

// ============================================================================
// Hint-Based Code Generation
// ============================================================================

/// Generate code from a structured hint.
/// This is the new primary code generation path that works with preprocessed hints.
pub fn translate_from_hint(hint: &StatementHint, language: &str) -> Result<String> {
    match language.to_lowercase().as_str() {
        "python" => translate_hint_python(hint),
        _ => translate_hint_c(hint),
    }
}

fn normalize_array_index_c(array: &str, index: &str) -> String {
    if index == "last" {
        format!("(sizeof({0}) / sizeof({0}[0]) - 1)", array)
    } else {
        index.to_string()
    }
}

fn normalize_array_index_python(array: &str, index: &str) -> String {
    if index == "last" {
        format!("len({}) - 1", array)
    } else {
        index.to_string()
    }
}

static FLOAT_LITERAL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\d+\.\d+").expect("valid float literal regex")
});
static FLOAT_SUFFIX_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d+(?:\.\d+)?f$").expect("valid float suffix regex")
});
static LONG_SUFFIX_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d+l$").expect("valid long suffix regex")
});
static UNSIGNED_SUFFIX_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d+u$").expect("valid unsigned suffix regex")
});
static UNSIGNED_LONG_SUFFIX_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d+ul$|^\d+lu$").expect("valid unsigned long suffix regex")
});

fn infer_c_type_from_expression(expr: &str) -> Option<&'static str> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_lowercase();

    if lower.starts_with('"') && lower.ends_with('"') {
        return Some("char *");
    }
    if lower.starts_with('\'') && lower.ends_with('\'') {
        return Some("char");
    }
    if UNSIGNED_LONG_SUFFIX_RE.is_match(&lower) {
        return Some("unsigned long");
    }
    if LONG_SUFFIX_RE.is_match(&lower) {
        return Some("long");
    }
    if UNSIGNED_SUFFIX_RE.is_match(&lower) {
        return Some("unsigned int");
    }
    if FLOAT_SUFFIX_RE.is_match(&lower) {
        return Some("float");
    }
    if FLOAT_LITERAL_RE.is_match(&lower) {
        return Some("double");
    }
    if lower.contains("sizeof(") || lower.contains("strlen(") {
        return Some("size_t");
    }
    if lower.contains("malloc(") || lower.contains("calloc(") || lower.contains("realloc(") {
        return Some("void *");
    }
    for func in ["sqrt(", "pow(", "sin(", "cos(", "tan(", "exp(", "log("] {
        if lower.contains(func) {
            return Some("double");
        }
    }
    if lower == "true" || lower == "false" {
        return Some("bool");
    }
    if ["==", "!=", ">=", "<=", "<", ">", "&&", "||"].iter().any(|op| lower.contains(op)) {
        return Some("int");
    }
    None
}

/// Generate C code from a hint.
fn translate_hint_c(hint: &StatementHint) -> Result<String> {
    match hint {
        StatementHint::Declaration {
            names,
            type_hint,
            qualifiers,
            initial_value,
            is_array,
            array_size,
        } => {
            if names.is_empty() {
                return Err(anyhow!("Declaration requires at least one variable name"));
            }
            
            let inferred = initial_value
                .as_ref()
                .and_then(|v| infer_c_type_from_expression(v))
                .map(|ty| ty as &str);
            let c_type = type_hint.as_deref().or(inferred).unwrap_or("int");
            let qualifier_prefix = if qualifiers.is_empty() {
                String::new()
            } else {
                format!("{} ", qualifiers.join(" "))
            };
            
            if *is_array {
                let size = array_size.as_deref().unwrap_or("10");
                let decls: Vec<String> = names
                    .iter()
                    .map(|name| format!("{}{} {}[{}];", qualifier_prefix, c_type, name, size))
                    .collect();
                Ok(decls.join("\n"))
            } else {
                let declarations: Vec<String> = names
                    .iter()
                    .map(|name| {
                        if let Some(val) = initial_value {
                            format!("{} = {}", name, val)
                        } else {
                            name.clone()
                        }
                    })
                    .collect();
                Ok(format!("{}{} {};", qualifier_prefix, c_type, declarations.join(", ")))
            }
        }

        StatementHint::Assignment { targets, value } => {
            if targets.is_empty() {
                return Err(anyhow!("Assignment requires at least one target"));
            }
            if targets.len() == 1 {
                Ok(format!("{} = {};", targets[0], value))
            } else {
                // Multiple targets with same value
                let assigns: Vec<String> = targets
                    .iter()
                    .map(|t| format!("{} = {}", t, value))
                    .collect();
                Ok(format!("{};", assigns.join(", ")))
            }
        }

        StatementHint::While { condition, body_action } => {
            let body = body_action
                .as_ref()
                .map(|a| format!("    {};", a))
                .unwrap_or_else(|| "    ".to_string());
            Ok(format!("while ({}) {{\n{}\n}}", condition, body))
        }

        StatementHint::InfiniteLoop => {
            Ok("while (1) {\n    \n}".to_string())
        }

        StatementHint::ForEver => {
            Ok("for (;;) {\n    \n}".to_string())
        }

        StatementHint::Goto { label } => Ok(format!("goto {};", label)),
        StatementHint::Label { name } => Ok(format!("{}:", name)),

        StatementHint::Read { variables } => {
            if variables.is_empty() {
                return Err(anyhow!("Read requires at least one variable"));
            }
            let mut lines = Vec::new();
            for var in variables {
                lines.push(format!("int {};", var));
                lines.push(format!("if (scanf(\"%d\", &{}) != 1) {{ fprintf(stderr, \"Invalid input\\n\"); return 1; }}", var));
            }
            Ok(lines.join("\n"))
        }

        StatementHint::FileOpen { var_name, path, mode } => {
            Ok(format!(
                "FILE *{var} = fopen(\"{path}\", \"{mode}\");\nif ({var} == NULL) {{ perror(\"fopen failed\"); return 1; }}",
                var = var_name,
                path = path.trim_matches('"'),
                mode = mode
            ))
        }
        StatementHint::FileClose { var_name } => Ok(format!("fclose({});", var_name)),
        StatementHint::FileRead { var_name, buffer, size } => {
            Ok(format!("fread({}, 1, {}, {});", buffer, size, var_name))
        }
        StatementHint::FileWrite { var_name, buffer, size } => {
            Ok(format!("fwrite({}, 1, {}, {});", buffer, size, var_name))
        }
        StatementHint::Fgets { buffer, size, var_name } => {
            Ok(format!("fgets({}, {}, {});", buffer, size, var_name))
        }
        StatementHint::Fputs { content, var_name } => {
            Ok(format!("fputs(\"{}\", {});", content.trim_matches('"'), var_name))
        }
        StatementHint::Fprintf { var_name, format: fmt, args } => {
            if args.is_empty() {
                Ok(format!("fprintf({}, \"{}\");", var_name, fmt.trim_matches('"')))
            } else {
                Ok(format!("fprintf({}, \"{}\", {});", var_name, fmt.trim_matches('"'), args.join(", ")))
            }
        }
        StatementHint::Fscanf { var_name, args } => {
            let _fmt = "%d".repeat(args.len().max(1));
            let mut fmt_chars = String::new();
            for _ in 0..args.len() {
                fmt_chars.push_str("%d");
            }
            if args.is_empty() {
                Ok(format!("fscanf({}, \"{}\", &value);", var_name, fmt_chars))
            } else {
                let refs: Vec<String> = args.iter().map(|a| format!("&{}", a)).collect();
                Ok(format!("fscanf({}, \"{}\", {});", var_name, fmt_chars, refs.join(", ")))
            }
        }

        StatementHint::Loop {
            iterator,
            start,
            end,
            collection,
            body_action,
        } => {
            let iter = iterator.as_deref().unwrap_or("i");
            
            if let Some(col) = collection {
                // Collection-based loop
                let body = body_action
                    .as_ref()
                    .map(|a| format!("    {};", a))
                    .unwrap_or_else(|| "    ".to_string());
                Ok(format!(
                    "for (int {iter} = 0; {iter} < sizeof({col}) / sizeof({col}[0]); {iter}++) {{\n{body}\n}}",
                    iter = iter,
                    col = col,
                    body = body
                ))
            } else {
                // Range-based loop (inclusive end)
                let start_val = start.as_deref().unwrap_or("0");
                let end_val = end.as_deref().unwrap_or("10");
                let body = body_action
                    .as_ref()
                    .map(|a| format!("    {};", a))
                    .unwrap_or_else(|| "    ".to_string());
                Ok(format!(
                    "for (int {iter} = {start}; {iter} <= {end}; {iter}++) {{\n{body}\n}}",
                    iter = iter,
                    start = start_val,
                    end = end_val,
                    body = body
                ))
            }
        }

        StatementHint::Conditional {
            condition,
            then_action,
            else_action,
            is_else_if,
        } => {
            let keyword = if *is_else_if { "else if" } else { "if" };
            let mut lines = vec![format!("{} ({}) {{", keyword, condition)];
            
            if let Some(action) = then_action {
                // Add empty line for cursor, then the action
                lines.push("    ".to_string());
                let translated = translate_inline_action(action);
                lines.push(format!("    {}", translated));
            } else {
                lines.push("    ".to_string());
            }
            lines.push("}".to_string());
            
            if let Some(else_act) = else_action {
                lines.push("else {".to_string());
                // Add empty line for cursor, then the action
                lines.push("    ".to_string());
                let translated = translate_inline_action(else_act);
                lines.push(format!("    {}", translated));
                lines.push("}".to_string());
            }
            
            Ok(lines.join("\n"))
        }

        StatementHint::Else => Ok("else {\n    \n}".to_string()),

        StatementHint::EndBlock => Ok("}".to_string()),

        StatementHint::Print { content, is_literal } => {
            if *is_literal {
                // For literals, strip any existing quotes and wrap in printf
                let inner = content.trim_matches('"');
                Ok(format!("printf(\"{}\\n\");", inner))
            } else {
                // For variable expressions, use %d for integer-like identifiers
                // This is a simple heuristic - single lowercase identifiers are likely int
                Ok(format!("printf(\"%d\\n\", {});", content))
            }
        }

        StatementHint::Return { value } => {
            if let Some(val) = value {
                Ok(format!("return {};", val))
            } else {
                Ok("return;".to_string())
            }
        }

        StatementHint::Arithmetic {
            operation,
            left,
            right,
            target,
        } => {
            let op_symbol = match operation {
                HintArithmeticOp::Add => "+",
                HintArithmeticOp::Subtract => "-",
                HintArithmeticOp::Multiply => "*",
                HintArithmeticOp::Divide => "/",
                HintArithmeticOp::Modulo => "%",
            };
            
            let expr = format!("{} {} {}", left, op_symbol, right);
            
            if let Some(t) = target {
                Ok(format!("{} = {};", t, expr))
            } else {
                let default_name = match operation {
                    HintArithmeticOp::Add => "sum",
                    HintArithmeticOp::Subtract => "difference",
                    HintArithmeticOp::Multiply => "product",
                    HintArithmeticOp::Divide => "quotient",
                    HintArithmeticOp::Modulo => "remainder",
                };
                Ok(format!("int {} = {};", default_name, expr))
            }
        }

        StatementHint::CompoundAssign { target, operator, value } => {
            let op = match operator {
                crate::hints::CompoundOp::AddAssign => "+=",
                crate::hints::CompoundOp::SubAssign => "-=",
                crate::hints::CompoundOp::MulAssign => "*=",
                crate::hints::CompoundOp::DivAssign => "/=",
                crate::hints::CompoundOp::ModAssign => "%=",
                crate::hints::CompoundOp::ShlAssign => "<<=",
                crate::hints::CompoundOp::ShrAssign => ">>=",
                crate::hints::CompoundOp::AndAssign => "&=",
                crate::hints::CompoundOp::OrAssign => "|=",
                crate::hints::CompoundOp::XorAssign => "^=",
            };
            Ok(format!("{} {} {};", target, op, value))
        }

        StatementHint::Logical { operation, left, right, target } => {
            let op = match operation {
                crate::hints::LogicalOp::And => "&&",
                crate::hints::LogicalOp::Or => "||",
                crate::hints::LogicalOp::Not => "!",
            };
            let expr = if *operation == crate::hints::LogicalOp::Not {
                format!("{}{}", op, left)
            } else {
                format!("{} {} {}", left, op, right.clone().unwrap_or_default())
            };
            if let Some(t) = target {
                Ok(format!("{} = {};", t, expr))
            } else {
                Ok(expr)
            }
        }

        StatementHint::Modify { target, delta } => {
            if *delta > 0 {
                Ok(format!("{}++;", target))
            } else {
                Ok(format!("{}--;", target))
            }
        }

        StatementHint::PrePostModify { target, delta, position } => {
            let op = if *delta > 0 { "++" } else { "--" };
            let expr = match position {
                crate::hints::IncDecPosition::Pre => format!("{}{}", op, target),
                crate::hints::IncDecPosition::Post => format!("{}{}", target, op),
            };
            Ok(format!("{};", expr))
        }

        StatementHint::FunctionDef {
            name,
            parameters,
            return_type,
        } => {
            let ret = return_type.as_deref().unwrap_or("int");
            let params = if parameters.is_empty() {
                "void".to_string()
            } else {
                parameters
                    .iter()
                    .map(|(ty, n)| format!("{} {}", ty, n))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            Ok(format!("{} {}({}) {{\n    \n}}", ret, name, params))
        }

        StatementHint::FunctionPrototype { name, parameters, return_type } => {
            let ret = return_type.as_deref().unwrap_or("int");
            let params = if parameters.is_empty() {
                "void".to_string()
            } else {
                parameters.iter().map(|(ty, n)| format!("{} {}", ty, n)).collect::<Vec<_>>().join(", ")
            };
            Ok(format!("{} {}({});", ret, name, params))
        }

        StatementHint::QualifiedFunction { qualifier, name, parameters, return_type } => {
            let ret = return_type.as_deref().unwrap_or("int");
            let params = if parameters.is_empty() {
                "void".to_string()
            } else {
                parameters.iter().map(|(ty, n)| format!("{} {}", ty, n)).collect::<Vec<_>>().join(", ")
            };
            Ok(format!("{} {} {}({}) {{\n    \n}}", qualifier, ret, name, params))
        }

        StatementHint::StructDef { name, fields } => {
            let body = if fields.is_empty() {
                "    int value;".to_string()
            } else {
                fields
                    .iter()
                    .map(|(ty, n)| format!("    {} {};", ty, n))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            Ok(format!("struct {} {{\n{}\n}};", name, body))
        }

        StatementHint::BitfieldDecl { name, type_hint, width } => {
            let ty = type_hint.as_deref().unwrap_or("unsigned int");
            Ok(format!("{} {} : {};", ty, name, width))
        }

        StatementHint::StructAccess { object, field } => {
            Ok(format!("{}.{}", object, field))
        }

        StatementHint::StructArrow { pointer, field } => {
            Ok(format!("{}->{}", pointer, field))
        }

        StatementHint::StructInit { struct_name, var_name, fields } => {
            if fields.is_empty() {
                Ok(format!("{} {} = {{0}};", struct_name, var_name))
            } else {
                let pairs: Vec<String> = fields.iter().map(|(k, v)| format!(".{} = {}", k, v)).collect();
                Ok(format!("{} {} = {{ {} }};", struct_name, var_name, pairs.join(", ")))
            }
        }

        StatementHint::AnonymousStruct { parent, fields } => {
            let body = if fields.is_empty() {
                "    int value;".to_string()
            } else {
                fields.iter().map(|(ty, n)| format!("    {} {};", ty, n)).collect::<Vec<_>>().join("\n")
            };
            if let Some(parent_name) = parent {
                Ok(format!("struct {} {{\n    struct {{\n{}\n    }};\n}};", parent_name, body))
            } else {
                Ok(format!("struct {{\n{}\n}};", body))
            }
        }

        StatementHint::UnionDef { name, fields } => {
            let body = if fields.is_empty() {
                "    int value;".to_string()
            } else {
                fields.iter().map(|(ty, n)| format!("    {} {};", ty, n)).collect::<Vec<_>>().join("\n")
            };
            Ok(format!("union {} {{\n{}\n}};", name, body))
        }

        StatementHint::AnonymousUnion { parent, fields } => {
            let body = if fields.is_empty() {
                "    int value;".to_string()
            } else {
                fields.iter().map(|(ty, n)| format!("    {} {};", ty, n)).collect::<Vec<_>>().join("\n")
            };
            if let Some(parent_name) = parent {
                Ok(format!("union {} {{\n    union {{\n{}\n    }};\n}};", parent_name, body))
            } else {
                Ok(format!("union {{\n{}\n}};", body))
            }
        }

        StatementHint::StructArray { struct_name, var_name, size } => {
            Ok(format!("{} {}[{}];", struct_name, var_name, size))
        }

        StatementHint::MainFunction => {
            Ok("int main(void) {\n    return 0;\n}".to_string())
        }

        // New hint types
        StatementHint::Switch { expression } => {
            Ok(format!("switch ({}) {{\n    \n}}", expression))
        }

        StatementHint::Case { value, action } => {
            if let Some(act) = action {
                let translated = translate_inline_action(&act);
                // Add empty line for cursor, then the action
                Ok(format!("case {}:\n    \n    {}\n    break;", value, translated))
            } else {
                Ok(format!("case {}:\n    ", value))
            }
        }

        StatementHint::Default { action } => {
            if let Some(act) = action {
                let translated = translate_inline_action(&act);
                // Add empty line for cursor, then the action
                Ok(format!("default:\n    \n    {}\n    break;", translated))
            } else {
                Ok("default:\n    ".to_string())
            }
        }

        StatementHint::DoWhileStart => {
            Ok("do {".to_string())
        }

        StatementHint::DoWhileEnd { condition } => {
            Ok(format!("}} while ({});", condition))
        }

        StatementHint::Break => {
            Ok("break;".to_string())
        }

        StatementHint::Continue => {
            Ok("continue;".to_string())
        }

        StatementHint::FunctionCall { name, arguments } => {
            if arguments.is_empty() {
                Ok(format!("{}();", name))
            } else {
                Ok(format!("{}({});", name, arguments.join(", ")))
            }
        }

        StatementHint::StdLibCall { name, args } => {
            if args.is_empty() {
                Ok(format!("{}();", name))
            } else {
                Ok(format!("{}({});", name, args.join(", ")))
            }
        }

        StatementHint::Include { header, is_system } => {
            if *is_system {
                Ok(format!("#include <{}>", header))
            } else {
                Ok(format!("#include \"{}\"", header))
            }
        }

        StatementHint::Define { name, value } => {
            Ok(format!("#define {} {}", name, value))
        }

        StatementHint::IfDef { symbol } => Ok(format!("#ifdef {}", symbol)),
        StatementHint::IfNDef { symbol } => Ok(format!("#ifndef {}", symbol)),
        StatementHint::EndIf => Ok("#endif".to_string()),
        StatementHint::Undef { symbol } => Ok(format!("#undef {}", symbol)),
        StatementHint::Pragma { value } => Ok(format!("#pragma {}", value)),
        StatementHint::MacroFunction { name, params, body } => {
            let joined = if params.is_empty() { "".to_string() } else { params.join(",") };
            Ok(format!("#define {}({}) {}", name, joined, body))
        }

        StatementHint::PointerDecl { base_type, name } => {
            Ok(format!("{} *{};", base_type, name))
        }

        StatementHint::Dereference { target } => {
            Ok(format!("*{}", target))
        }

        StatementHint::AddressOf { target } => {
            Ok(format!("&{}", target))
        }

        StatementHint::Malloc { count, element_type } => {
            let c_type = match element_type.to_lowercase().as_str() {
                "int" | "integer" => "int",
                "float" => "float",
                "double" => "double",
                "char" => "char",
                _ => "int",
            };
            Ok(format!("malloc({} * sizeof({}))", count, c_type))
        }

        StatementHint::Free { target } => {
            Ok(format!("free({});", target))
        }

        StatementHint::Realloc { pointer, count, element_type } => {
            let c_type = match element_type.to_lowercase().as_str() {
                "int" | "integer" => "int",
                "float" => "float",
                "double" => "double",
                "char" => "char",
                _ => "int",
            };
            Ok(format!("realloc({}, {} * sizeof({}))", pointer, count, c_type))
        }

        StatementHint::Calloc { count, element_type, target } => {
            let c_type = match element_type.to_lowercase().as_str() {
                "int" | "integer" => "int",
                "float" => "float",
                "double" => "double",
                "char" => "char",
                _ => "int",
            };
            let call = format!("calloc({}, sizeof({}))", count, c_type);
            if let Some(t) = target {
                Ok(format!("{} = {};", t, call))
            } else {
                Ok(call)
            }
        }

        StatementHint::PointerArithmetic { pointer, offset, direction } => {
            let sign = match direction {
                crate::hints::PointerDir::Forward => "+",
                crate::hints::PointerDir::Backward => "-",
            };
            Ok(format!("{} {}= {};", pointer, sign, offset))
        }

        StatementHint::FunctionPointer { return_type, name, params } => {
            let params_str = if params.is_empty() { "void".to_string() } else { params.join(", ") };
            Ok(format!("{} (*{})({});", return_type, name, params_str))
        }

        StatementHint::DoublePointer { base_type, name } => {
            Ok(format!("{} **{};", base_type, name))
        }

        StatementHint::NullAssign { target } => {
            Ok(format!("{} = NULL;", target))
        }

        StatementHint::EnumDef { name, values } => {
            if values.is_empty() {
                Ok(format!("enum {} {{\n\n}};", name))
            } else {
                Ok(format!("enum {} {{ {} }};", name, values.join(", ")))
            }
        }

        StatementHint::Typedef { original_type, new_name } => {
            Ok(format!("typedef {} {};", original_type, new_name))
        }

        StatementHint::StringDecl { name, initial_value, size } => {
            if let Some(val) = initial_value {
                Ok(format!("char {}[] = \"{}\";", name, val))
            } else if let Some(sz) = size {
                Ok(format!("char {}[{}];", name, sz))
            } else {
                Ok(format!("char {}[256];", name))
            }
        }

        StatementHint::Strcpy { dest, src } => Ok(format!("strcpy({}, {});", dest, src)),
        StatementHint::Strncpy { dest, src, count } => Ok(format!("strncpy({}, {}, {});", dest, src, count)),
        StatementHint::Strcat { dest, src } => Ok(format!("strcat({}, {});", dest, src)),
        StatementHint::Strcmp { left, right, target } => {
            let expr = format!("strcmp({}, {})", left, right);
            if let Some(t) = target {
                Ok(format!("int {} = {};", t, expr))
            } else {
                Ok(expr)
            }
        }
        StatementHint::Strlen { target, store_in } => {
            let expr = format!("strlen({})", target);
            if let Some(t) = store_in {
                Ok(format!("size_t {} = {};", t, expr))
            } else {
                Ok(expr)
            }
        }
        StatementHint::Sprintf { buffer, format: fmt, args } => {
            if args.is_empty() {
                Ok(format!("sprintf({}, \"{}\");", buffer, fmt.trim_matches('\"')))
            } else {
                Ok(format!("sprintf({}, \"{}\", {});", buffer, fmt.trim_matches('\"'), args.join(", ")))
            }
        }

        StatementHint::Memcpy { dest, src, size } => {
            Ok(format!("memcpy({}, {}, {});", dest, src, size))
        }
        StatementHint::Memset { dest, value, size } => {
            Ok(format!("memset({}, {}, {});", dest, value, size))
        }
        StatementHint::Exit { code } => Ok(format!("exit({});", code)),
        StatementHint::Rand { store_in } => {
            if let Some(t) = store_in {
                Ok(format!("int {} = rand();", t))
            } else {
                Ok("rand();".to_string())
            }
        }
        StatementHint::MathFunc { func, args, store_in } => {
            let fname = match func {
                crate::hints::MathFuncKind::Sqrt => "sqrt",
                crate::hints::MathFuncKind::Pow => "pow",
                crate::hints::MathFuncKind::Abs => "abs",
                crate::hints::MathFuncKind::Sin => "sin",
                crate::hints::MathFuncKind::Cos => "cos",
                crate::hints::MathFuncKind::Tan => "tan",
                crate::hints::MathFuncKind::Exp => "exp",
                crate::hints::MathFuncKind::Log => "log",
            };
            let call = format!("{}({})", fname, args.join(", "));
            if let Some(t) = store_in {
                Ok(format!("double {} = {};", t, call))
            } else {
                Ok(call)
            }
        }

        StatementHint::Assert { expression } => Ok(format!("assert({});", expression)),
        StatementHint::StaticAssert { condition, message } => {
            let msg = message.as_deref().unwrap_or("static assertion failed");
            Ok(format!("_Static_assert({}, \"{}\");", condition, msg))
        }
        StatementHint::CommaExpression { expressions } => {
            Ok(format!("({})", expressions.join(", ")))
        }
        StatementHint::Perror { message } => {
            if let Some(m) = message {
                Ok(format!("perror(\"{}\");", m.trim_matches('\"')))
            } else {
                Ok("perror(NULL);".to_string())
            }
        }
        StatementHint::ErrnoCheck => Ok("if (errno) {\n    perror(\"error\");\n}".to_string()),

        StatementHint::Comment { text, is_block } => {
            if *is_block {
                Ok("/*".to_string())
            } else {
                Ok(format!("// {}", text))
            }
        }

        StatementHint::Bitwise { operation, left, right, target } => {
            let op = match operation {
                crate::hints::BitwiseOp::And => "&",
                crate::hints::BitwiseOp::Or => "|",
                crate::hints::BitwiseOp::Xor => "^",
                crate::hints::BitwiseOp::Not => "~",
                crate::hints::BitwiseOp::ShiftLeft => "<<",
                crate::hints::BitwiseOp::ShiftRight => ">>",
            };
            
            let expr = if let Some(r) = right {
                format!("{} {} {}", left, op, r)
            } else {
                format!("{}{}", op, left)
            };
            
            if let Some(t) = target {
                Ok(format!("{} = {};", t, expr))
            } else {
                Ok(expr)
            }
        }

        StatementHint::Cast { expression, target_type } => {
            Ok(format!("({}){}", target_type, expression))
        }

        StatementHint::ArrayAccess { array, index, value } => {
            let idx = normalize_array_index_c(array, index);
            if let Some(val) = value {
                Ok(format!("{}[{}] = {};", array, idx, val))
            } else {
                Ok(format!("{}[{}]", array, idx))
            }
        }

        StatementHint::MultiArrayDecl { type_hint, name, dimensions } => {
            let ty = type_hint.as_deref().unwrap_or("int");
            let dims = dimensions.iter().map(|d| format!("[{}]", d)).collect::<Vec<_>>().join("");
            Ok(format!("{} {}{};", ty, name, dims))
        }

        StatementHint::ArrayInit { type_hint, name, values } => {
            let ty = type_hint.as_deref().unwrap_or("int");
            Ok(format!("{} {}[] = {{ {} }};", ty, name, values.join(", ")))
        }

        StatementHint::DesignatedInit { type_hint, name, designators } => {
            let ty = type_hint.as_deref().unwrap_or("int");
            let entries: Vec<String> = designators
                .iter()
                .map(|(d, v)| format!("{} = {}", d, v))
                .collect();
            Ok(format!("{} {}[] = {{ {} }};", ty, name, entries.join(", ")))
        }

        StatementHint::CompoundLiteral { type_hint, values, fields, is_array } => {
            if *is_array {
                Ok(format!("({}[]){{ {} }}", type_hint, values.join(", ")))
            } else {
                let pairs: Vec<String> = fields.iter().map(|(k, v)| format!(".{} = {}", k, v)).collect();
                Ok(format!("({}){{ {} }}", type_hint, pairs.join(", ")))
            }
        }

        StatementHint::MultiDimAccess { array, indices, value } => {
            let idx = indices.iter().map(|d| format!("[{}]", d)).collect::<Vec<_>>().join("");
            if let Some(v) = value {
                Ok(format!("{}{} = {};", array, idx, v))
            } else {
                Ok(format!("{}{}", array, idx))
            }
        }

        StatementHint::SizeOf { target } => {
            Ok(format!("sizeof({})", target))
        }

        StatementHint::Ternary { condition, true_value, false_value } => {
            Ok(format!("{} ? {} : {}", condition, true_value, false_value))
        }

        StatementHint::TryBlock => Ok("/* try */".to_string()),
        StatementHint::ExceptBlock { exception_type, variable } => {
            let ex = exception_type.clone().unwrap_or_else(|| "Exception".to_string());
            if let Some(var) = variable {
                Ok(format!("/* catch {} as {} */", ex, var))
            } else {
                Ok(format!("/* catch {} */", ex))
            }
        }
        StatementHint::FinallyBlock => Ok("/* finally */".to_string()),
        StatementHint::RaiseException { exception_type, message } => {
            if let Some(msg) = message {
                Ok(format!("/* raise {}: {} */", exception_type, msg))
            } else {
                Ok(format!("/* raise {} */", exception_type))
            }
        }
        StatementHint::ErrorCheck { function_call, .. } => {
            Ok(format!("/* check error for {} */", function_call))
        }
        StatementHint::SetJmp { buffer } => Ok(format!("setjmp({});", buffer)),
        StatementHint::LongJmp { buffer, value } => Ok(format!("longjmp({}, {});", buffer, value)),

        StatementHint::TestFunction { name, .. } => Ok(format!("void {}(void) {{\n}}", name)),
        StatementHint::TestAssert { expression, .. } => Ok(format!("assert({});", expression)),
        StatementHint::TestAssertEqual { left, right, .. } => Ok(format!("assert({} == {});", left, right)),
        StatementHint::TestAssertNotEqual { left, right } => Ok(format!("assert({} != {});", left, right)),
        StatementHint::TestAssertTrue { expression } => Ok(format!("assert({});", expression)),
        StatementHint::TestAssertFalse { expression } => Ok(format!("assert(!({}));", expression)),
        StatementHint::TestSetup { name } => Ok(format!("void {}(void) {{\n}}", name)),
        StatementHint::TestTeardown { name } => Ok(format!("void {}(void) {{\n}}", name)),
        StatementHint::MockFunction { name, return_value } => Ok(format!("int {}(void) {{ return {}; }}", name, return_value)),

        StatementHint::LabeledBreak { label } => Ok(format!("goto {};", label)),
        StatementHint::LabeledContinue { label } => Ok(format!("goto {};", label)),
        StatementHint::LabeledLoop { label, loop_hint } => {
            let loop_code = translate_hint_c(loop_hint)?;
            Ok(format!("{}:\n{}", label, loop_code))
        }
        StatementHint::MatchBlock { expression } => Ok(format!("switch ({}) {{\n}}", expression)),
        StatementHint::MatchCase { pattern, guard, action } => {
            let guard_comment = guard.as_ref().map(|g| format!(" /* if {} */", g)).unwrap_or_default();
            let act = action.as_ref().map(|a| format!(" {};", a)).unwrap_or_else(|| " break;".to_string());
            Ok(format!("case {}:{}{}", pattern, guard_comment, act))
        }
        StatementHint::MatchWildcard { action } => {
            let act = action.as_ref().map(|a| format!(" {};", a)).unwrap_or_else(|| " break;".to_string());
            Ok(format!("default:{}", act))
        }
        StatementHint::GuardClause { condition, action } => {
            Ok(format!("if (!({})) {{ {}{} }}", condition, action, if action.ends_with(';') { "" } else { ";" }))
        }
        StatementHint::ConditionalChain { conditions, else_action } => {
            if conditions.is_empty() {
                return Err(anyhow!("Conditional chain requires conditions"));
            }
            let mut lines = Vec::new();
            for (i, (cond, act)) in conditions.iter().enumerate() {
                let keyword = if i == 0 { "if" } else { "else if" };
                lines.push(format!("{} ({}) {{ {}{} }}", keyword, cond, act, if act.ends_with(';') { "" } else { ";" }));
            }
            if let Some(act) = else_action {
                lines.push(format!("else {{ {}{} }}", act, if act.ends_with(';') { "" } else { ";" }));
            }
            Ok(lines.join(" "))
        }

        StatementHint::LinkedListCreate { name, .. } => Ok(format!("LinkedList *{} = linked_list_create();", name)),
        StatementHint::LinkedListInsert { list, value, position } => {
            if let Some(pos) = position {
                Ok(format!("linked_list_insert_at({}, {}, {});", list, value, pos))
            } else {
                Ok(format!("linked_list_insert({}, {});", list, value))
            }
        }
        StatementHint::LinkedListRemove { list, position } => Ok(format!("linked_list_remove({}, {});", list, position)),
        StatementHint::LinkedListTraverse { list, iterator } => Ok(format!("/* traverse {} with {} */", list, iterator)),
        StatementHint::StackCreate { name, .. } => Ok(format!("Stack *{} = stack_create();", name)),
        StatementHint::StackPush { stack, value } => Ok(format!("stack_push({}, {});", stack, value)),
        StatementHint::StackPop { stack, target } => {
            if let Some(t) = target {
                Ok(format!("{} = stack_pop({});", t, stack))
            } else {
                Ok(format!("stack_pop({});", stack))
            }
        }
        StatementHint::StackPeek { stack, target } => {
            if let Some(t) = target {
                Ok(format!("{} = stack_peek({});", t, stack))
            } else {
                Ok(format!("stack_peek({});", stack))
            }
        }
        StatementHint::StackIsEmpty { stack } => Ok(format!("stack_is_empty({})", stack)),
        StatementHint::QueueCreate { name, .. } => Ok(format!("Queue *{} = queue_create();", name)),
        StatementHint::QueueEnqueue { queue, value } => Ok(format!("queue_enqueue({}, {});", queue, value)),
        StatementHint::QueueDequeue { queue, target } => {
            if let Some(t) = target {
                Ok(format!("{} = queue_dequeue({});", t, queue))
            } else {
                Ok(format!("queue_dequeue({});", queue))
            }
        }
        StatementHint::MapCreate { name, .. } => Ok(format!("Map *{} = map_create();", name)),
        StatementHint::MapPut { map, key, value } => Ok(format!("map_put({}, {}, {});", map, key, value)),
        StatementHint::MapGet { map, key, target } => {
            if let Some(t) = target {
                Ok(format!("{} = map_get({}, {});", t, map, key))
            } else {
                Ok(format!("map_get({}, {});", map, key))
            }
        }
        StatementHint::MapRemove { map, key } => Ok(format!("map_remove({}, {});", map, key)),
        StatementHint::MapContainsKey { map, key } => Ok(format!("map_contains({}, {})", map, key)),
        StatementHint::MapKeys { map, target } => {
            if let Some(t) = target {
                Ok(format!("{} = map_keys({});", t, map))
            } else {
                Ok(format!("map_keys({});", map))
            }
        }
        StatementHint::MapValues { map, target } => {
            if let Some(t) = target {
                Ok(format!("{} = map_values({});", t, map))
            } else {
                Ok(format!("map_values({});", map))
            }
        }
        StatementHint::SetCreate { name, .. } => Ok(format!("Set *{} = set_create();", name)),
        StatementHint::SetAdd { set, value } => Ok(format!("set_add({}, {});", set, value)),
        StatementHint::SetRemove { set, value } => Ok(format!("set_remove({}, {});", set, value)),
        StatementHint::SetContains { set, value } => Ok(format!("set_contains({}, {})", set, value)),
        StatementHint::SetUnion { set1, set2, target } => Ok(format!("Set *{} = set_union({}, {});", target, set1, set2)),
        StatementHint::SetIntersection { set1, set2, target } => Ok(format!("Set *{} = set_intersection({}, {});", target, set1, set2)),
        StatementHint::TreeNode { name, value, left, right } => {
            let l = left.clone().unwrap_or_else(|| "NULL".to_string());
            let r = right.clone().unwrap_or_else(|| "NULL".to_string());
            Ok(format!("TreeNode {} = {{ {}, {}, {} }};", name, value, l, r))
        }
        StatementHint::TreeInsert { tree, value } => Ok(format!("tree_insert({}, {});", tree, value)),
        StatementHint::TreeSearch { tree, value } => Ok(format!("tree_search({}, {});", tree, value)),
        StatementHint::TreeTraverse { tree, order } => Ok(format!("tree_traverse({}, \"{}\");", tree, order)),

        StatementHint::ClassDef { name, parent, interfaces, .. } => {
            let mut comment = String::new();
            if let Some(p) = parent {
                comment.push_str(&format!(" /* extends {} */", p));
            }
            if !interfaces.is_empty() {
                comment.push_str(&format!(" /* implements {} */", interfaces.join(", ")));
            }
            Ok(format!("struct {} {{ }};{}", name, comment))
        }
        StatementHint::ClassField { name, type_hint, .. } => {
            let ty = type_hint.as_deref().unwrap_or("int");
            Ok(format!("{} {};", ty, name))
        }
        StatementHint::ClassMethod { name, parameters, return_type, .. } => {
            let ret = return_type.as_deref().unwrap_or("void");
            let params = if parameters.is_empty() {
                "void".to_string()
            } else {
                parameters.iter().map(|(t, n)| format!("{} {}", t, n)).collect::<Vec<_>>().join(", ")
            };
            Ok(format!("{} {}({});", ret, name, params))
        }
        StatementHint::Constructor { .. } => Ok("/* constructor */".to_string()),
        StatementHint::Destructor { .. } => Ok("/* destructor */".to_string()),
        StatementHint::InterfaceDef { name, .. } => Ok(format!("/* interface {} */", name)),
        StatementHint::InterfaceMethod { name, parameters, return_type } => {
            let ret = return_type.as_deref().unwrap_or("void");
            let params = if parameters.is_empty() {
                "void".to_string()
            } else {
                parameters.iter().map(|(t, n)| format!("{} {}", t, n)).collect::<Vec<_>>().join(", ")
            };
            Ok(format!("{} {}({});", ret, name, params))
        }
        StatementHint::ObjectCreate { class_name, variable, .. } => {
            Ok(format!("{} *{} = malloc(sizeof({}));", class_name, variable, class_name))
        }
        StatementHint::MethodCall { object, method, arguments } => {
            if arguments.is_empty() {
                Ok(format!("{}(&{});", method, object))
            } else {
                Ok(format!("{}(&{}, {});", method, object, arguments.join(", ")))
            }
        }
        StatementHint::PropertyAccess { object, property } => Ok(format!("{}.{}", object, property)),
        StatementHint::PropertyAssign { object, property, value } => Ok(format!("{}.{} = {};", object, property, value)),
        StatementHint::SuperCall { .. } => Ok("/* super call */".to_string()),
        StatementHint::ThisReference => Ok("this".to_string()),

        StatementHint::Lambda { .. } => Ok("/* lambda */".to_string()),
        StatementHint::HigherOrderFunction { function, callback, collection } => {
            if let Some(col) = collection {
                Ok(format!("{}({}, {});", function, col, callback))
            } else {
                Ok(format!("{}({});", function, callback))
            }
        }
        StatementHint::MapFunction { collection, transform, target } => {
            let expr = format!("map({}, {})", transform, collection);
            if let Some(t) = target {
                Ok(format!("{} = {};", t, expr))
            } else {
                Ok(expr)
            }
        }
        StatementHint::FilterFunction { collection, predicate, target } => {
            let expr = format!("filter({}, {})", predicate, collection);
            if let Some(t) = target {
                Ok(format!("{} = {};", t, expr))
            } else {
                Ok(expr)
            }
        }
        StatementHint::ReduceFunction { collection, reducer, initial, target } => {
            let expr = if let Some(init) = initial {
                format!("reduce({}, {}, {})", reducer, collection, init)
            } else {
                format!("reduce({}, {})", reducer, collection)
            };
            if let Some(t) = target {
                Ok(format!("{} = {};", t, expr))
            } else {
                Ok(expr)
            }
        }
        StatementHint::ForEachFunction { collection, action } => {
            Ok(format!("for_each({}, {});", collection, action))
        }

        StatementHint::ThreadCreate { name, function, arguments } => {
            let thread_name = name.clone().unwrap_or_else(|| "thread".to_string());
            if arguments.is_empty() {
                Ok(format!("pthread_t {}; pthread_create(&{}, NULL, {}, NULL);", thread_name, thread_name, function))
            } else {
                Ok(format!("pthread_t {}; pthread_create(&{}, NULL, {}, {});", thread_name, thread_name, function, arguments.join(", ")))
            }
        }
        StatementHint::ThreadJoin { thread } => Ok(format!("pthread_join({}, NULL);", thread)),
        StatementHint::ThreadDetach { thread } => Ok(format!("pthread_detach({});", thread)),
        StatementHint::ThreadSleep { duration, .. } => Ok(format!("sleep({});", duration)),
        StatementHint::MutexCreate { name } => Ok(format!("pthread_mutex_t {}; pthread_mutex_init(&{}, NULL);", name, name)),
        StatementHint::MutexLock { mutex } => Ok(format!("pthread_mutex_lock(&{});", mutex)),
        StatementHint::MutexUnlock { mutex } => Ok(format!("pthread_mutex_unlock(&{});", mutex)),
        StatementHint::MutexTryLock { mutex } => Ok(format!("pthread_mutex_trylock(&{});", mutex)),
        StatementHint::SemaphoreCreate { name, initial } => Ok(format!("sem_t {}; sem_init(&{}, 0, {});", name, name, initial)),
        StatementHint::SemaphoreWait { semaphore } => Ok(format!("sem_wait(&{});", semaphore)),
        StatementHint::SemaphoreSignal { semaphore } => Ok(format!("sem_post(&{});", semaphore)),
        StatementHint::ConditionCreate { name } => Ok(format!("pthread_cond_t {}; pthread_cond_init(&{}, NULL);", name, name)),
        StatementHint::ConditionWait { condition, mutex } => Ok(format!("pthread_cond_wait(&{}, &{});", condition, mutex)),
        StatementHint::ConditionSignal { condition } => Ok(format!("pthread_cond_signal(&{});", condition)),
        StatementHint::ConditionBroadcast { condition } => Ok(format!("pthread_cond_broadcast(&{});", condition)),
        StatementHint::AtomicCreate { name, initial } => Ok(format!("atomic_int {} = {};", name, initial)),
        StatementHint::AtomicLoad { atomic, target } => {
            if let Some(t) = target {
                Ok(format!("{} = atomic_load(&{});", t, atomic))
            } else {
                Ok(format!("atomic_load(&{});", atomic))
            }
        }
        StatementHint::AtomicStore { atomic, value } => Ok(format!("atomic_store(&{}, {});", atomic, value)),
        StatementHint::AtomicCompareExchange { atomic, expected, desired } => Ok(format!("atomic_compare_exchange_strong(&{}, &{}, {});", atomic, expected, desired)),
        StatementHint::AtomicIncrement { atomic } => Ok(format!("atomic_fetch_add(&{}, 1);", atomic)),
        StatementHint::AtomicDecrement { atomic } => Ok(format!("atomic_fetch_sub(&{}, 1);", atomic)),

        StatementHint::AsyncFunction { name, parameters, return_type } => {
            let ret = return_type.as_deref().unwrap_or("void");
            let params = if parameters.is_empty() {
                "void".to_string()
            } else {
                parameters.iter().map(|(t, n)| format!("{} {}", t, n)).collect::<Vec<_>>().join(", ")
            };
            Ok(format!("/* async */ {} {}({});", ret, name, params))
        }
        StatementHint::AwaitExpression { expression, target } => {
            if let Some(t) = target {
                Ok(format!("{} = /* await */ {};", t, expression))
            } else {
                Ok(format!("/* await */ {};", expression))
            }
        }
        StatementHint::PromiseCreate { name, executor } => Ok(format!("/* promise {} with {} */", name, executor)),
        StatementHint::PromiseThen { promise, handler } => Ok(format!("/* {} then {} */", promise, handler)),
        StatementHint::PromiseCatch { promise, handler } => Ok(format!("/* {} catch {} */", promise, handler)),
        StatementHint::PromiseAll { promises, target } => Ok(format!("/* promise all {:?} into {} */", promises, target)),
        StatementHint::PromiseRace { promises, target } => Ok(format!("/* promise race {:?} into {} */", promises, target)),

        StatementHint::DocComment { text, .. } => Ok(format!("/** {} */", text)),
        StatementHint::DocFunction { brief, .. } => Ok(format!("/** {} */", brief)),
        StatementHint::DocClass { brief, .. } => Ok(format!("/** {} */", brief)),

        StatementHint::Unknown { original } => {
            Err(anyhow!("UNHANDLED: {}", original))
        }
    }
}

/// Generate Python code from a hint.
fn translate_hint_python(hint: &StatementHint) -> Result<String> {
    match hint {
        StatementHint::Declaration {
            names,
            type_hint: _,
            qualifiers: _,
            initial_value,
            is_array,
            array_size,
        } => {
            if names.is_empty() {
                return Err(anyhow!("Declaration requires at least one variable name"));
            }
            
            if *is_array {
                let size = array_size.as_deref().unwrap_or("10");
                let decls: Vec<String> = names
                    .iter()
                    .map(|name| format!("{} = [0] * {}", name, size))
                    .collect();
                Ok(decls.join("\n"))
            } else {
                let val = initial_value.as_deref().unwrap_or("None");
                if names.len() == 1 {
                    Ok(format!("{} = {}", names[0], val))
                } else {
                    Ok(format!("{} = {}", names.join(" = "), val))
                }
            }
        }

        StatementHint::Assignment { targets, value } => {
            if targets.is_empty() {
                return Err(anyhow!("Assignment requires at least one target"));
            }
            if targets.len() == 1 {
                Ok(format!("{} = {}", targets[0], value))
            } else {
                // Python multiple assignment: a = b = c = value
                Ok(format!("{} = {}", targets.join(" = "), value))
            }
        }

        StatementHint::While { condition, body_action } => {
            let body = body_action
                .as_ref()
                .map(|a| format!("    {}", a))
                .unwrap_or_else(|| "    pass".to_string());
            Ok(format!("while {}:\n{}", condition, body))
        }

        StatementHint::Read { variables } => {
            if variables.is_empty() {
                return Err(anyhow!("Read requires at least one variable"));
            }
            let lines: Vec<String> = variables
                .iter()
                .map(|var| format!("{} = int(input())", var))
                .collect();
            Ok(lines.join("\n"))
        }

        StatementHint::Loop {
            iterator,
            start,
            end,
            collection,
            body_action,
        } => {
            let iter = iterator.as_deref().unwrap_or("i");
            
            if let Some(col) = collection {
                let body = body_action
                    .as_ref()
                    .map(|a| format!("    {}", a))
                    .unwrap_or_else(|| "    pass".to_string());
                Ok(format!("for {} in {}:\n{}", iter, col, body))
            } else {
                let start_val = start.as_deref().unwrap_or("0");
                let end_val = end.as_deref().unwrap_or("10");
                let body = body_action
                    .as_ref()
                    .map(|a| format!("    {}", a))
                    .unwrap_or_else(|| "    pass".to_string());
                Ok(format!("for {} in range({}, {}):\n{}", iter, start_val, end_val, body))
            }
        }

        StatementHint::Conditional {
            condition,
            then_action,
            else_action,
            is_else_if,
        } => {
            let keyword = if *is_else_if { "elif" } else { "if" };
            let mut lines = vec![format!("{} {}:", keyword, condition)];
            
            if let Some(action) = then_action {
                lines.push(format!("    {}", action));
            } else {
                lines.push("    pass".to_string());
            }
            
            if let Some(else_act) = else_action {
                lines.push("else:".to_string());
                lines.push(format!("    {}", else_act));
            }
            
            Ok(lines.join("\n"))
        }

        StatementHint::Else => Ok("else:\n    pass".to_string()),

        StatementHint::EndBlock => Ok(String::new()), // Python doesn't use closing braces

        StatementHint::Print { content, is_literal } => {
            if *is_literal {
                // For literals, ensure content is quoted for Python
                if content.starts_with('"') || content.starts_with('\'') {
                    Ok(format!("print({})", content))
                } else {
                    Ok(format!("print(\"{}\")", content))
                }
            } else {
                // For variables, just print the identifier
                Ok(format!("print({})", content))
            }
        }

        StatementHint::Return { value } => {
            if let Some(val) = value {
                Ok(format!("return {}", val))
            } else {
                Ok("return".to_string())
            }
        }

        StatementHint::Arithmetic {
            operation,
            left,
            right,
            target,
        } => {
            let op_symbol = match operation {
                HintArithmeticOp::Add => "+",
                HintArithmeticOp::Subtract => "-",
                HintArithmeticOp::Multiply => "*",
                HintArithmeticOp::Divide => "/",
                HintArithmeticOp::Modulo => "%",
            };
            
            let expr = format!("{} {} {}", left, op_symbol, right);
            
            if let Some(t) = target {
                Ok(format!("{} = {}", t, expr))
            } else {
                let default_name = match operation {
                    HintArithmeticOp::Add => "sum_result",
                    HintArithmeticOp::Subtract => "difference_result",
                    HintArithmeticOp::Multiply => "product_result",
                    HintArithmeticOp::Divide => "quotient_result",
                    HintArithmeticOp::Modulo => "remainder_result",
                };
                Ok(format!("{} = {}", default_name, expr))
            }
        }

        StatementHint::Modify { target, delta } => {
            if *delta > 0 {
                Ok(format!("{} += 1", target))
            } else {
                Ok(format!("{} -= 1", target))
            }
        }
        StatementHint::PrePostModify { target, delta, .. } => {
            if *delta > 0 {
                Ok(format!("{} += 1  # pre/post increment", target))
            } else {
                Ok(format!("{} -= 1  # pre/post decrement", target))
            }
        }

        StatementHint::FunctionDef {
            name,
            parameters,
            return_type: _,
        } => {
            let params = if parameters.is_empty() {
                String::new()
            } else {
                parameters
                    .iter()
                    .map(|(_, n)| n.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            Ok(format!("def {}({}):\n    pass", name, params))
        }

        StatementHint::StructDef { name, fields } => {
            // Python uses classes for struct-like behavior
            let mut lines = vec![format!("class {}:", name)];
            lines.push("    def __init__(self):".to_string());
            if fields.is_empty() {
                lines.push("        self.value = None".to_string());
            } else {
                for (_, field_name) in fields {
                    lines.push(format!("        self.{} = None", field_name));
                }
            }
            Ok(lines.join("\n"))
        }
        StatementHint::BitfieldDecl { name, type_hint, width } => {
            let ty = type_hint.as_deref().unwrap_or("int");
            Ok(format!("# bitfield {} : {} ({})", name, width, ty))
        }
        StatementHint::AnonymousStruct { parent, fields } => {
            let mut lines = Vec::new();
            if let Some(p) = parent {
                lines.push(format!("# anonymous struct inside {}", p));
            } else {
                lines.push("# anonymous struct".to_string());
            }
            for (_, field_name) in fields {
                lines.push(format!("# field {}", field_name));
            }
            Ok(lines.join("\n"))
        }
        StatementHint::AnonymousUnion { parent, fields } => {
            let mut lines = Vec::new();
            if let Some(p) = parent {
                lines.push(format!("# anonymous union inside {}", p));
            } else {
                lines.push("# anonymous union".to_string());
            }
            for (_, field_name) in fields {
                lines.push(format!("# field {}", field_name));
            }
            Ok(lines.join("\n"))
        }

        StatementHint::MainFunction => {
            Ok("def main():\n    pass\n\nif __name__ == \"__main__\":\n    main()".to_string())
        }

        // New hint types for Python
        StatementHint::Switch { expression } => {
            // Python 3.10+ has match/case
            Ok(format!("match {}:\n    case _:\n        pass", expression))
        }

        StatementHint::Case { value, action } => {
            if let Some(act) = action {
                Ok(format!("case {}:\n    {}", value, act))
            } else {
                Ok(format!("case {}:\n    pass", value))
            }
        }

        StatementHint::Default { action } => {
            if let Some(act) = action {
                Ok(format!("case _:\n    {}", act))
            } else {
                Ok("case _:\n    pass".to_string())
            }
        }

        StatementHint::DoWhileStart => {
            Ok("while True:  # do-while start".to_string())
        }

        StatementHint::DoWhileEnd { condition } => {
            Ok(format!("    if not ({}):\n        break", condition))
        }

        StatementHint::Break => {
            Ok("break".to_string())
        }

        StatementHint::Continue => {
            Ok("continue".to_string())
        }

        StatementHint::FunctionCall { name, arguments } => {
            if arguments.is_empty() {
                Ok(format!("{}()", name))
            } else {
                Ok(format!("{}({})", name, arguments.join(", ")))
            }
        }

        StatementHint::Include { header, is_system: _ } => {
            // Map C headers to Python imports
            let py_import = match header.trim_end_matches(".h").to_lowercase().as_str() {
                "stdio" => "import sys",
                "stdlib" => "import os",
                "string" => "# string operations built-in",
                "math" => "import math",
                "time" => "import time",
                _ => &format!("# include {} (no Python equivalent)", header),
            };
            Ok(py_import.to_string())
        }

        StatementHint::Define { name, value } => {
            Ok(format!("{} = {}", name, value))
        }

        StatementHint::PointerDecl { base_type: _, name } => {
            // Python doesn't have pointers
            Ok(format!("{} = None  # pointer", name))
        }

        StatementHint::Dereference { target } => {
            Ok(target.to_string())  // Python doesn't dereference
        }

        StatementHint::AddressOf { target } => {
            Ok(format!("id({})", target))
        }

        StatementHint::Malloc { count, element_type: _ } => {
            Ok(format!("[None] * {}", count))
        }

        StatementHint::Free { target: _ } => {
            Ok("# memory managed automatically".to_string())
        }

        StatementHint::EnumDef { name, values } => {
            let mut lines = vec![
                "from enum import Enum".to_string(),
                format!("class {}(Enum):", name),
            ];
            for (i, val) in values.iter().enumerate() {
                lines.push(format!("    {} = {}", val, i));
            }
            if values.is_empty() {
                lines.push("    pass".to_string());
            }
            Ok(lines.join("\n"))
        }

        StatementHint::Typedef { original_type: _, new_name } => {
            // Python uses duck typing, typedef is essentially an alias
            Ok(format!("{} = type  # typedef alias", new_name))
        }

        StatementHint::StringDecl { name, initial_value, size: _ } => {
            if let Some(val) = initial_value {
                Ok(format!("{} = \"{}\"", name, val))
            } else {
                Ok(format!("{} = \"\"", name))
            }
        }

        StatementHint::Comment { text, is_block } => {
            if *is_block {
                Ok("\"\"\"".to_string())
            } else {
                Ok(format!("# {}", text))
            }
        }

        StatementHint::Bitwise { operation, left, right, target } => {
            let op = match operation {
                crate::hints::BitwiseOp::And => "&",
                crate::hints::BitwiseOp::Or => "|",
                crate::hints::BitwiseOp::Xor => "^",
                crate::hints::BitwiseOp::Not => "~",
                crate::hints::BitwiseOp::ShiftLeft => "<<",
                crate::hints::BitwiseOp::ShiftRight => ">>",
            };
            
            let expr = if let Some(r) = right {
                format!("{} {} {}", left, op, r)
            } else {
                format!("{}{}", op, left)
            };
            
            if let Some(t) = target {
                Ok(format!("{} = {}", t, expr))
            } else {
                Ok(expr)
            }
        }

        StatementHint::Cast { expression, target_type } => {
            let py_type = match target_type.to_lowercase().as_str() {
                "int" | "integer" => "int",
                "float" | "double" => "float",
                "char" | "string" => "str",
                "bool" | "boolean" => "bool",
                _ => target_type,
            };
            Ok(format!("{}({})", py_type, expression))
        }

        StatementHint::ArrayAccess { array, index, value } => {
            let idx = normalize_array_index_python(array, index);
            if let Some(val) = value {
                Ok(format!("{}[{}] = {}", array, idx, val))
            } else {
                Ok(format!("{}[{}]", array, idx))
            }
        }

        StatementHint::SizeOf { target } => {
            Ok(format!("len({})", target))
        }

        StatementHint::Ternary { condition, true_value, false_value } => {
            Ok(format!("{} if {} else {}", true_value, condition, false_value))
        }

        StatementHint::CompoundAssign { target, operator, value } => {
            let op = match operator {
                crate::hints::CompoundOp::AddAssign => "+=",
                crate::hints::CompoundOp::SubAssign => "-=",
                crate::hints::CompoundOp::MulAssign => "*=",
                crate::hints::CompoundOp::DivAssign => "/=",
                crate::hints::CompoundOp::ModAssign => "%=",
                crate::hints::CompoundOp::ShlAssign => "<<=",
                crate::hints::CompoundOp::ShrAssign => ">>=",
                crate::hints::CompoundOp::AndAssign => "&=",
                crate::hints::CompoundOp::OrAssign => "|=",
                crate::hints::CompoundOp::XorAssign => "^=",
            };
            Ok(format!("{} {} {}", target, op, value))
        }

        StatementHint::Logical { operation, left, right, target } => {
            let op = match operation {
                crate::hints::LogicalOp::And => "and",
                crate::hints::LogicalOp::Or => "or",
                crate::hints::LogicalOp::Not => "not",
            };
            let expr = if *operation == crate::hints::LogicalOp::Not {
                format!("{} {}", op, left)
            } else {
                format!("{} {} {}", left, op, right.clone().unwrap_or_default())
            };
            if let Some(t) = target {
                Ok(format!("{} = {}", t, expr))
            } else {
                Ok(expr)
            }
        }

        StatementHint::IfDef { symbol } => Ok(format!("#ifdef {}", symbol)),
        StatementHint::IfNDef { symbol } => Ok(format!("#ifndef {}", symbol)),
        StatementHint::EndIf => Ok("#endif".to_string()),
        StatementHint::Undef { symbol } => Ok(format!("#undef {}", symbol)),
        StatementHint::Pragma { value } => Ok(format!("#pragma {}", value)),
        StatementHint::MacroFunction { name, params, body } => {
            Ok(format!("#define {}({}) {}", name, params.join(","), body))
        }

        StatementHint::Realloc { pointer, count, element_type: _ } => {
            Ok(format!("{0} = [None] * {1}", pointer, count))
        }
        StatementHint::Calloc { count, element_type: _, target } => {
            let expr = format!("[0] * {}", count);
            if let Some(t) = target {
                Ok(format!("{} = {}", t, expr))
            } else {
                Ok(expr)
            }
        }
        StatementHint::PointerArithmetic { pointer, offset, direction } => {
            let sign = match direction {
                crate::hints::PointerDir::Forward => "+=",
                crate::hints::PointerDir::Backward => "-=",
            };
            Ok(format!("{} {} {}", pointer, sign, offset))
        }
        StatementHint::FunctionPointer { return_type: _, name, params: _ } => {
            Ok(format!("{} = None  # function pointer", name))
        }
        StatementHint::DoublePointer { base_type: _, name } => {
            Ok(format!("{} = None  # double pointer", name))
        }
        StatementHint::NullAssign { target } => Ok(format!("{} = None", target)),

        StatementHint::MultiArrayDecl { type_hint: _, name, dimensions } => {
            let total = dimensions.join(" * ");
            Ok(format!("{name} = [0] * ({})", total, name=name))
        }
        StatementHint::ArrayInit { type_hint: _, name, values } => {
            Ok(format!("{} = [{}]", name, values.join(", ")))
        }
        StatementHint::DesignatedInit { type_hint: _, name, designators } => {
            let items: Vec<String> = designators
                .iter()
                .map(|(d, v)| format!("{}: {}", d.trim_matches(&['[', ']'][..]), v))
                .collect();
            Ok(format!("{} = {{ {} }}  # designated init", name, items.join(", ")))
        }
        StatementHint::CompoundLiteral { type_hint, values, fields, is_array } => {
            if *is_array {
                Ok(format!("[{}]  # compound literal {}", values.join(", "), type_hint))
            } else {
                let pairs: Vec<String> = fields.iter().map(|(k, v)| format!("\"{}\": {}", k, v)).collect();
                Ok(format!("{{ {} }}  # compound literal {}", pairs.join(", "), type_hint))
            }
        }
        StatementHint::MultiDimAccess { array, indices, value } => {
            let idx = indices.join("][");
            if let Some(v) = value {
                Ok(format!("{}[{}] = {}", array, idx, v))
            } else {
                Ok(format!("{}[{}]", array, idx))
            }
        }

        StatementHint::StructAccess { object, field } => Ok(format!("{}.{}", object, field)),
        StatementHint::StructArrow { pointer, field } => Ok(format!("{}.{}", pointer, field)),
        StatementHint::StructInit { struct_name: _, var_name, fields } => {
            let mut assigns = Vec::new();
            for (k, v) in fields {
                assigns.push(format!("'{}': {}", k, v));
            }
            Ok(format!("{name} = {{{fields}}}", name=var_name, fields=assigns.join(", ")))
        }
        StatementHint::UnionDef { name, fields } => {
            let body: Vec<String> = fields.iter().map(|(t,n)| format!("{}: None # {}", n, t)).collect();
            Ok(format!("{} = {{ {} }}", name, body.join(", ")))
        }
        StatementHint::StructArray { struct_name: _, var_name, size } => {
            Ok(format!("{} = [None] * {}", var_name, size))
        }

        StatementHint::Goto { label } => Ok(format!("# goto {}", label)),
        StatementHint::Label { name } => Ok(format!("# label {}", name)),
        StatementHint::InfiniteLoop => Ok("while True:\n    pass".to_string()),
        StatementHint::ForEver => Ok("while True:\n    pass".to_string()),

        StatementHint::FunctionPrototype { name, parameters, return_type: _ } => {
            let params = parameters.iter().map(|(_, n)| n.clone()).collect::<Vec<_>>().join(", ");
            Ok(format!("def {}({}):\n    pass", name, params))
        }
        StatementHint::QualifiedFunction { qualifier: _, name, parameters, return_type: _ } => {
            let params = parameters.iter().map(|(_, n)| n.clone()).collect::<Vec<_>>().join(", ");
            Ok(format!("def {}({}):\n    pass", name, params))
        }

        StatementHint::FileOpen { var_name, path, mode } => {
            Ok(format!("{} = open(\"{}\", \"{}\")", var_name, path.trim_matches('\"'), mode))
        }
        StatementHint::FileClose { var_name } => Ok(format!("{}.close()", var_name)),
        StatementHint::FileRead { var_name, buffer, size } => {
            Ok(format!("{} = {}.read({})", buffer, var_name, size))
        }
        StatementHint::FileWrite { var_name, buffer, size: _ } => {
            Ok(format!("{}.write({})", var_name, buffer))
        }
        StatementHint::Fgets { buffer, size: _, var_name } => {
            Ok(format!("{} = {}.readline()", buffer, var_name))
        }
        StatementHint::Fputs { content, var_name } => {
            Ok(format!("{}.write(\"{}\")", var_name, content.trim_matches('\"')))
        }
        StatementHint::Fprintf { var_name, format: fmt, args } => {
            if args.is_empty() {
                Ok(format!("{}.write(f\"{}\")", var_name, fmt.trim_matches('\"')))
            } else {
                Ok(format!("{}.write(f\"{}\")", var_name, fmt.trim_matches('\"')))
            }
        }
        StatementHint::Fscanf { var_name: _, args: _ } => Ok("# fscanf not directly supported".to_string()),

        StatementHint::Strcpy { dest, src } => Ok(format!("{} = str({})", dest, src)),
        StatementHint::Strncpy { dest, src, count } => Ok(format!("{} = str({})[:{}]", dest, src, count)),
        StatementHint::Strcat { dest, src } => Ok(format!("{} = str({}) + str({})", dest, dest, src)),
        StatementHint::Strcmp { left, right, target } => {
            let expr = format!("({} > {}) - ({} < {})", left, right, left, right);
            if let Some(t) = target {
                Ok(format!("{} = {}", t, expr))
            } else {
                Ok(expr)
            }
        }
        StatementHint::Strlen { target, store_in } => {
            if let Some(t) = store_in {
                Ok(format!("{} = len({})", t, target))
            } else {
                Ok(format!("len({})", target))
            }
        }
        StatementHint::Sprintf { buffer, format: fmt, args: _ } => {
            Ok(format!("{} = f\"{}\"", buffer, fmt.trim_matches('\"')))
        }

        StatementHint::Memcpy { dest, src, size: _ } => Ok(format!("{} = {}[:] ", dest, src)),
        StatementHint::Memset { dest, value, size } => Ok(format!("{} = [{}] * {}", dest, value, size)),
        StatementHint::Exit { code } => Ok(format!("raise SystemExit({})", code)),
        StatementHint::Rand { store_in } => {
            if let Some(t) = store_in {
                Ok(format!("import random\n{} = random.randint(0, 2147483647)", t))
            } else {
                Ok("import random\nrandom.randint(0, 2147483647)".to_string())
            }
        }
        StatementHint::MathFunc { func, args, store_in } => {
            let fname = match func {
                crate::hints::MathFuncKind::Sqrt => "math.sqrt",
                crate::hints::MathFuncKind::Pow => "math.pow",
                crate::hints::MathFuncKind::Abs => "abs",
                crate::hints::MathFuncKind::Sin => "math.sin",
                crate::hints::MathFuncKind::Cos => "math.cos",
                crate::hints::MathFuncKind::Tan => "math.tan",
                crate::hints::MathFuncKind::Exp => "math.exp",
                crate::hints::MathFuncKind::Log => "math.log",
            };
            let call = format!("{}({})", fname, args.join(", "));
            if let Some(t) = store_in {
                Ok(format!("import math\n{} = {}", t, call))
            } else {
                Ok(format!("import math\n{}", call))
            }
        }

        StatementHint::StdLibCall { name, args } => {
            if args.is_empty() {
                Ok(format!("{}()", name))
            } else {
                Ok(format!("{}({})", name, args.join(", ")))
            }
        }

        StatementHint::Assert { expression } => Ok(format!("assert {}", expression)),
        StatementHint::StaticAssert { condition, message } => {
            if let Some(msg) = message {
                Ok(format!("assert {}, \"{}\"  # static assert", condition, msg))
            } else {
                Ok(format!("assert {}  # static assert", condition))
            }
        }
        StatementHint::CommaExpression { expressions } => {
            Ok(format!("({})", expressions.join(", ")))
        }
        StatementHint::Perror { message } => {
            if let Some(m) = message {
                Ok(format!("import sys\nprint({}, file=sys.stderr)", m))
            } else {
                Ok("import sys\nprint('error', file=sys.stderr)".to_string())
            }
        }
        StatementHint::ErrnoCheck => Ok("# errno check (not applicable)".to_string()),

        StatementHint::TryBlock => Ok("try:".to_string()),
        StatementHint::ExceptBlock { exception_type, variable } => {
            let ex = exception_type.clone().unwrap_or_else(|| "Exception".to_string());
            if let Some(var) = variable {
                Ok(format!("except {} as {}:", ex, var))
            } else {
                Ok(format!("except {}:", ex))
            }
        }
        StatementHint::FinallyBlock => Ok("finally:".to_string()),
        StatementHint::RaiseException { exception_type, message } => {
            if let Some(msg) = message {
                Ok(format!("raise {}(\"{}\")", exception_type, msg.trim_matches('\"')))
            } else {
                Ok(format!("raise {}", exception_type))
            }
        }
        StatementHint::ErrorCheck { function_call, .. } => Ok(format!("# check error for {}", function_call)),
        StatementHint::SetJmp { buffer } => Ok(format!("# setjmp {}", buffer)),
        StatementHint::LongJmp { buffer, value } => Ok(format!("# longjmp {} {}", buffer, value)),

        StatementHint::TestFunction { name, .. } => Ok(format!("def {}():\n    pass", name)),
        StatementHint::TestAssert { expression, message } => {
            if let Some(msg) = message {
                Ok(format!("assert {}, \"{}\"", expression, msg))
            } else {
                Ok(format!("assert {}", expression))
            }
        }
        StatementHint::TestAssertEqual { left, right, .. } => Ok(format!("assert {} == {}", left, right)),
        StatementHint::TestAssertNotEqual { left, right } => Ok(format!("assert {} != {}", left, right)),
        StatementHint::TestAssertTrue { expression } => Ok(format!("assert {}", expression)),
        StatementHint::TestAssertFalse { expression } => Ok(format!("assert not ({})", expression)),
        StatementHint::TestSetup { name } => Ok(format!("def {}():\n    pass", name)),
        StatementHint::TestTeardown { name } => Ok(format!("def {}():\n    pass", name)),
        StatementHint::MockFunction { name, return_value } => Ok(format!("def {}():\n    return {}", name, return_value)),

        StatementHint::LabeledBreak { label } => Ok(format!("# break {}", label)),
        StatementHint::LabeledContinue { label } => Ok(format!("# continue {}", label)),
        StatementHint::LabeledLoop { label, loop_hint } => {
            let loop_code = translate_hint_python(loop_hint)?;
            Ok(format!("# label {}\n{}", label, loop_code))
        }
        StatementHint::MatchBlock { expression } => Ok(format!("match {}:", expression)),
        StatementHint::MatchCase { pattern, guard, action } => {
            if let Some(g) = guard {
                if let Some(act) = action {
                    Ok(format!("case {} if {}:\n    {}", pattern, g, act))
                } else {
                    Ok(format!("case {} if {}:\n    pass", pattern, g))
                }
            } else if let Some(act) = action {
                Ok(format!("case {}:\n    {}", pattern, act))
            } else {
                Ok(format!("case {}:\n    pass", pattern))
            }
        }
        StatementHint::MatchWildcard { action } => {
            if let Some(act) = action {
                Ok(format!("case _:\n    {}", act))
            } else {
                Ok("case _:\n    pass".to_string())
            }
        }
        StatementHint::GuardClause { condition, action } => Ok(format!("if not ({}):\n    {}", condition, action)),
        StatementHint::ConditionalChain { conditions, else_action } => {
            let mut lines = Vec::new();
            for (i, (cond, act)) in conditions.iter().enumerate() {
                let keyword = if i == 0 { "if" } else { "elif" };
                lines.push(format!("{} {}:\n    {}", keyword, cond, act));
            }
            if let Some(act) = else_action {
                lines.push(format!("else:\n    {}", act));
            }
            Ok(lines.join("\n"))
        }

        StatementHint::LinkedListCreate { name, .. } => Ok(format!("{} = []", name)),
        StatementHint::LinkedListInsert { list, value, position } => {
            if let Some(pos) = position {
                Ok(format!("{}.insert({}, {})", list, pos, value))
            } else {
                Ok(format!("{}.append({})", list, value))
            }
        }
        StatementHint::LinkedListRemove { list, position } => Ok(format!("{}.pop({})", list, position)),
        StatementHint::LinkedListTraverse { list, iterator } => Ok(format!("for {} in {}:\n    pass", iterator, list)),
        StatementHint::StackCreate { name, .. } => Ok(format!("{} = []", name)),
        StatementHint::StackPush { stack, value } => Ok(format!("{}.append({})", stack, value)),
        StatementHint::StackPop { stack, target } => {
            if let Some(t) = target {
                Ok(format!("{} = {}.pop()", t, stack))
            } else {
                Ok(format!("{}.pop()", stack))
            }
        }
        StatementHint::StackPeek { stack, target } => {
            if let Some(t) = target {
                Ok(format!("{} = {}[-1]", t, stack))
            } else {
                Ok(format!("{}[-1]", stack))
            }
        }
        StatementHint::StackIsEmpty { stack } => Ok(format!("len({}) == 0", stack)),
        StatementHint::QueueCreate { name, .. } => Ok(format!("{} = []", name)),
        StatementHint::QueueEnqueue { queue, value } => Ok(format!("{}.append({})", queue, value)),
        StatementHint::QueueDequeue { queue, target } => {
            if let Some(t) = target {
                Ok(format!("{} = {}.pop(0)", t, queue))
            } else {
                Ok(format!("{}.pop(0)", queue))
            }
        }
        StatementHint::MapCreate { name, .. } => Ok(format!("{} = {{}}", name)),
        StatementHint::MapPut { map, key, value } => Ok(format!("{}[{}] = {}", map, key, value)),
        StatementHint::MapGet { map, key, target } => {
            if let Some(t) = target {
                Ok(format!("{} = {}.get({})", t, map, key))
            } else {
                Ok(format!("{}.get({})", map, key))
            }
        }
        StatementHint::MapRemove { map, key } => Ok(format!("{}.pop({}, None)", map, key)),
        StatementHint::MapContainsKey { map, key } => Ok(format!("{} in {}", key, map)),
        StatementHint::MapKeys { map, target } => {
            if let Some(t) = target {
                Ok(format!("{} = list({}.keys())", t, map))
            } else {
                Ok(format!("list({}.keys())", map))
            }
        }
        StatementHint::MapValues { map, target } => {
            if let Some(t) = target {
                Ok(format!("{} = list({}.values())", t, map))
            } else {
                Ok(format!("list({}.values())", map))
            }
        }
        StatementHint::SetCreate { name, .. } => Ok(format!("{} = set()", name)),
        StatementHint::SetAdd { set, value } => Ok(format!("{}.add({})", set, value)),
        StatementHint::SetRemove { set, value } => Ok(format!("{}.discard({})", set, value)),
        StatementHint::SetContains { set, value } => Ok(format!("{} in {}", value, set)),
        StatementHint::SetUnion { set1, set2, target } => Ok(format!("{} = {} | {}", target, set1, set2)),
        StatementHint::SetIntersection { set1, set2, target } => Ok(format!("{} = {} & {}", target, set1, set2)),
        StatementHint::TreeNode { name, value, left, right } => {
            let l = left.clone().unwrap_or_else(|| "None".to_string());
            let r = right.clone().unwrap_or_else(|| "None".to_string());
            Ok(format!("{} = ({}, {}, {})", name, value, l, r))
        }
        StatementHint::TreeInsert { tree, value } => Ok(format!("# insert {} into {}", value, tree)),
        StatementHint::TreeSearch { tree, value } => Ok(format!("# search {} in {}", value, tree)),
        StatementHint::TreeTraverse { tree, order } => Ok(format!("# traverse {} {}", tree, order)),

        StatementHint::ClassDef { name, parent, interfaces, .. } => {
            let mut bases = Vec::new();
            if let Some(p) = parent {
                bases.push(p.clone());
            }
            bases.extend(interfaces.clone());
            let base_list = if bases.is_empty() { String::new() } else { format!("({})", bases.join(", ")) };
            Ok(format!("class {}{}:\n    pass", name, base_list))
        }
        StatementHint::ClassField { name, initial_value, .. } => {
            let value = initial_value.clone().unwrap_or_else(|| "None".to_string());
            Ok(format!("{} = {}", name, value))
        }
        StatementHint::ClassMethod { name, parameters, is_static, .. } => {
            let mut params: Vec<String> = Vec::new();
            if !*is_static {
                params.push("self".to_string());
            }
            params.extend(parameters.iter().map(|(_, n)| n.clone()));
            let decorator = if *is_static { "@staticmethod\n" } else { "" };
            Ok(format!("{}def {}({}):\n    pass", decorator, name, params.join(", ")))
        }
        StatementHint::Constructor { parameters, .. } => {
            let mut params: Vec<String> = vec!["self".to_string()];
            params.extend(parameters.iter().map(|(_, n)| n.clone()));
            Ok(format!("def __init__({}):\n    pass", params.join(", ")))
        }
        StatementHint::Destructor { .. } => Ok("def __del__(self):\n    pass".to_string()),
        StatementHint::InterfaceDef { name, .. } => Ok(format!("class {}:\n    pass", name)),
        StatementHint::InterfaceMethod { name, parameters, .. } => {
            let mut params: Vec<String> = vec!["self".to_string()];
            params.extend(parameters.iter().map(|(_, n)| n.clone()));
            Ok(format!("def {}({}):\n    raise NotImplementedError()", name, params.join(", ")))
        }
        StatementHint::ObjectCreate { class_name, variable, arguments } => {
            if arguments.is_empty() {
                Ok(format!("{} = {}()", variable, class_name))
            } else {
                Ok(format!("{} = {}({})", variable, class_name, arguments.join(", ")))
            }
        }
        StatementHint::MethodCall { object, method, arguments } => {
            if arguments.is_empty() {
                Ok(format!("{}.{}()", object, method))
            } else {
                Ok(format!("{}.{}({})", object, method, arguments.join(", ")))
            }
        }
        StatementHint::PropertyAccess { object, property } => Ok(format!("{}.{}", object, property)),
        StatementHint::PropertyAssign { object, property, value } => Ok(format!("{}.{} = {}", object, property, value)),
        StatementHint::SuperCall { method, arguments } => {
            if let Some(m) = method {
                if arguments.is_empty() {
                    Ok(format!("super().{}()", m))
                } else {
                    Ok(format!("super().{}({})", m, arguments.join(", ")))
                }
            } else {
                Ok("super()".to_string())
            }
        }
        StatementHint::ThisReference => Ok("self".to_string()),

        StatementHint::Lambda { parameters, body, .. } => Ok(format!("lambda {}: {}", parameters.join(", "), body)),
        StatementHint::HigherOrderFunction { function, callback, collection } => {
            if let Some(col) = collection {
                Ok(format!("{}({}, {})", function, col, callback))
            } else {
                Ok(format!("{}({})", function, callback))
            }
        }
        StatementHint::MapFunction { collection, transform, target } => {
            let expr = format!("list(map({}, {}))", transform, collection);
            if let Some(t) = target {
                Ok(format!("{} = {}", t, expr))
            } else {
                Ok(expr)
            }
        }
        StatementHint::FilterFunction { collection, predicate, target } => {
            let expr = format!("list(filter({}, {}))", predicate, collection);
            if let Some(t) = target {
                Ok(format!("{} = {}", t, expr))
            } else {
                Ok(expr)
            }
        }
        StatementHint::ReduceFunction { collection, reducer, initial, target } => {
            let expr = if let Some(init) = initial {
                format!("functools.reduce({}, {}, {})", reducer, collection, init)
            } else {
                format!("functools.reduce({}, {})", reducer, collection)
            };
            if let Some(t) = target {
                Ok(format!("{} = {}", t, expr))
            } else {
                Ok(expr)
            }
        }
        StatementHint::ForEachFunction { collection, action } => Ok(format!("for _ in {}:\n    {}", collection, action)),

        StatementHint::ThreadCreate { function, arguments, .. } => {
            if arguments.is_empty() {
                Ok(format!("threading.Thread(target={}).start()", function))
            } else {
                Ok(format!("threading.Thread(target={}, args=({})).start()", function, arguments.join(", ")))
            }
        }
        StatementHint::ThreadJoin { thread } => Ok(format!("{}.join()", thread)),
        StatementHint::ThreadDetach { thread } => Ok(format!("# detach {}", thread)),
        StatementHint::ThreadSleep { duration, .. } => Ok(format!("time.sleep({})", duration)),
        StatementHint::MutexCreate { name } => Ok(format!("{} = threading.Lock()", name)),
        StatementHint::MutexLock { mutex } => Ok(format!("{}.acquire()", mutex)),
        StatementHint::MutexUnlock { mutex } => Ok(format!("{}.release()", mutex)),
        StatementHint::MutexTryLock { mutex } => Ok(format!("{}.acquire(blocking=False)", mutex)),
        StatementHint::SemaphoreCreate { name, initial } => Ok(format!("{} = threading.Semaphore({})", name, initial)),
        StatementHint::SemaphoreWait { semaphore } => Ok(format!("{}.acquire()", semaphore)),
        StatementHint::SemaphoreSignal { semaphore } => Ok(format!("{}.release()", semaphore)),
        StatementHint::ConditionCreate { name } => Ok(format!("{} = threading.Condition()", name)),
        StatementHint::ConditionWait { condition, .. } => Ok(format!("{}.wait()", condition)),
        StatementHint::ConditionSignal { condition } => Ok(format!("{}.notify()", condition)),
        StatementHint::ConditionBroadcast { condition } => Ok(format!("{}.notify_all()", condition)),
        StatementHint::AtomicCreate { name, initial } => Ok(format!("{} = {}", name, initial)),
        StatementHint::AtomicLoad { atomic, target } => {
            if let Some(t) = target {
                Ok(format!("{} = {}", t, atomic))
            } else {
                Ok(atomic.clone())
            }
        }
        StatementHint::AtomicStore { atomic, value } => Ok(format!("{} = {}", atomic, value)),
        StatementHint::AtomicCompareExchange { atomic, expected, desired } => Ok(format!("# compare_exchange {} {} {}", atomic, expected, desired)),
        StatementHint::AtomicIncrement { atomic } => Ok(format!("{} += 1", atomic)),
        StatementHint::AtomicDecrement { atomic } => Ok(format!("{} -= 1", atomic)),

        StatementHint::AsyncFunction { name, parameters, .. } => {
            let params = parameters.iter().map(|(_, n)| n.clone()).collect::<Vec<_>>().join(", ");
            Ok(format!("async def {}({}):\n    pass", name, params))
        }
        StatementHint::AwaitExpression { expression, target } => {
            if let Some(t) = target {
                Ok(format!("{} = await {}", t, expression))
            } else {
                Ok(format!("await {}", expression))
            }
        }
        StatementHint::PromiseCreate { name, executor } => Ok(format!("# promise {} with {}", name, executor)),
        StatementHint::PromiseThen { promise, handler } => Ok(format!("# {} then {}", promise, handler)),
        StatementHint::PromiseCatch { promise, handler } => Ok(format!("# {} catch {}", promise, handler)),
        StatementHint::PromiseAll { promises, target } => Ok(format!("{} = [{}]", target, promises.join(", "))),
        StatementHint::PromiseRace { promises, target } => Ok(format!("{} = [{}]", target, promises.join(", "))),

        StatementHint::DocComment { text, .. } => Ok(format!("\"\"\"{}\"\"\"", text)),
        StatementHint::DocFunction { brief, .. } => Ok(format!("\"\"\"{}\"\"\"", brief)),
        StatementHint::DocClass { brief, .. } => Ok(format!("\"\"\"{}\"\"\"", brief)),

        StatementHint::Unknown { original } => {
            Err(anyhow!("UNHANDLED: {}", original))
        }
    }
}

// ============================================================================
// Context-Aware Code Generation
// ============================================================================

use crate::hints::{ValidationResult, VariableContext};

/// Generate code with context-aware variable declarations.
/// This handles the smart declaration logic:
/// - Variables in `needs_declaration` get `int x = ...`
/// - Variables in `already_declared` get `x = ...`
pub fn translate_with_context(
    hint: &StatementHint,
    validation: &ValidationResult,
    context: &VariableContext,
    language: &str,
) -> Result<String> {
    match language.to_lowercase().as_str() {
        "python" => translate_hint_python(hint), // Python doesn't need type declarations
        _ => translate_with_context_c(hint, validation, context),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hints::{StatementHint, Visibility};

    #[test]
    fn test_translate_try_block() {
        let c = translate_from_hint(&StatementHint::TryBlock, "c").unwrap();
        assert!(c.contains("try"));
        let py = translate_from_hint(&StatementHint::TryBlock, "python").unwrap();
        assert!(py.starts_with("try:"));
    }

    #[test]
    fn test_translate_class_def_python() {
        let hint = StatementHint::ClassDef {
            name: "Foo".to_string(),
            parent: None,
            interfaces: Vec::new(),
            is_abstract: false,
        };
        let py = translate_from_hint(&hint, "python").unwrap();
        assert!(py.starts_with("class Foo"));
    }

    #[test]
    fn test_translate_stack_push_c() {
        let hint = StatementHint::StackPush { stack: "stack".to_string(), value: "x".to_string() };
        let c = translate_from_hint(&hint, "c").unwrap();
        assert!(c.contains("stack_push"));
    }

    #[test]
    fn test_translate_await_python() {
        let hint = StatementHint::AwaitExpression { expression: "fetch()".to_string(), target: None };
        let py = translate_from_hint(&hint, "python").unwrap();
        assert!(py.starts_with("await"));
    }

    #[test]
    fn test_translate_class_field_c() {
        let hint = StatementHint::ClassField {
            name: "count".to_string(),
            type_hint: Some("int".to_string()),
            visibility: Visibility::Private,
            is_static: false,
            initial_value: None,
        };
        let c = translate_from_hint(&hint, "c").unwrap();
        assert!(c.contains("int count"));
    }
}

/// Generate C code with context-aware declarations.
fn translate_with_context_c(
    hint: &StatementHint,
    validation: &ValidationResult,
    _context: &VariableContext,
) -> Result<String> {
    match hint {
        // Declaration with smart redeclaration handling
        StatementHint::Declaration { names, type_hint, qualifiers, initial_value, is_array, array_size } => {
            if names.is_empty() {
                return Err(anyhow!("Declaration requires at least one variable name"));
            }

            let mut lines = Vec::new();
            let mut new_vars = Vec::new();
            let mut existing_vars = Vec::new();

            for name in names {
                if validation.needs_declaration.contains(name) {
                    new_vars.push(name.clone());
                } else {
                    existing_vars.push(name.clone());
                }
            }

            // Handle arrays (only declare new ones; skip redeclare)
            let qualifier_prefix = if qualifiers.is_empty() {
                String::new()
            } else {
                format!("{} ", qualifiers.join(" "))
            };

            if *is_array {
                if !new_vars.is_empty() {
                    let ty = type_hint.as_deref().unwrap_or("int");
                    let size = array_size.as_deref().unwrap_or("10");
                    for n in new_vars {
                        lines.push(format!("{}{} {}[{}];", qualifier_prefix, ty, n, size));
                    }
                }
                // If all were existing arrays and no init, we skip emitting to avoid redeclare
                if lines.is_empty() {
                    return translate_hint_c(hint);
                }
                return Ok(lines.join("\n"));
            }

            // Non-array: declare new vars; assign existing when initial_value present
            let alloc_check_vars: Vec<String> = new_vars
                .iter()
                .cloned()
                .chain(existing_vars.iter().cloned())
                .collect();
            if !new_vars.is_empty() {
                let inferred = initial_value
                    .as_ref()
                    .and_then(|v| infer_c_type_from_expression(v))
                    .map(|ty| ty as &str);
                let ty = type_hint.as_deref().or(inferred).unwrap_or("int");
                if let Some(val) = initial_value {
                    let decls: Vec<String> = new_vars.iter().map(|v| format!("{} = {}", v, val)).collect();
                    lines.push(format!("{}{} {};", qualifier_prefix, ty, decls.join(", ")));
                } else {
                    lines.push(format!("{}{} {};", qualifier_prefix, ty, new_vars.join(", ")));
                }
            }

            if let Some(val) = initial_value {
                for n in existing_vars {
                    lines.push(format!("{} = {};", n, val));
                }
                if val.contains("malloc(") || val.contains("calloc(") || val.contains("realloc(") {
                    for var in alloc_check_vars {
                        lines.push(format!("if ({} == NULL) {{ perror(\"malloc failed\"); exit(1); }}", var));
                    }
                }
            }

            if lines.is_empty() {
                // No output needed (already declared and no init)
                return translate_hint_c(hint);
            }

            Ok(lines.join("\n"))
        }

        // Assignment with smart declaration
        StatementHint::Assignment { targets, value } => {
            if targets.is_empty() {
                return Err(anyhow!("Assignment requires at least one target"));
            }
            
            let mut lines = Vec::new();
            
            // Separate new declarations from existing assignments
            let mut new_vars = Vec::new();
            let mut existing_vars = Vec::new();
            
            for target in targets {
                if validation.needs_declaration.contains(target) {
                    new_vars.push(target.clone());
                } else {
                    existing_vars.push(target.clone());
                }
            }
            
            // Generate declarations for new variables
            if !new_vars.is_empty() {
                let inferred = infer_c_type_from_expression(value).unwrap_or("int");
                let decls: Vec<String> = new_vars
                    .iter()
                    .map(|v| format!("{} = {}", v, value))
                    .collect();
                lines.push(format!("{} {};", inferred, decls.join(", ")));
            }
            
            // Generate simple assignments for existing variables
            for var in existing_vars.iter() {
                lines.push(format!("{} = {};", var, value));
            }

            if value.contains("malloc(") || value.contains("calloc(") || value.contains("realloc(") {
                for var in new_vars.iter().chain(existing_vars.iter()) {
                    lines.push(format!("if ({} == NULL) {{ perror(\"malloc failed\"); exit(1); }}", var));
                }
            }
            
            if lines.is_empty() {
                // Fallback - shouldn't happen but be safe
                return translate_hint_c(hint);
            }
            
            Ok(lines.join("\n"))
        }
        
        // Loop with iterator declaration
        StatementHint::Loop {
            iterator,
            start,
            end,
            collection,
            body_action,
        } => {
            let iter = iterator.as_deref().unwrap_or("i");
            let needs_decl = validation.needs_declaration.contains(&iter.to_string());
            
            if let Some(col) = collection {
                let body = body_action
                    .as_ref()
                    .map(|a| format!("    {};", a))
                    .unwrap_or_else(|| "    ".to_string());
                    
                if needs_decl {
                    Ok(format!(
                        "for (int {iter} = 0; {iter} < sizeof({col}) / sizeof({col}[0]); {iter}++) {{\n{body}\n}}",
                        iter = iter, col = col, body = body
                    ))
                } else {
                    Ok(format!(
                        "for ({iter} = 0; {iter} < sizeof({col}) / sizeof({col}[0]); {iter}++) {{\n{body}\n}}",
                        iter = iter, col = col, body = body
                    ))
                }
            } else {
                let start_val = start.as_deref().unwrap_or("0");
                let end_val = end.as_deref().unwrap_or("10");
                let body = body_action
                    .as_ref()
                    .map(|a| format!("    {};", a))
                    .unwrap_or_else(|| "    ".to_string());
                    
                if needs_decl {
                    Ok(format!(
                        "for (int {iter} = {start}; {iter} <= {end}; {iter}++) {{\n{body}\n}}",
                        iter = iter, start = start_val, end = end_val, body = body
                    ))
                } else {
                    Ok(format!(
                        "for ({iter} = {start}; {iter} <= {end}; {iter}++) {{\n{body}\n}}",
                        iter = iter, start = start_val, end = end_val, body = body
                    ))
                }
            }
        }
        
        // Read with declaration (always declares)
        StatementHint::Read { variables } => {
            if variables.is_empty() {
                return Err(anyhow!("Read requires at least one variable"));
            }
            let mut lines = Vec::new();
            for var in variables {
                // Only declare if it's new
                if validation.needs_declaration.contains(var) {
                    lines.push(format!("int {};", var));
                }
                lines.push(format!("if (scanf(\"%d\", &{}) != 1) {{ fprintf(stderr, \"Invalid input\\n\"); return 1; }}", var));
            }
            Ok(lines.join("\n"))
        }
        
        // Print with context-aware variable resolution
        StatementHint::Print { content, is_literal } => {
            // Override is_literal based on context
            let final_is_literal = if content.starts_with('"') || content.starts_with('\'') {
                // Quoted - always literal
                true
            } else if *is_literal {
                // Was determined to be literal at parse time
                true
            } else {
                // Check if it's actually a declared variable
                !_context.is_declared(content)
            };
            
            if final_is_literal {
                let inner = content.trim_matches('"').trim_matches('\'');
                Ok(format!("printf(\"{}\\n\");", inner))
            } else {
                Ok(format!("printf(\"%d\\n\", {});", content))
            }
        }
        
        // While loop with smart variable handling
        StatementHint::While { condition, body_action } => {
            // Parse condition to find the loop variable
            let loop_info = parse_while_condition(condition);
            
            let mut result = Vec::new();
            
            // Only auto-declare and auto-increment for counter patterns (i < 10, j <= n, etc.)
            // For general comparisons like (a < b), just generate the while loop
            if loop_info.is_counter_pattern {
                // If we found a loop variable that needs declaration, declare it
                if let Some(ref var) = loop_info.variable {
                    if !_context.is_declared(var) {
                        // Determine initial value based on comparison direction
                        let init_val = if loop_info.is_less_than {
                            // Start at 0 for i < n
                            "0".to_string()
                        } else {
                            // Start at the bound for i > 0 
                            loop_info.bound.clone().unwrap_or_else(|| "0".to_string())
                        };
                        result.push(format!("int {} = {};", var, init_val));
                    }
                }
            }
            
            // Build the body
            let body = if let Some(action) = body_action {
                format!("    {};", action)
            } else if loop_info.is_counter_pattern {
                // Only auto-add incrementer/decrementer for counter patterns
                // Add an empty line before the incrementer for cursor positioning
                if let Some(ref var) = loop_info.variable {
                    if loop_info.is_less_than {
                        format!("    \n    {}++;", var)
                    } else {
                        format!("    \n    {}--;", var)
                    }
                } else {
                    "    ".to_string()
                }
            } else {
                // General while loop - leave body empty for user
                "    ".to_string()
            };
            
            result.push(format!("while ({}) {{\n{}\n}}", condition, body));
            
            Ok(result.join("\n"))
        }
        
        // Conditional with proper cursor positioning
        StatementHint::Conditional { condition, then_action, else_action, is_else_if } => {
            let keyword = if *is_else_if { "else if" } else { "if" };
            let mut lines = vec![format!("{} ({}) {{", keyword, condition)];
            
            if let Some(action) = then_action {
                // Add empty line for cursor, then the action
                lines.push("    ".to_string());
                let translated = translate_inline_action(action);
                lines.push(format!("    {}", translated));
            } else {
                // Empty body - cursor goes here
                lines.push("    ".to_string());
            }
            lines.push("}".to_string());
            
            if let Some(else_act) = else_action {
                lines.push("else {".to_string());
                // Add empty line for cursor, then the action
                lines.push("    ".to_string());
                let translated = translate_inline_action(else_act);
                lines.push(format!("    {}", translated));
                lines.push("}".to_string());
            }
            
            Ok(lines.join("\n"))
        }
        
        // For other hint types, delegate to the basic translator
        _ => translate_hint_c(hint),
    }
}

/// Parse a while condition to extract loop variable info
struct WhileLoopInfo {
    variable: Option<String>,
    bound: Option<String>,
    is_less_than: bool, // true for <, <=; false for >, >=
    is_counter_pattern: bool, // true if this looks like a counter loop (i < 10)
}

fn parse_while_condition(condition: &str) -> WhileLoopInfo {
    let condition = condition.trim();
    
    // Look for comparison operators
    for (op, is_less) in [(" < ", true), (" <= ", true), (" > ", false), (" >= ", false)] {
        if let Some(idx) = condition.find(op) {
            let left = condition[..idx].trim();
            let right = condition[idx + op.len()..].trim();
            
            // Left side is likely the variable, right is the bound
            // Check if left looks like an identifier
            if left.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') 
               && left.chars().next().map(|c| c.is_ascii_alphabetic() || c == '_').unwrap_or(false) {
                
                // Determine if this is a counter pattern:
                // - Variable is a common iterator name (i, j, k, n, count, counter, idx, index)
                // - OR bound is a numeric literal
                let is_counter_var = matches!(left.to_lowercase().as_str(), 
                    "i" | "j" | "k" | "n" | "count" | "counter" | "idx" | "index" | "iter");
                let bound_is_numeric = right.chars().all(|c| c.is_ascii_digit() || c == '-');
                let is_counter_pattern = is_counter_var || bound_is_numeric;
                
                return WhileLoopInfo {
                    variable: Some(left.to_string()),
                    bound: Some(right.to_string()),
                    is_less_than: is_less,
                    is_counter_pattern,
                };
            }
        }
    }
    
    // Couldn't parse - no auto-handling
    WhileLoopInfo {
        variable: None,
        bound: None,
        is_less_than: true,
        is_counter_pattern: false,
    }
}

/// Check if print content should be treated as a variable based on context
pub fn resolve_print_content(content: &str, context: &VariableContext) -> (String, bool) {
    // Quoted content is always literal
    if content.starts_with('"') || content.starts_with('\'') {
        return (content.to_string(), true);
    }
    
    // Check if it's a declared variable
    if context.is_declared(content) {
        return (content.to_string(), false); // is_literal = false means it's a variable
    }
    
    // Not declared - treat as literal string
    (content.to_string(), true)
}

