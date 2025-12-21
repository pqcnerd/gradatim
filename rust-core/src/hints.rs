//! Structured hint extraction from English instructions.
//!
//! This module parses English/pseudocode and extracts structured hints
//! that help the AI generate more accurate code. The hints describe
//! the *intent* of the instruction without being language-specific.

use serde::Serialize;

use crate::models::TranslateLineRequest;

/// A structured hint extracted from an English instruction.
/// These hints are language-agnostic and describe programmer intent.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StatementHint {
    /// Variable declaration: "declare x", "create an integer called count"
    Declaration {
        names: Vec<String>,
        type_hint: Option<String>,
        initial_value: Option<String>,
        is_array: bool,
        array_size: Option<String>,
    },

    /// Assignment: "set x to 5", "assign 10 to counter"
    Assignment {
        target: String,
        value: String,
    },

    /// Loop construct: "loop from 0 to 10", "iterate over array"
    Loop {
        iterator: Option<String>,
        start: Option<String>,
        end: Option<String>,
        collection: Option<String>,
        body_action: Option<String>,
    },

    /// Conditional: "if x > 5", "when count equals zero"
    Conditional {
        condition: String,
        then_action: Option<String>,
        else_action: Option<String>,
        is_else_if: bool,
    },

    /// Else branch: "else", "otherwise"
    Else,

    /// End block: "end", "end if", "end loop"
    EndBlock,

    /// Print/output: "print hello", "output the result"
    Print {
        content: String,
        is_literal: bool,
    },

    /// Return statement: "return x", "return 0"
    Return {
        value: Option<String>,
    },

    /// Arithmetic operation: "add a and b", "multiply x by 2"
    Arithmetic {
        operation: ArithmeticOp,
        left: String,
        right: String,
        target: Option<String>,
    },

    /// Increment/decrement: "increment x", "decrement counter"
    Modify {
        target: String,
        delta: i32, // +1 or -1
    },

    /// Function definition: "function foo taking int x"
    FunctionDef {
        name: String,
        parameters: Vec<(String, String)>, // (type, name)
        return_type: Option<String>,
    },

    /// Struct definition: "struct Point with x and y"
    StructDef {
        name: String,
        fields: Vec<(String, String)>, // (type, name)
    },

    /// Main entry point: "create main", "entry point"
    MainFunction,

    /// Unknown - AI handles without specific hints
    Unknown {
        original: String,
    },
}

