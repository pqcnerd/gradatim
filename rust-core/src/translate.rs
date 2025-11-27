use anyhow::{anyhow, Result};

use crate::models::TranslateLineRequest;

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

    if let Some(rest) = strip_keyword(line, "declare") {
        return handle_declare(rest);
    }

    if starts_with_list_keyword(line) {
        return handle_declare(line);
    }

    if let Some(rest) = strip_keyword(line, "set") {
        return handle_assignment(rest);
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

    if let Some(rest) = strip_keyword(line, "if") {
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

    if line.trim().eq_ignore_ascii_case("else") {
        return Ok("else {\n    \n}".to_string());
    }

    if line.to_lowercase().starts_with("else if ") {
        let condition = line[8..].trim();
        let c_expr = normalize_condition(condition);
        return Ok(format!("else if ({c_expr}) {{\n    \n}}"));
    }

    if line.to_lowercase().starts_with("end") {
        return Ok("}".to_string());
    }

    if let Some(rest) = strip_keyword(line, "loop")
        .or_else(|| strip_keyword(line, "for loop"))
        .or_else(|| strip_keyword(line, "for"))
    {
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
        let size_expr = value.unwrap_or("10");
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
            if let Some(value) = value {
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

    let (label_section, after_from_section) = if let Some(pos) = lower.find("from") {
        (
            remainder[..pos].trim().to_string(),
            remainder[pos + 4..].trim_start(),
        )
    } else {
        ("i".to_string(), remainder)
    };

    let lower_after_from = after_from_section.to_lowercase();
    let to_rel = lower_after_from.find("to").ok_or_else(|| {
        anyhow!("Loop sentences should include `to <end>` after the starting expression")
    })?;

    let start = after_from_section[..to_rel].trim();
    let tail = after_from_section[to_rel + 2..].trim();
    if start.is_empty() || tail.is_empty() {
        return Err(anyhow!("Loop bounds are incomplete"));
    }

    let (end_expr, action) = split_range_and_action(tail);
    if end_expr.is_empty() {
        return Err(anyhow!("Loop end expression is missing"));
    }

    let iterator = {
        let sanitized = sanitize_identifier(&label_section);
        if sanitized.is_empty() {
            "i".to_string()
        } else {
            sanitized
        }
    };

    let start_expr = extract_range_value(start);
    let mut lines = Vec::with_capacity(3);
    lines.push(format!(
        "for (int {iter} = {start}; {iter} < {end}; {iter}++) {{",
        iter = iterator,
        start = start_expr,
        end = end_expr
    ));
    lines.push(build_c_body_line(action.as_deref(), Some(&iterator)));
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
        let size_expr = value.unwrap_or("10");
        let decls: Vec<String> = names
            .into_iter()
            .map(|name| format!("{name} = [0] * {size}", size = size_expr))
            .collect();
        return Ok(decls.join("\n"));
    }

    let rhs = value.unwrap_or("None");

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

    let (label_section, after_from_section) = if let Some(pos) = lower.find("from") {
        (
            remainder[..pos].trim().to_string(),
            remainder[pos + 4..].trim_start(),
        )
    } else {
        ("i".to_string(), remainder)
    };

    let lower_after_from = after_from_section.to_lowercase();
    let to_rel = lower_after_from.find("to").ok_or_else(|| {
        anyhow!("Loop sentences should include `to <end>` after the starting expression")
    })?;

    let start_raw = after_from_section[..to_rel].trim();
    let tail = after_from_section[to_rel + 2..].trim();
    if start_raw.is_empty() || tail.is_empty() {
        return Err(anyhow!("Loop bounds are incomplete"));
    }

    let (end_expr, action) = split_range_and_action(tail);
    if end_expr.is_empty() {
        return Err(anyhow!("Loop end expression is missing"));
    }

    let iterator = {
        let sanitized = sanitize_identifier(&label_section);
        if sanitized.is_empty() {
            "i".to_string()
        } else {
            sanitized
        }
    };

    let start_expr = extract_range_value(start_raw);
    let mut lines = Vec::new();
    lines.push(format!(
        "for {iter} in range({start}, {end}):",
        iter = iterator,
        start = start_expr,
        end = end_expr
    ));
    lines.push(build_python_body_line(action.as_deref(), Some(&iterator)));

    Ok(lines.join("\n"))
}

fn split_value(input: &str) -> (&str, Option<&str>) {
    if let Some(idx) = input.rfind(char::is_whitespace) {
        let (head, tail) = input.split_at(idx);
        let value = tail.trim();

        if looks_like_value(value) {
            return (head.trim_end(), Some(value));
        }
    }

    (input, None)
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
    let trimmed = line.trim_start().to_lowercase();
    trimmed.starts_with("list ")
        || trimmed.starts_with("array ")
        || trimmed.starts_with("vector ")
        || trimmed.starts_with("arr ")
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
        let lower = token.to_lowercase();
        if matches!(lower.as_str(), "list" | "array" | "vector" | "arr") {
            is_list = true;
            continue;
        }
        if matches!(lower.as_str(), "of" | "a" | "an" | "with") {
            continue;
        }
        filtered.push(token);
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

