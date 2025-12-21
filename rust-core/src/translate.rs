//! Code generation from English instructions and structured hints.
//!
//! This module contains both the new hint-based translation functions
//! and the legacy string-based translation functions (kept for potential
//! future use or reference).

#![allow(dead_code)]

use anyhow::{anyhow, Result};

use crate::hints::{ArithmeticOp as HintArithmeticOp, StatementHint};
use crate::models::TranslateLineRequest;

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
    match normalize_action(action) {
        Some(Action::Literal(expr)) => format!("    printf(\"{}\\n\");", expr),
        Some(Action::Identifier(expr)) => format!("    printf(\"%s\\n\", {expr});"),
        Some(Action::CollectionIndex(expr)) => format!("    printf(\"%d\\n\", {expr}[{iter_name}]);"),
        Some(Action::Printf(expr)) => {
            if expr.trim_end().ends_with(';') {
                format!("    {expr}")
            } else {
                format!("    {expr};")
            }
        }
        Some(Action::Raw(expr)) => {
            if expr.trim_end().ends_with(';') {
                format!("    {expr}")
            } else {
                format!("    {expr};")
            }
        }
        None => "    ".to_string(),
    }
}

fn build_python_body_line(action: Option<&str>, iterator: Option<&str>) -> String {
    let iter_name = iterator.unwrap_or("i");
    match normalize_action(action) {
        Some(Action::Literal(expr)) => format!("    print(\"{}\")", expr),
        Some(Action::Identifier(expr)) => format!("    print({expr})"),
        Some(Action::CollectionIndex(expr)) => format!("    print({expr}[{iter_name}])"),
        Some(Action::Printf(expr)) => format!("    print({expr})"),
        Some(Action::Raw(expr)) => format!("    {expr}"),
        None => "    pass".to_string(),
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

/// Generate C code from a hint.
fn translate_hint_c(hint: &StatementHint) -> Result<String> {
    match hint {
        StatementHint::Declaration {
            names,
            type_hint,
            initial_value,
            is_array,
            array_size,
        } => {
            if names.is_empty() {
                return Err(anyhow!("Declaration requires at least one variable name"));
            }
            
            let c_type = type_hint.as_deref().unwrap_or("int");
            
            if *is_array {
                let size = array_size.as_deref().unwrap_or("10");
                let decls: Vec<String> = names
                    .iter()
                    .map(|name| format!("{} {}[{}];", c_type, name, size))
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
                Ok(format!("{} {};", c_type, declarations.join(", ")))
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

        StatementHint::Read { variables } => {
            if variables.is_empty() {
                return Err(anyhow!("Read requires at least one variable"));
            }
            let mut lines = Vec::new();
            for var in variables {
                lines.push(format!("int {};", var));
                lines.push(format!("scanf(\"%d\", &{});", var));
            }
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
                lines.push(format!("    {};", action));
            } else {
                lines.push("    ".to_string());
            }
            lines.push("}".to_string());
            
            if let Some(else_act) = else_action {
                lines.push("else {".to_string());
                lines.push(format!("    {};", else_act));
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
                };
                Ok(format!("int {} = {};", default_name, expr))
            }
        }

        StatementHint::Modify { target, delta } => {
            if *delta > 0 {
                Ok(format!("{}++;", target))
            } else {
                Ok(format!("{}--;", target))
            }
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

        StatementHint::MainFunction => {
            Ok("int main(void) {\n    return 0;\n}".to_string())
        }

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

        StatementHint::MainFunction => {
            Ok("def main():\n    pass\n\nif __name__ == \"__main__\":\n    main()".to_string())
        }

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

/// Generate C code with context-aware declarations.
fn translate_with_context_c(
    hint: &StatementHint,
    validation: &ValidationResult,
    _context: &VariableContext,
) -> Result<String> {
    match hint {
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
                let decls: Vec<String> = new_vars
                    .iter()
                    .map(|v| format!("{} = {}", v, value))
                    .collect();
                lines.push(format!("int {};", decls.join(", ")));
            }
            
            // Generate simple assignments for existing variables
            for var in existing_vars {
                lines.push(format!("{} = {};", var, value));
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
                lines.push(format!("scanf(\"%d\", &{});", var));
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
        
        // For other hint types, delegate to the basic translator
        _ => translate_hint_c(hint),
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