/// Arithmetic operation types.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArithmeticOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl StatementHint {
    /// Returns true if this hint represents a trivial/simple construct
    /// that can be handled without AI.
    pub fn is_trivial(&self) -> bool {
        match self {
            // Simple declarations without arrays
            StatementHint::Declaration { is_array: false, .. } => true,
            // Basic operations
            StatementHint::Assignment { .. } => true,
            StatementHint::Modify { .. } => true,
            StatementHint::Return { .. } => true,
            StatementHint::Print { .. } => true,
            StatementHint::Arithmetic { .. } => true,
            // Control flow markers
            StatementHint::Else => true,
            StatementHint::EndBlock => true,
            StatementHint::MainFunction => true,
            // Simple conditionals without embedded actions (rule-based generates complete blocks)
            StatementHint::Conditional { then_action: None, else_action: None, .. } => true,
            // Simple loops (rule-based generates complete blocks with braces)
            StatementHint::Loop { .. } => true,
            // Everything else goes to AI
            _ => false,
        }
    }

    /// Format hints as a human-readable string for the AI prompt.
    pub fn to_prompt_hints(&self) -> String {
        match self {
            StatementHint::Declaration {
                names,
                type_hint,
                initial_value,
                is_array,
                array_size,
            } => {
                let mut parts = vec![format!("- Intent: Declaration")];
                parts.push(format!("- Variables: {}", names.join(", ")));
                if let Some(ty) = type_hint {
                    parts.push(format!("- Type: {}", ty));
                }
                if *is_array {
                    parts.push("- Is array: yes".to_string());
                    if let Some(size) = array_size {
                        parts.push(format!("- Array size: {}", size));
                    }
                }
                if let Some(val) = initial_value {
                    parts.push(format!("- Initial value: {}", val));
                }
                parts.join("\n")
            }

            StatementHint::Assignment { target, value } => {
                format!("- Intent: Assignment\n- Target: {}\n- Value: {}", target, value)
            }

            StatementHint::Loop {
                iterator,
                start,
                end,
                collection,
                body_action,
            } => {
                let mut parts = vec!["- Intent: Loop".to_string()];
                if let Some(iter) = iterator {
                    parts.push(format!("- Iterator: {}", iter));
                }
                if let Some(s) = start {
                    parts.push(format!("- Start: {}", s));
                }
                if let Some(e) = end {
                    parts.push(format!("- End: {}", e));
                }
                if let Some(col) = collection {
                    parts.push(format!("- Collection: {}", col));
                }
                if let Some(action) = body_action {
                    parts.push(format!("- Body action: {}", action));
                }
                parts.join("\n")
            }

            StatementHint::Conditional {
                condition,
                then_action,
                else_action,
                is_else_if,
            } => {
                let mut parts = vec![
                    format!("- Intent: {}", if *is_else_if { "Else-If" } else { "If" }),
                    format!("- Condition: {}", condition),
                ];
                if let Some(then) = then_action {
                    parts.push(format!("- Then: {}", then));
                }
                if let Some(els) = else_action {
                    parts.push(format!("- Else: {}", els));
                }
                parts.join("\n")
            }

            StatementHint::Else => "- Intent: Else branch".to_string(),

            StatementHint::EndBlock => "- Intent: End block (closing brace)".to_string(),

            StatementHint::Print { content, is_literal } => {
                format!(
                    "- Intent: Print\n- Content: {}\n- Is literal string: {}",
                    content,
                    if *is_literal { "yes" } else { "no" }
                )
            }

            StatementHint::Return { value } => {
                if let Some(val) = value {
                    format!("- Intent: Return\n- Value: {}", val)
                } else {
                    "- Intent: Return (void)".to_string()
                }
            }

            StatementHint::Arithmetic {
                operation,
                left,
                right,
                target,
            } => {
                let op_str = match operation {
                    ArithmeticOp::Add => "Add",
                    ArithmeticOp::Subtract => "Subtract",
                    ArithmeticOp::Multiply => "Multiply",
                    ArithmeticOp::Divide => "Divide",
                };
                let mut parts = vec![
                    format!("- Intent: Arithmetic ({})", op_str),
                    format!("- Left operand: {}", left),
                    format!("- Right operand: {}", right),
                ];
                if let Some(t) = target {
                    parts.push(format!("- Store result in: {}", t));
                }
                parts.join("\n")
            }

            StatementHint::Modify { target, delta } => {
                let action = if *delta > 0 { "Increment" } else { "Decrement" };
                format!("- Intent: {}\n- Target: {}", action, target)
            }

            StatementHint::FunctionDef {
                name,
                parameters,
                return_type,
            } => {
                let mut parts = vec![
                    "- Intent: Function definition".to_string(),
                    format!("- Name: {}", name),
                ];
                if !parameters.is_empty() {
                    let params: Vec<String> = parameters
                        .iter()
                        .map(|(ty, n)| format!("{} {}", ty, n))
                        .collect();
                    parts.push(format!("- Parameters: {}", params.join(", ")));
                }
                if let Some(ret) = return_type {
                    parts.push(format!("- Return type: {}", ret));
                }
                parts.join("\n")
            }

            StatementHint::StructDef { name, fields } => {
                let mut parts = vec![
                    "- Intent: Struct definition".to_string(),
                    format!("- Name: {}", name),
                ];
                if !fields.is_empty() {
                    let flds: Vec<String> = fields
                        .iter()
                        .map(|(ty, n)| format!("{} {}", ty, n))
                        .collect();
                    parts.push(format!("- Fields: {}", flds.join(", ")));
                }
                parts.join("\n")
            }

            StatementHint::MainFunction => "- Intent: Main function entry point".to_string(),

            StatementHint::Unknown { original } => {
                format!("- Intent: Unknown (AI should interpret)\n- Original: \"{}\"", original)
            }
        }
    }
}

// ============================================================================
// Hint Extraction Keywords
// ============================================================================

const DECLARE_KEYWORDS: &[&str] = &[
    "declare", "define", "create", "make", "set up", "setup", "initialize", "init",
    "turn", "convert", "build", "form",
];

