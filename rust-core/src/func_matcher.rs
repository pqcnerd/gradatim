use std::collections::HashSet;

use crate::stdlib_db::{FunctionDatabase, FunctionSpec};

#[derive(Debug)]
pub struct FunctionMatch {
    pub name: String,
    pub args: Vec<String>,
}

/// Attempt to match an English line to a stdlib function based on included headers.
pub fn match_function_call(
    english: &str,
    included_headers: &HashSet<String>,
    db: &FunctionDatabase,
) -> Option<FunctionMatch> {
    let line = english.trim();
    if line.is_empty() {
        return None;
    }

    // Normalize headers: allow both with/without .h
    let normalized: HashSet<String> = included_headers
        .iter()
        .map(|h| if h.ends_with(".h") { h.clone() } else { format!("{}.h", h) })
        .collect();

    // Try each function whose header is included
    for func in db.all() {
        if !normalized.contains(func.header) {
            continue;
        }

        // Simple style: starts with function name
        if line.to_lowercase().starts_with(func.name) {
            let rest = line[func.name.len()..].trim();
            let args = parse_args(rest);
            return Some(FunctionMatch {
                name: func.name.to_string(),
                args,
            });
        }

        // Natural patterns
        for pat in func.patterns {
            if line.to_lowercase().starts_with(&pat.to_lowercase()) {
                return match_natural(func, line);
            }
        }
    }

    None
}

fn parse_args(rest: &str) -> Vec<String> {
    if rest.is_empty() {
        return Vec::new();
    }
    // split by comma first, then whitespace
    let mut args = Vec::new();
    for chunk in rest.split(',') {
        let trimmed = chunk.trim();
        if !trimmed.is_empty() {
            args.push(trimmed.to_string());
        }
    }
    if args.len() == 1 && args[0].contains(' ') {
        // fallback: space separated
        args = args[0].split_whitespace().map(|s| s.to_string()).collect();
    }
    args
}

fn match_natural(func: &FunctionSpec, line: &str) -> Option<FunctionMatch> {
    let lower = line.to_lowercase();
    match func.name {
        "printf" => {
            if lower.starts_with("print formatted") || lower.starts_with("print format") || lower.starts_with("print ") {
                let rest = line.splitn(3, ' ').skip(2).collect::<Vec<&str>>().join(" ");
                let (fmt, args) = parse_printf(rest.trim());
                return Some(FunctionMatch { name: "printf".to_string(), args: vec![fmt].into_iter().chain(args).collect() });
            }
            None
        }
        "sqrt" => {
            if let Some(idx) = lower.find("square root of") {
                let arg = line[idx + "square root of".len()..].trim();
                if !arg.is_empty() {
                    return Some(FunctionMatch { name: "sqrt".to_string(), args: vec![arg.to_string()] });
                }
            }
            None
        }
        "malloc" => {
            if lower.contains("allocate") && lower.contains("byte") {
                let tokens: Vec<&str> = line.split_whitespace().collect();
                for t in tokens {
                    if let Ok(_) = t.parse::<usize>() {
                        return Some(FunctionMatch { name: "malloc".to_string(), args: vec![t.to_string()] });
                    }
                }
            }
            None
        }
        "strcpy" => {
            if lower.starts_with("copy string") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let src = parts[2];
                    let dest = parts.last().unwrap_or(&"");
                    if !src.is_empty() && !dest.is_empty() {
                        return Some(FunctionMatch { name: "strcpy".to_string(), args: vec![dest.to_string(), src.to_string()] });
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn parse_printf(text: &str) -> (String, Vec<String>) {
    if text.is_empty() {
        return ("\"\"".to_string(), Vec::new());
    }
    let mut tokens = text.split_whitespace().peekable();
    let mut fmt = String::new();
    let mut args = Vec::new();
    while let Some(tok) = tokens.next() {
        let lt = tok.to_lowercase();
        if lt == "percent" {
            if let Some(next) = tokens.next() {
                let code = match next.to_lowercase().as_str() {
                    "d" | "i" | "int" | "integer" => "%d",
                    "s" | "string" => "%s",
                    "f" | "float" => "%f",
                    "c" | "char" | "character" => "%c",
                    _ => "",
                };
                if !code.is_empty() {
                    fmt.push_str(code);
                }
            }
        } else if lt == "newline" {
            fmt.push_str("\\n");
        } else {
            // treat as argument start
            let mut collected = vec![tok.to_string()];
            collected.extend(tokens.map(|s| s.to_string()));
            args.push(collected.join(" "));
            break;
        }
    }

    if fmt.is_empty() && !args.is_empty() {
        fmt.push_str("%s");
    }

    (format!("\"{}\"", fmt), args)
}