const LOOP_KEYWORDS: &[&str] = &[
    "loop", "for loop", "for each", "foreach", "for every", "for all",
    "iterate", "iterate over", "iterate through", "loop over", "loop through",
    "go through", "go over", "cycle through", "traverse", "walk through",
];

const IF_KEYWORDS: &[&str] = &[
    "if", "when", "whenever", "in case", "provided that", "assuming",
    "in the event", "should",
];

const ELSE_IF_KEYWORDS: &[&str] = &["else if", "otherwise if", "elsewhen", "else when"];

const LIST_KEYWORDS: &[&str] = &["list", "array", "vector", "arr", "collection"];

const ADD_KEYWORDS: &[&str] = &["add", "sum", "total", "combine", "plus", "tally", "sum up"];
const SUBTRACT_KEYWORDS: &[&str] = &[
    "subtract", "minus", "difference", "difference between", "difference of",
    "remove", "take away", "decrease",
];
const MULTIPLY_KEYWORDS: &[&str] = &["multiply", "product", "product of", "times"];
const DIVIDE_KEYWORDS: &[&str] = &["divide", "quotient", "quotient of", "split", "divide by"];

// ============================================================================
// Extraction Functions
// ============================================================================

/// Extract structured hints from a translation request.
pub fn extract(request: &TranslateLineRequest) -> StatementHint {
    let line = request.english_line.trim();

    if line.is_empty() {
        return StatementHint::Unknown {
            original: line.to_string(),
        };
    }

    // Try each extractor in order of specificity
    if let Some(hint) = try_extract_else_if(line) {
        return hint;
    }

    if let Some(hint) = try_extract_else(line) {
        return hint;
    }

    if let Some(hint) = try_extract_end_block(line) {
        return hint;
    }

    if let Some(hint) = try_extract_main(line) {
        return hint;
    }

    if let Some(hint) = try_extract_return(line) {
        return hint;
    }

    if let Some(hint) = try_extract_print(line) {
        return hint;
    }

    if let Some(hint) = try_extract_increment_decrement(line) {
        return hint;
    }

    if let Some(hint) = try_extract_arithmetic(line) {
        return hint;
    }

    if let Some(hint) = try_extract_assignment(line) {
        return hint;
    }

    if let Some(hint) = try_extract_declaration(line) {
        return hint;
    }

    if let Some(hint) = try_extract_loop(line) {
        return hint;
    }

    if let Some(hint) = try_extract_conditional(line) {
        return hint;
    }

    if let Some(hint) = try_extract_function(line) {
        return hint;
    }

    if let Some(hint) = try_extract_struct(line) {
        return hint;
    }

    // Fallback to unknown
    StatementHint::Unknown {
        original: line.to_string(),
    }
}

fn strip_keyword<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let lower = line.to_lowercase();
    if lower.starts_with(keyword) {
        let rest = &line[keyword.len()..];
        if rest.is_empty() || rest.starts_with(' ') || rest.starts_with(':') {
            return Some(rest.trim_start());
        }
    }
    None
}

fn strip_any_keyword<'a>(line: &'a str, keywords: &[&str]) -> Option<&'a str> {
    // Sort by length descending to match longer keywords first
    let mut sorted: Vec<_> = keywords.iter().collect();
    sorted.sort_by(|a, b| b.len().cmp(&a.len()));
    
    for keyword in sorted {
        if let Some(rest) = strip_keyword(line, keyword) {
            return Some(rest);
        }
    }
    None
}

fn try_extract_else_if(line: &str) -> Option<StatementHint> {
    let rest = strip_any_keyword(line, ELSE_IF_KEYWORDS)?;
    let condition = normalize_condition(rest.trim());
    Some(StatementHint::Conditional {
        condition,
        then_action: None,
        else_action: None,
        is_else_if: true,
    })
}

fn try_extract_else(line: &str) -> Option<StatementHint> {
    let trimmed = line.trim().to_lowercase();
    if trimmed == "else" || trimmed == "otherwise" {
        Some(StatementHint::Else)
    } else {
        None
    }
}

fn try_extract_end_block(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    if lower.starts_with("end") || lower == "}" {
        Some(StatementHint::EndBlock)
    } else {
        None
    }
}

fn try_extract_main(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    if lower.starts_with("create main")
        || lower.starts_with("make main")
        || lower.starts_with("write main")
        || lower.starts_with("build main")
        || lower.contains("entry point")
        || lower == "main"
    {
        Some(StatementHint::MainFunction)
    } else {
        None
    }
}

fn try_extract_return(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "return")?;
    let value = if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    };
    Some(StatementHint::Return { value })
}

fn try_extract_print(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "print")?;
    let content = rest.trim();
    
    if content.is_empty() {
        return None;
    }
    
    // Determine if content is a literal or variable expression
    let is_literal = if content.starts_with('"') && content.ends_with('"') {
        // Explicitly quoted - definitely a literal
        true
    } else if content.contains(' ') {
        // Multi-word without quotes - treat as implicit literal (e.g., "hello world")
        true
    } else {
        // Single word - check if it looks like a variable name
        // If it's purely alphanumeric/underscore and starts with letter/underscore, it's a variable
        // Otherwise treat as literal text
        let first_char = content.chars().next().unwrap_or('_');
        let looks_like_identifier = (first_char.is_ascii_alphabetic() || first_char == '_')
            && content.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        !looks_like_identifier
    };
    
    Some(StatementHint::Print {
        content: content.to_string(),
        is_literal,
    })
}

fn try_extract_increment_decrement(line: &str) -> Option<StatementHint> {
    if let Some(rest) = strip_keyword(line, "increment") {
        let target = sanitize_identifier(rest);
        if !target.is_empty() {
            return Some(StatementHint::Modify { target, delta: 1 });
        }
    }
    if let Some(rest) = strip_keyword(line, "decrement") {
        let target = sanitize_identifier(rest);
        if !target.is_empty() {
            return Some(StatementHint::Modify { target, delta: -1 });
        }
    }
    None
}

fn try_extract_arithmetic(line: &str) -> Option<StatementHint> {
    let trimmed = line.trim();

    // Try add
    if let Some(rest) = strip_any_keyword(trimmed, ADD_KEYWORDS) {
        if let Some((left, right, target)) = parse_binary_op(rest, &[" and ", " to "]) {
            return Some(StatementHint::Arithmetic {
                operation: ArithmeticOp::Add,
                left,
                right,
                target,
            });
        }
    }

    // Try subtract
    if let Some(rest) = strip_any_keyword(trimmed, SUBTRACT_KEYWORDS) {
        if let Some((left, right, target)) = parse_binary_op(rest, &[" from ", " and "]) {
            // For subtract, "subtract X from Y" means Y - X
            return Some(StatementHint::Arithmetic {
                operation: ArithmeticOp::Subtract,
                left: right,
                right: left,
                target,
            });
        }
    }

    // Try multiply
    if let Some(rest) = strip_any_keyword(trimmed, MULTIPLY_KEYWORDS) {
        if let Some((left, right, target)) = parse_binary_op(rest, &[" by ", " and "]) {
            return Some(StatementHint::Arithmetic {
                operation: ArithmeticOp::Multiply,
                left,
                right,
                target,
            });
        }
    }

    // Try divide
    if let Some(rest) = strip_any_keyword(trimmed, DIVIDE_KEYWORDS) {
        if let Some((left, right, target)) = parse_binary_op(rest, &[" by ", " into "]) {
            return Some(StatementHint::Arithmetic {
                operation: ArithmeticOp::Divide,
                left,
                right,
                target,
            });
        }
    }

    None
}

fn parse_binary_op(text: &str, separators: &[&str]) -> Option<(String, String, Option<String>)> {
    let lower = text.to_lowercase();
    
    for sep in separators {
        if let Some(idx) = lower.find(sep) {
            let left = text[..idx].trim().to_string();
            let rest = &text[idx + sep.len()..];
            
            // Check for target (store in, into, to, as)
            let (right, target) = extract_target(rest);
            
            if !left.is_empty() && !right.is_empty() {
                return Some((left, right, target));
            }
        }
    }
    None
}

fn extract_target(text: &str) -> (String, Option<String>) {
    let lower = text.to_lowercase();
    for marker in [" into ", " store in ", " to ", " as ", " storing in "] {
        if let Some(idx) = lower.find(marker) {
            let value = text[..idx].trim().to_string();
            let target = text[idx + marker.len()..].trim().to_string();
            if !target.is_empty() {
                return (value, Some(target));
            }
        }
    }
    (text.trim().to_string(), None)
}

fn try_extract_assignment(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "set")?;
    let lower = rest.to_lowercase();
    
    if let Some(idx) = lower.find(" to ") {
        let target = sanitize_identifier(&rest[..idx]);
        let value = rest[idx + 4..].trim().to_string();
        if !target.is_empty() && !value.is_empty() {
            return Some(StatementHint::Assignment { target, value });
        }
    }
    None
}

fn try_extract_declaration(line: &str) -> Option<StatementHint> {
    // Check for list/array keywords first
    let is_array = LIST_KEYWORDS.iter().any(|kw| {
        let lower = line.to_lowercase();
        lower.contains(kw)
    });

    // Try to strip declaration keywords
    let rest = strip_any_keyword(line, DECLARE_KEYWORDS)
        .or_else(|| {
            // Also match lines starting with list keywords
            for kw in LIST_KEYWORDS {
                if let Some(r) = strip_keyword(line, kw) {
                    return Some(r);
                }
            }
            None
        })?;

    // Parse the declaration
    let (names, type_hint, initial_value, array_size) = parse_declaration_parts(rest, is_array);
    
    if names.is_empty() {
        return None;
    }

    Some(StatementHint::Declaration {
        names,
        type_hint,
        initial_value,
        is_array,
        array_size,
    })
}

fn parse_declaration_parts(text: &str, is_array: bool) -> (Vec<String>, Option<String>, Option<String>, Option<String>) {
    let mut names = Vec::new();
    let mut type_hint = None;
    let mut initial_value = None;
    let mut array_size = None;

    // Noise words that are ONLY filtered when used as articles (before another word)
    // NOT filtered when they appear with commas or as standalone identifiers
    let article_noise = ["a", "an", "the"];
    // These are always filtered as they are connecting words
    let connecting_noise = ["of", "with", "named", "called", "as", "that", "is", "be", "to"];
    let type_words = ["int", "integer", "float", "double", "char", "string", "bool", "boolean"];
    
    let tokens: Vec<&str> = text.split_whitespace().collect();
    
    for (i, token) in tokens.iter().enumerate() {
        let clean = token.trim_matches(|c: char| c == ',' || c == ';');
        let lower = clean.to_lowercase();
        
        // Check if this token had a comma (indicating it's part of a list)
        let has_comma = token.contains(',');
        
        // Skip connecting noise words always
        if connecting_noise.contains(&lower.as_str()) {
            continue;
        }
        
        // Skip article noise words ONLY if:
        // - They don't have a comma attached (not part of a list like "a, b")
        // - They are followed by another word (acting as article)
        if article_noise.contains(&lower.as_str()) && !has_comma && i + 1 < tokens.len() {
            continue;
        }
        
        // Skip array keywords (already detected)
        if LIST_KEYWORDS.contains(&lower.as_str()) {
            continue;
        }
        
        // Detect type
        if type_words.contains(&lower.as_str()) {
            type_hint = Some(lower.clone());
            continue;
        }
        
        // Check for numeric value (could be initial value or array size)
        if clean.parse::<f64>().is_ok() {
            if is_array && array_size.is_none() {
                array_size = Some(clean.to_string());
            } else {
                initial_value = Some(clean.to_string());
            }
            continue;
        }
        
        // Otherwise it's likely a variable name
        let ident = sanitize_identifier(clean);
        if !ident.is_empty() {
            names.push(ident);
        }
    }

    (names, type_hint, initial_value, array_size)
}

fn try_extract_loop(line: &str) -> Option<StatementHint> {
    let rest = strip_any_keyword(line, LOOP_KEYWORDS)?;
    
    let lower = rest.to_lowercase();
    
    // Check for range loop (contains "to")
    if lower.contains(" to ") {
        let (iterator, start, end) = parse_range_loop(rest);
        let body_action = extract_loop_action(rest);
        
        return Some(StatementHint::Loop {
            iterator: Some(iterator),
            start: Some(start),
            end: Some(end),
            collection: None,
            body_action,
        });
    }
    
    // Check for collection loop (contains "in" or "over")
    for marker in [" in ", " over "] {
        if let Some(idx) = lower.find(marker) {
            let before = rest[..idx].trim();
            let after = rest[idx + marker.len()..].trim();
            
            let iterator = if before.is_empty() { "item".to_string() } else { sanitize_identifier(before) };
            let collection = sanitize_identifier(after.split_whitespace().next().unwrap_or(""));
            
            return Some(StatementHint::Loop {
                iterator: Some(iterator),
                start: None,
                end: None,
                collection: Some(collection),
                body_action: None,
            });
        }
    }
    
    // Generic loop without clear structure
    Some(StatementHint::Loop {
        iterator: Some("i".to_string()),
        start: None,
        end: None,
        collection: None,
        body_action: None,
    })
}

fn parse_range_loop(text: &str) -> (String, String, String) {
    let lower = text.to_lowercase();
    let mut iterator = "i".to_string();
    let mut start = "0".to_string();
    let mut end = "10".to_string();
    
    // Try to find "from X to Y" pattern
    if let Some(from_idx) = lower.find("from ") {
        let after_from = &text[from_idx + 5..];
        if let Some(to_idx) = after_from.to_lowercase().find(" to ") {
            start = after_from[..to_idx].trim().to_string();
            let after_to = &after_from[to_idx + 4..];
            // End is everything until the next keyword or end
            end = after_to.split_whitespace().next().unwrap_or("10").to_string();
        }
        
        // Iterator is before "from"
        let before_from = text[..from_idx].trim();
        if !before_from.is_empty() {
            iterator = sanitize_identifier(before_from);
        }
    } else if let Some(to_idx) = lower.find(" to ") {
        // Just "X to Y" pattern
        let before = text[..to_idx].trim();
        let after = &text[to_idx + 4..];
        
        // Try to parse start
        let tokens: Vec<&str> = before.split_whitespace().collect();
        if let Some(last) = tokens.last() {
            if last.parse::<i32>().is_ok() {
                start = last.to_string();
                if tokens.len() > 1 {
                    iterator = sanitize_identifier(tokens[0]);
                }
            } else {
                iterator = sanitize_identifier(last);
            }
        }
        
        end = after.split_whitespace().next().unwrap_or("10").to_string();
    }
    
    (iterator, start, end)
}

fn extract_loop_action(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    for marker in ["printing ", "print "] {
        if let Some(idx) = lower.find(marker) {
            let action = text[idx + marker.len()..].trim();
            if !action.is_empty() {
                return Some(format!("print {}", action));
            }
        }
    }
    None
}

fn try_extract_conditional(line: &str) -> Option<StatementHint> {
    let rest = strip_any_keyword(line, IF_KEYWORDS)?;
    
    let (condition, then_action, else_action) = parse_conditional_parts(rest);
    
    Some(StatementHint::Conditional {
        condition,
        then_action,
        else_action,
        is_else_if: false,
    })
}

fn parse_conditional_parts(text: &str) -> (String, Option<String>, Option<String>) {
    let mut main = text;
    let mut else_action = None;
    let mut then_action = None;
    
    let lower = text.to_lowercase();
    
    // Extract else action
    for marker in [" otherwise ", " else "] {
        if let Some(idx) = lower.find(marker) {
            else_action = Some(text[idx + marker.len()..].trim().to_string());
            main = &text[..idx];
            break;
        }
    }
    
    // Extract then action
    let main_lower = main.to_lowercase();
    for marker in [" then ", " do "] {
        if let Some(idx) = main_lower.find(marker) {
            then_action = Some(main[idx + marker.len()..].trim().to_string());
            main = &main[..idx];
            break;
        }
    }
    
    let condition = normalize_condition(main.trim());
    (condition, then_action, else_action)
}

fn try_extract_function(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "function")
        .or_else(|| strip_keyword(line, "create function"))
        .or_else(|| strip_keyword(line, "make function"))
        .or_else(|| strip_keyword(line, "define function"))?;
    
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    
    let name = sanitize_identifier(tokens[0]);
    if name.is_empty() {
        return None;
    }
    
    // Try to parse parameters and return type
    let (parameters, return_type) = parse_function_signature(&tokens[1..]);
    
    Some(StatementHint::FunctionDef {
        name,
        parameters,
        return_type,
    })
}

fn parse_function_signature(tokens: &[&str]) -> (Vec<(String, String)>, Option<String>) {
    let mut parameters = Vec::new();
    let mut return_type = None;
    
    let type_map = [
        ("int", "int"), ("integer", "int"), ("integers", "int"),
        ("float", "float"), ("floats", "float"),
        ("double", "double"), ("doubles", "double"),
        ("char", "char"), ("chars", "char"),
        ("string", "char *"), ("strings", "char *"),
        ("bool", "bool"), ("boolean", "bool"),
        ("void", "void"),
    ];
    
    let noise = ["taking", "with", "and", "parameters", "parameter", "returning", "returns"];
    
    let mut i = 0;
    let mut in_return = false;
    
    while i < tokens.len() {
        let token = tokens[i].trim_matches(|c: char| c == ',' || c == ';');
        let lower = token.to_lowercase();
        
        if lower == "returning" || lower == "returns" {
            in_return = true;
            i += 1;
            continue;
        }
        
        if noise.contains(&lower.as_str()) {
            i += 1;
            continue;
        }
        
        // Check if it's a type
        if let Some((_, mapped)) = type_map.iter().find(|(k, _)| *k == lower.as_str()) {
            if in_return {
                return_type = Some(mapped.to_string());
            } else if i + 1 < tokens.len() {
                // Next token should be the parameter name
                let name = sanitize_identifier(tokens[i + 1]);
                if !name.is_empty() {
                    parameters.push((mapped.to_string(), name));
                    i += 1;
                }
            }
        }
        
        i += 1;
    }
    
    (parameters, return_type)
}

fn try_extract_struct(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "struct")?;
    
    let lower = rest.to_lowercase();
    let mut name_part = rest;
    let mut fields_part = "";
    
    for marker in [" with ", " having ", " containing "] {
        if let Some(idx) = lower.find(marker) {
            name_part = &rest[..idx];
            fields_part = &rest[idx + marker.len()..];
            break;
        }
    }
    
    let name = sanitize_identifier(name_part.trim());
    if name.is_empty() {
        return None;
    }
    
    let fields = parse_struct_fields(fields_part);
    
    Some(StatementHint::StructDef { name, fields })
}

fn parse_struct_fields(text: &str) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    
    let type_map = [
        ("int", "int"), ("integer", "int"),
        ("float", "float"), ("double", "double"),
        ("char", "char"), ("string", "char *"),
        ("bool", "bool"),
    ];
    
    for chunk in text.split(|c| c == ',' || c == ';').flat_map(|s| s.split(" and ")) {
        let tokens: Vec<&str> = chunk.split_whitespace().collect();
        
        let mut field_type = "int";
        let mut field_name = "";
        
        for token in tokens.iter().rev() {
            let clean = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
            let lower = clean.to_lowercase();
            
            if let Some((_, mapped)) = type_map.iter().find(|(k, _)| *k == lower.as_str()) {
                field_type = mapped;
            } else if !clean.is_empty() {
                field_name = clean;
                break;
            }
        }
        
        let name = sanitize_identifier(field_name);
        if !name.is_empty() {
            fields.push((field_type.to_string(), name));
        }
    }
    
    fields
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
        .replace(" is not ", " != ")
        .replace(" is ", " == ")
}

// ============================================================================
// Context Analysis - Variable Declaration Checking
// ============================================================================

/// Result of context analysis for a hint.
#[derive(Debug, Clone)]
pub struct ContextAnalysis {
    /// Warning messages (non-blocking)
    pub warnings: Vec<String>,
    /// Variables that appear to be used but not declared
    pub undeclared_vars: Vec<String>,
}

impl ContextAnalysis {
    pub fn empty() -> Self {
        Self {
            warnings: Vec::new(),
            undeclared_vars: Vec::new(),
        }
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// Extract declared variable names from preceding code.
/// This is a simple heuristic that looks for common declaration patterns.
pub fn extract_declared_variables(code_before: &str) -> Vec<String> {
    let mut declared = Vec::new();
    
    for line in code_before.lines() {
        let trimmed = line.trim();
        
        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }
        
        // Look for C-style declarations: "type name" or "type name = value"
        // Pattern: int x; int x = 5; int x, y, z;
        for type_keyword in ["int", "float", "double", "char", "bool", "long", "short", "unsigned"] {
            if trimmed.starts_with(type_keyword) && trimmed.len() > type_keyword.len() {
                let rest = &trimmed[type_keyword.len()..];
                // Extract identifiers after the type
                for part in rest.split(',') {
                    let part = part.trim().trim_end_matches(';');
                    // Handle "x = 5" or just "x"
                    let name = part.split('=').next().unwrap_or("").trim();
                    // Handle arrays like "x[10]"
                    let name = name.split('[').next().unwrap_or(name).trim();
                    // Handle pointers like "*x"
                    let name = name.trim_start_matches('*').trim();
                    if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                        declared.push(name.to_string());
                    }
                }
            }
        }
        
        // Look for Python-style assignments: "x = value"
        if trimmed.contains('=') && !trimmed.contains("==") && !trimmed.contains("!=") {
            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            if parts.len() == 2 {
                let lhs = parts[0].trim();
                // Simple identifier on left side
                if lhs.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    declared.push(lhs.to_string());
                }
            }
        }
    }
    
    declared
}

/// Analyze a hint in context to produce warnings.
/// This checks if variables used in the hint have been declared.
pub fn analyze_context(hint: &StatementHint, code_before: &str) -> ContextAnalysis {
    let declared = extract_declared_variables(code_before);
    let mut analysis = ContextAnalysis::empty();
    
    // Get variables used by this hint
    let used_vars = match hint {
        StatementHint::Assignment { target, .. } => vec![target.clone()],
        StatementHint::Modify { target, .. } => vec![target.clone()],
        StatementHint::Print { content, is_literal } => {
            if *is_literal {
                vec![]
            } else {
                vec![content.clone()]
            }
        }
        StatementHint::Conditional { condition, .. } => {
            // Extract identifiers from condition
            extract_identifiers_from_expression(condition)
        }
        _ => vec![],
    };
    
    // Check which used variables are not declared
    for var in used_vars {
        let var_lower = var.to_lowercase();
        // Skip obvious non-variables (numbers, known constants)
        if var.parse::<f64>().is_ok() {
            continue;
        }
        if ["true", "false", "null", "nullptr", "none"].contains(&var_lower.as_str()) {
            continue;
        }
        // Check if declared
        if !declared.iter().any(|d| d == &var) {
            analysis.undeclared_vars.push(var.clone());
            analysis.warnings.push(format!(
                "Variable '{}' may not be declared. Consider adding: declare {}",
                var, var
            ));
        }
    }
    
    analysis
}

/// Extract identifier-like tokens from an expression string.
fn extract_identifiers_from_expression(expr: &str) -> Vec<String> {
    let mut identifiers = Vec::new();
    let mut current = String::new();
    
    for ch in expr.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch);
        } else {
            if !current.is_empty() {
                // Check if it looks like an identifier (not a number)
                if current.chars().next().map(|c| c.is_ascii_alphabetic() || c == '_').unwrap_or(false) {
                    identifiers.push(current.clone());
                }
                current.clear();
            }
        }
    }
    
    if !current.is_empty() && current.chars().next().map(|c| c.is_ascii_alphabetic() || c == '_').unwrap_or(false) {
        identifiers.push(current);
    }
    
    identifiers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(line: &str) -> TranslateLineRequest {
        TranslateLineRequest {
            english_line: line.to_string(),
            code_before: String::new(),
            code_after: String::new(),
            language: "c".to_string(),
            line_index: 0,
            api_key: None,
            model: None,
            max_lines: None,
        }
    }

    #[test]
    fn test_declaration() {
        let hint = extract(&make_request("declare x"));
        assert!(matches!(hint, StatementHint::Declaration { .. }));
    }

    #[test]
    fn test_assignment() {
        let hint = extract(&make_request("set x to 5"));
        assert!(matches!(hint, StatementHint::Assignment { target, value } if target == "x" && value == "5"));
    }

    #[test]
    fn test_loop() {
        let hint = extract(&make_request("loop from 0 to 10"));
        assert!(matches!(hint, StatementHint::Loop { .. }));
    }

    #[test]
    fn test_conditional() {
        let hint = extract(&make_request("if x > 5"));
        assert!(matches!(hint, StatementHint::Conditional { .. }));
    }

    #[test]
    fn test_print() {
        let hint = extract(&make_request("print hello world"));
        assert!(matches!(hint, StatementHint::Print { .. }));
    }
}

