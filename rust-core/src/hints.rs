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

    /// Assignment: "set x to 5", "set a, b, c 0"
    /// Can handle multiple targets with the same value
    Assignment {
        targets: Vec<String>,
        value: String,
    },

    /// Loop construct: "loop from 0 to 10", "for i 0 to n"
    Loop {
        iterator: Option<String>,
        start: Option<String>,
        end: Option<String>,
        collection: Option<String>,
        body_action: Option<String>,
    },

    /// While loop: "while i < n do ..."
    While {
        condition: String,
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

    /// Read input: "read n", "input x"
    Read {
        variables: Vec<String>,
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

    /// Switch statement: "switch a"
    Switch {
        expression: String,
    },

    /// Case in switch: "case 1", "case 2 do X"
    Case {
        value: String,
        action: Option<String>,
    },

    /// Default case: "default", "default do X"
    Default {
        action: Option<String>,
    },

    /// Do-while loop start: "do"
    DoWhileStart,

    /// Do-while loop end: "end do while condition"
    DoWhileEnd {
        condition: String,
    },

    /// Break statement: "break", "break out"
    Break,

    /// Continue statement: "continue", "skip iteration"
    Continue,

    /// Function call: "call foo with x, y"
    FunctionCall {
        name: String,
        arguments: Vec<String>,
    },

    /// Include directive: "include stdio"
    Include {
        header: String,
        is_system: bool,
    },

    /// Define macro: "define MAX 100"
    Define {
        name: String,
        value: String,
    },

    /// Pointer declaration: "pointer to int x"
    PointerDecl {
        base_type: String,
        name: String,
    },

    /// Dereference: "dereference p"
    Dereference {
        target: String,
    },

    /// Address of: "address of x"
    AddressOf {
        target: String,
    },

    /// Malloc: "allocate n integers"
    Malloc {
        count: String,
        element_type: String,
    },

    /// Free: "free p"
    Free {
        target: String,
    },

    /// Enum definition: "enum Color with red, green, blue"
    EnumDef {
        name: String,
        values: Vec<String>,
    },

    /// Typedef: "typedef int Number"
    Typedef {
        original_type: String,
        new_name: String,
    },

    /// String declaration: "string s equals hello"
    StringDecl {
        name: String,
        initial_value: Option<String>,
        size: Option<String>,
    },

    /// Comment: "comment this does X"
    Comment {
        text: String,
        is_block: bool,
    },

    /// Bitwise operation: "x bitwise and y"
    Bitwise {
        operation: BitwiseOp,
        left: String,
        right: Option<String>,
        target: Option<String>,
    },

    /// Type cast: "cast x to int"
    Cast {
        expression: String,
        target_type: String,
    },

    /// Array access: "get a at i", "set a at 0 to 5"
    ArrayAccess {
        array: String,
        index: String,
        value: Option<String>, // None for read, Some for write
    },

    /// Sizeof: "size of a"
    SizeOf {
        target: String,
    },

    /// Ternary: "if x then y else z" (inline)
    Ternary {
        condition: String,
        true_value: String,
        false_value: String,
    },

    /// Unknown - AI handles without specific hints
    Unknown {
        original: String,
    },
}

/// Bitwise operation types.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BitwiseOp {
    And,
    Or,
    Xor,
    Not,
    ShiftLeft,
    ShiftRight,
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
            StatementHint::Read { .. } => true,
            // Control flow markers
            StatementHint::Else => true,
            StatementHint::EndBlock => true,
            StatementHint::MainFunction => true,
            // Simple conditionals (rule-based generates complete blocks)
            StatementHint::Conditional { .. } => true,
            // While loops
            StatementHint::While { .. } => true,
            // Simple loops (rule-based generates complete blocks with braces)
            StatementHint::Loop { .. } => true,
            // New control flow
            StatementHint::Switch { .. } => true,
            StatementHint::Case { .. } => true,
            StatementHint::Default { .. } => true,
            StatementHint::DoWhileStart => true,
            StatementHint::DoWhileEnd { .. } => true,
            StatementHint::Break => true,
            StatementHint::Continue => true,
            // Function calls
            StatementHint::FunctionCall { .. } => true,
            // Preprocessor
            StatementHint::Include { .. } => true,
            StatementHint::Define { .. } => true,
            // Pointers and memory
            StatementHint::PointerDecl { .. } => true,
            StatementHint::Dereference { .. } => true,
            StatementHint::AddressOf { .. } => true,
            StatementHint::Malloc { .. } => true,
            StatementHint::Free { .. } => true,
            // Types
            StatementHint::EnumDef { .. } => true,
            StatementHint::Typedef { .. } => true,
            StatementHint::StringDecl { .. } => true,
            // Comments
            StatementHint::Comment { .. } => true,
            // Operations
            StatementHint::Bitwise { .. } => true,
            StatementHint::Cast { .. } => true,
            StatementHint::ArrayAccess { .. } => true,
            StatementHint::SizeOf { .. } => true,
            StatementHint::Ternary { .. } => true,
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

            StatementHint::Assignment { targets, value } => {
                format!("- Intent: Assignment\n- Targets: {}\n- Value: {}", targets.join(", "), value)
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

            StatementHint::While { condition, body_action } => {
                let mut parts = vec![
                    "- Intent: While loop".to_string(),
                    format!("- Condition: {}", condition),
                ];
                if let Some(action) = body_action {
                    parts.push(format!("- Body action: {}", action));
                }
                parts.join("\n")
            }

            StatementHint::Read { variables } => {
                format!("- Intent: Read input\n- Variables: {}", variables.join(", "))
            }

            StatementHint::Switch { expression } => {
                format!("- Intent: Switch statement\n- Expression: {}", expression)
            }

            StatementHint::Case { value, action } => {
                let mut s = format!("- Intent: Case\n- Value: {}", value);
                if let Some(a) = action {
                    s.push_str(&format!("\n- Action: {}", a));
                }
                s
            }

            StatementHint::Default { action } => {
                let mut s = "- Intent: Default case".to_string();
                if let Some(a) = action {
                    s.push_str(&format!("\n- Action: {}", a));
                }
                s
            }

            StatementHint::DoWhileStart => "- Intent: Do-while loop start".to_string(),

            StatementHint::DoWhileEnd { condition } => {
                format!("- Intent: Do-while loop end\n- Condition: {}", condition)
            }

            StatementHint::Break => "- Intent: Break statement".to_string(),

            StatementHint::Continue => "- Intent: Continue statement".to_string(),

            StatementHint::FunctionCall { name, arguments } => {
                if arguments.is_empty() {
                    format!("- Intent: Function call\n- Name: {}", name)
                } else {
                    format!("- Intent: Function call\n- Name: {}\n- Arguments: {}", name, arguments.join(", "))
                }
            }

            StatementHint::Include { header, is_system } => {
                format!("- Intent: Include\n- Header: {}\n- System header: {}", header, is_system)
            }

            StatementHint::Define { name, value } => {
                format!("- Intent: Define macro\n- Name: {}\n- Value: {}", name, value)
            }

            StatementHint::PointerDecl { base_type, name } => {
                format!("- Intent: Pointer declaration\n- Type: {} *\n- Name: {}", base_type, name)
            }

            StatementHint::Dereference { target } => {
                format!("- Intent: Dereference\n- Target: {}", target)
            }

            StatementHint::AddressOf { target } => {
                format!("- Intent: Address of\n- Target: {}", target)
            }

            StatementHint::Malloc { count, element_type } => {
                format!("- Intent: Allocate memory\n- Count: {}\n- Type: {}", count, element_type)
            }

            StatementHint::Free { target } => {
                format!("- Intent: Free memory\n- Target: {}", target)
            }

            StatementHint::EnumDef { name, values } => {
                format!("- Intent: Enum definition\n- Name: {}\n- Values: {}", name, values.join(", "))
            }

            StatementHint::Typedef { original_type, new_name } => {
                format!("- Intent: Typedef\n- Original: {}\n- New name: {}", original_type, new_name)
            }

            StatementHint::StringDecl { name, initial_value, size } => {
                let mut s = format!("- Intent: String declaration\n- Name: {}", name);
                if let Some(v) = initial_value {
                    s.push_str(&format!("\n- Initial value: {}", v));
                }
                if let Some(sz) = size {
                    s.push_str(&format!("\n- Size: {}", sz));
                }
                s
            }

            StatementHint::Comment { text, is_block } => {
                format!("- Intent: Comment\n- Text: {}\n- Block: {}", text, is_block)
            }

            StatementHint::Bitwise { operation, left, right, target } => {
                let op_str = match operation {
                    BitwiseOp::And => "AND",
                    BitwiseOp::Or => "OR",
                    BitwiseOp::Xor => "XOR",
                    BitwiseOp::Not => "NOT",
                    BitwiseOp::ShiftLeft => "Shift Left",
                    BitwiseOp::ShiftRight => "Shift Right",
                };
                let mut s = format!("- Intent: Bitwise {}\n- Left: {}", op_str, left);
                if let Some(r) = right {
                    s.push_str(&format!("\n- Right: {}", r));
                }
                if let Some(t) = target {
                    s.push_str(&format!("\n- Store in: {}", t));
                }
                s
            }

            StatementHint::Cast { expression, target_type } => {
                format!("- Intent: Type cast\n- Expression: {}\n- To type: {}", expression, target_type)
            }

            StatementHint::ArrayAccess { array, index, value } => {
                let mut s = format!("- Intent: Array access\n- Array: {}\n- Index: {}", array, index);
                if let Some(v) = value {
                    s.push_str(&format!("\n- Set value: {}", v));
                }
                s
            }

            StatementHint::SizeOf { target } => {
                format!("- Intent: Size of\n- Target: {}", target)
            }

            StatementHint::Ternary { condition, true_value, false_value } => {
                format!("- Intent: Ternary\n- Condition: {}\n- True: {}\n- False: {}", condition, true_value, false_value)
            }

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
    "loop", "for loop", "for each", "foreach", "for every", "for all", "for",
    "while",  // "while" with range syntax like "while i 0 to 10" becomes a for loop
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

    if let Some(hint) = try_extract_read(line) {
        return hint;
    }

    if let Some(hint) = try_extract_while(line) {
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

    // New extractors
    if let Some(hint) = try_extract_switch(line) {
        return hint;
    }

    if let Some(hint) = try_extract_case(line) {
        return hint;
    }

    if let Some(hint) = try_extract_default(line) {
        return hint;
    }

    if let Some(hint) = try_extract_do_while(line) {
        return hint;
    }

    if let Some(hint) = try_extract_break_continue(line) {
        return hint;
    }

    if let Some(hint) = try_extract_function_call(line) {
        return hint;
    }

    if let Some(hint) = try_extract_include(line) {
        return hint;
    }

    if let Some(hint) = try_extract_define(line) {
        return hint;
    }

    if let Some(hint) = try_extract_pointer(line) {
        return hint;
    }

    if let Some(hint) = try_extract_memory_ops(line) {
        return hint;
    }

    if let Some(hint) = try_extract_enum(line) {
        return hint;
    }

    if let Some(hint) = try_extract_typedef(line) {
        return hint;
    }

    if let Some(hint) = try_extract_string_decl(line) {
        return hint;
    }

    if let Some(hint) = try_extract_comment(line) {
        return hint;
    }

    if let Some(hint) = try_extract_bitwise(line) {
        return hint;
    }

    if let Some(hint) = try_extract_cast(line) {
        return hint;
    }

    if let Some(hint) = try_extract_array_access(line) {
        return hint;
    }

    if let Some(hint) = try_extract_sizeof(line) {
        return hint;
    }

    if let Some(hint) = try_extract_ternary(line) {
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
    
    // First try with explicit separators like "and", "to", etc.
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
    
    // Fallback: handle "add a b" pattern (space-separated operands)
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.len() >= 2 {
        let left = tokens[0].to_string();
        let (right, target) = extract_target(&tokens[1..].join(" "));
        if !left.is_empty() && !right.is_empty() {
            return Some((left, right, target));
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
    // Try "set" or "make" as assignment keywords
    let rest = strip_keyword(line, "set")
        .or_else(|| strip_keyword(line, "make"))?;
    
    // Skip if this looks like "make function" or "make main"
    let rest_lower = rest.to_lowercase();
    if rest_lower.starts_with("function") || rest_lower.starts_with("main") {
        return None;
    }
    
    // Try to parse: "target(s) [to] value"
    // Patterns:
    //   set a 5
    //   set a to 5
    //   set a, b, c 0
    //   set a, b 0
    //   set total a + b * 3
    
    let lower = rest.to_lowercase();
    
    // Check for "to" separator
    if let Some(to_idx) = lower.find(" to ") {
        let targets_part = &rest[..to_idx];
        let value = rest[to_idx + 4..].trim().to_string();
        
        let targets: Vec<String> = targets_part
            .split(',')
            .map(|s| sanitize_identifier(s.trim()))
            .filter(|s| !s.is_empty())
            .collect();
        
        if !targets.is_empty() && !value.is_empty() {
            return Some(StatementHint::Assignment { targets, value });
        }
    }
    
    // No "to" - parse as "targets value" or "targets expression"
    // Split on last contiguous identifier/expression
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    
    // Find where targets end and value begins
    // Targets are comma-separated identifiers at the start
    let mut target_end_idx = 0;
    let mut targets = Vec::new();
    
    for (i, token) in tokens.iter().enumerate() {
        let clean = token.trim_matches(',');
        // If it's a valid identifier (possibly with comma), it's a target
        if is_valid_identifier_token(clean) {
            targets.push(sanitize_identifier(clean));
            target_end_idx = i + 1;
            // If this token doesn't have a trailing comma and next exists, 
            // the rest might be the value
            if !token.ends_with(',') && i + 1 < tokens.len() {
                break;
            }
        } else {
            break;
        }
    }
    
    if targets.is_empty() || target_end_idx >= tokens.len() {
        return None;
    }
    
    // Everything after targets is the value
    let value = tokens[target_end_idx..].join(" ");
    
    if !value.is_empty() {
        return Some(StatementHint::Assignment { targets, value });
    }
    
    None
}

fn is_valid_identifier_token(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let first = s.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn try_extract_while(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "while")?;
    
    let lower = rest.to_lowercase();
    
    // Check if this is actually a for-loop style syntax: "while i 0 to 10"
    // If it contains " to " with numbers, treat it as a for loop instead
    if lower.contains(" to ") {
        // Check if the pattern looks like "iterator start to end"
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 4 {
            // Check if second token is a number or second-to-last is "to"
            if tokens.iter().any(|t| t.parse::<f64>().is_ok()) {
                // This looks like a for loop, not a while loop
                // Return None to let try_extract_loop handle it
                return None;
            }
        }
    }
    
    // Find "do" separator for body action
    let (condition_part, body_action) = if let Some(do_idx) = lower.find(" do ") {
        let cond = rest[..do_idx].trim();
        let action = rest[do_idx + 4..].trim();
        (cond, if action.is_empty() { None } else { Some(action.to_string()) })
    } else {
        (rest.trim(), None)
    };
    
    let condition = normalize_condition(condition_part);
    
    Some(StatementHint::While { condition, body_action })
}

fn try_extract_read(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "read")
        .or_else(|| strip_keyword(line, "input"))?;
    
    let variables: Vec<String> = rest
        .split(',')
        .map(|s| sanitize_identifier(s.trim()))
        .filter(|s| !s.is_empty())
        .collect();
    
    if variables.is_empty() {
        return None;
    }
    
    Some(StatementHint::Read { variables })
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
    
    // Try to find "from X to Y" pattern: "i from 0 to 10"
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
        // Pattern without "from": could be "i 0 to 10" or "0 to 10"
        let before = text[..to_idx].trim();
        let after = &text[to_idx + 4..];
        
        let tokens: Vec<&str> = before.split_whitespace().collect();
        
        match tokens.len() {
            0 => {}
            1 => {
                // Just "0 to 10" - no iterator, first token is start
                let tok = tokens[0];
                if tok.parse::<f64>().is_ok() {
                    start = tok.to_string();
                } else {
                    // It's an identifier, treat as iterator
                    iterator = sanitize_identifier(tok);
                }
            }
            2 => {
                // "i 0 to 10" - iterator and start
                let first = tokens[0];
                let second = tokens[1];
                
                if second.parse::<f64>().is_ok() || is_expression(second) {
                    // First is iterator, second is start
                    iterator = sanitize_identifier(first);
                    start = second.to_string();
                } else {
                    // Fallback: first is iterator, second might be expression
                    iterator = sanitize_identifier(first);
                    start = second.to_string();
                }
            }
            _ => {
                // Multiple tokens before "to"
                // First token is likely iterator, rest is start expression
                iterator = sanitize_identifier(tokens[0]);
                start = tokens[1..].join(" ");
            }
        }
        
        // Parse end - could be a single value or expression like "n-1" or "n - 1"
        let end_str = after.trim();
        // Take until we hit a loop action keyword
        let end_tokens: Vec<&str> = end_str.split_whitespace().collect();
        let mut end_parts = Vec::new();
        for tok in end_tokens {
            let tok_lower = tok.to_lowercase();
            if ["do", "print", "printing", "then"].contains(&tok_lower.as_str()) {
                break;
            }
            end_parts.push(tok);
        }
        if !end_parts.is_empty() {
            end = end_parts.join(" ");
        }
    }
    
    (iterator, start, end)
}

fn is_expression(s: &str) -> bool {
    s.chars().any(|c| matches!(c, '+' | '-' | '*' | '/' | '(' | ')'))
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
    
    // Extract then action - try explicit separators first
    let main_lower = main.to_lowercase();
    for marker in [" then ", " do "] {
        if let Some(idx) = main_lower.find(marker) {
            then_action = Some(main[idx + marker.len()..].trim().to_string());
            main = &main[..idx];
            break;
        }
    }
    
    // If no explicit separator, look for action keywords after a comparison
    // Pattern: "a <= b print a" -> condition="a <= b", then_action="print a"
    if then_action.is_none() {
        if let Some((cond, action)) = split_condition_from_action(main) {
            main = cond;
            then_action = Some(action.to_string());
        }
    }
    
    let condition = normalize_condition(main.trim());
    (condition, then_action, else_action)
}

/// Action keywords that indicate the start of an inline action
const ACTION_KEYWORDS: &[&str] = &[
    "print", "return", "set", "make", "declare", "increment", "decrement",
    "add", "subtract", "multiply", "divide", "call", "break", "continue",
];

/// Split a condition from an inline action when no explicit separator exists
/// Example: "a <= b print a" -> ("a <= b", "print a")
fn split_condition_from_action(text: &str) -> Option<(&str, &str)> {
    let lower = text.to_lowercase();
    
    // Find the earliest action keyword that appears after a comparison operator
    let comparison_ops = [" <= ", " >= ", " < ", " > ", " == ", " != ", 
                          " equals ", " is ", " not "];
    
    // First, check if there's a comparison operator
    let has_comparison = comparison_ops.iter().any(|op| lower.contains(op));
    if !has_comparison {
        return None;
    }
    
    // Find the position after the comparison
    let mut comparison_end = 0;
    for op in comparison_ops {
        if let Some(idx) = lower.find(op) {
            let end_pos = idx + op.len();
            // Skip past the operand after the comparison
            let rest = &text[end_pos..];
            let operand_end = rest.find(' ').unwrap_or(rest.len());
            let pos = end_pos + operand_end;
            if pos > comparison_end {
                comparison_end = pos;
            }
        }
    }
    
    if comparison_end == 0 {
        return None;
    }
    
    // Now look for action keywords in the remaining text
    let remaining = &text[comparison_end..];
    let remaining_lower = remaining.to_lowercase();
    
    for keyword in ACTION_KEYWORDS {
        // Look for the keyword as a word boundary
        let pattern = format!(" {}", keyword);
        if let Some(idx) = remaining_lower.find(&pattern) {
            let split_point = comparison_end + idx;
            let condition = text[..split_point].trim();
            let action = text[split_point..].trim();
            if !condition.is_empty() && !action.is_empty() {
                return Some((condition, action));
            }
        }
        // Also check if it starts with the keyword
        if remaining_lower.trim_start().starts_with(keyword) {
            let condition = text[..comparison_end].trim();
            let action = remaining.trim();
            if !condition.is_empty() && !action.is_empty() {
                return Some((condition, action));
            }
        }
    }
    
    None
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
// New Extractors
// ============================================================================

fn try_extract_switch(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "switch")?;
    let expression = rest.trim().to_string();
    if expression.is_empty() {
        return None;
    }
    Some(StatementHint::Switch { expression })
}

fn try_extract_case(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "case")?;
    let lower = rest.to_lowercase();
    
    // Check for "do" action
    let (value_part, action) = if let Some(idx) = lower.find(" do ") {
        (&rest[..idx], Some(rest[idx + 4..].trim().to_string()))
    } else {
        (rest, None)
    };
    
    let value = value_part.trim().to_string();
    if value.is_empty() {
        return None;
    }
    
    Some(StatementHint::Case { value, action })
}

fn try_extract_default(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    if !lower.starts_with("default") {
        return None;
    }
    
    let rest = &line.trim()[7..]; // Skip "default"
    let action = if rest.is_empty() {
        None
    } else if rest.to_lowercase().starts_with(" do ") {
        Some(rest[4..].trim().to_string())
    } else {
        None
    };
    
    Some(StatementHint::Default { action })
}

fn try_extract_do_while(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    
    // "do" alone starts a do-while
    if lower == "do" {
        return Some(StatementHint::DoWhileStart);
    }
    
    // "end do while condition" ends it
    if lower.starts_with("end do while ") {
        let condition = normalize_condition(&line.trim()[13..]);
        return Some(StatementHint::DoWhileEnd { condition });
    }
    
    None
}

fn try_extract_break_continue(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    
    if lower == "break" || lower == "break out" || lower.starts_with("break out of") {
        return Some(StatementHint::Break);
    }
    
    if lower == "continue" || lower == "skip iteration" || lower == "next iteration" {
        return Some(StatementHint::Continue);
    }
    
    None
}

fn try_extract_function_call(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "call")?;
    
    let lower = rest.to_lowercase();
    let (name_part, args_part) = if let Some(idx) = lower.find(" with ") {
        (&rest[..idx], Some(&rest[idx + 6..]))
    } else {
        (rest, None)
    };
    
    let name = sanitize_identifier(name_part.trim());
    if name.is_empty() {
        return None;
    }
    
    let arguments = if let Some(args) = args_part {
        args.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        Vec::new()
    };
    
    Some(StatementHint::FunctionCall { name, arguments })
}

fn try_extract_include(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "include")?;
    let header = rest.trim();
    
    if header.is_empty() {
        return None;
    }
    
    // Check if user specified quotes (local include)
    let (header, is_system) = if header.starts_with('"') && header.ends_with('"') {
        (header[1..header.len()-1].to_string(), false)
    } else {
        // Map common header names
        let mapped = match header.to_lowercase().as_str() {
            "stdio" => "stdio.h",
            "stdlib" => "stdlib.h",
            "string" => "string.h",
            "math" => "math.h",
            "stdbool" => "stdbool.h",
            "stdint" => "stdint.h",
            "ctype" => "ctype.h",
            "time" => "time.h",
            "assert" => "assert.h",
            _ => header,
        };
        (mapped.to_string(), true)
    };
    
    Some(StatementHint::Include { header, is_system })
}

fn try_extract_define(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "define")?;
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    
    if tokens.len() < 2 {
        return None;
    }
    
    let name = tokens[0].to_string();
    let value = tokens[1..].join(" ");
    
    Some(StatementHint::Define { name, value })
}

fn try_extract_pointer(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    
    // "pointer to int x" or "int pointer x"
    if let Some(rest) = strip_keyword(line, "pointer to") {
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 2 {
            let base_type = tokens[0].to_string();
            let name = sanitize_identifier(tokens[1]);
            if !name.is_empty() {
                return Some(StatementHint::PointerDecl { base_type, name });
            }
        }
    }
    
    // "int pointer p" pattern
    for ty in ["int", "float", "double", "char", "void"] {
        if lower.starts_with(&format!("{} pointer ", ty)) {
            let rest = &line.trim()[ty.len() + 9..];
            let name = sanitize_identifier(rest.split_whitespace().next().unwrap_or(""));
            if !name.is_empty() {
                return Some(StatementHint::PointerDecl { base_type: ty.to_string(), name });
            }
        }
    }
    
    // "dereference p"
    if let Some(rest) = strip_keyword(line, "dereference") {
        let target = sanitize_identifier(rest.trim());
        if !target.is_empty() {
            return Some(StatementHint::Dereference { target });
        }
    }
    
    // "address of x"
    if let Some(rest) = strip_keyword(line, "address of") {
        let target = sanitize_identifier(rest.trim());
        if !target.is_empty() {
            return Some(StatementHint::AddressOf { target });
        }
    }
    
    None
}

fn try_extract_memory_ops(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    
    // "free p"
    if let Some(rest) = strip_keyword(line, "free") {
        let target = sanitize_identifier(rest.trim());
        if !target.is_empty() {
            return Some(StatementHint::Free { target });
        }
    }
    
    // "allocate n integers" or "allocate memory for x"
    if let Some(rest) = strip_keyword(line, "allocate") {
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 2 {
            let count = tokens[0].to_string();
            let element_type = tokens[1].trim_end_matches('s').to_string(); // "integers" -> "integer"
            return Some(StatementHint::Malloc { count, element_type });
        }
    }
    
    // "malloc n integers"
    if let Some(rest) = strip_keyword(line, "malloc") {
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if !tokens.is_empty() {
            let count = tokens[0].to_string();
            let element_type = if tokens.len() > 1 { tokens[1].to_string() } else { "int".to_string() };
            return Some(StatementHint::Malloc { count, element_type });
        }
    }
    
    None
}

fn try_extract_enum(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "enum")?;
    
    let lower = rest.to_lowercase();
    let (name_part, values_part) = if let Some(idx) = lower.find(" with ") {
        (&rest[..idx], Some(&rest[idx + 6..]))
    } else {
        (rest, None)
    };
    
    let name = sanitize_identifier(name_part.trim());
    if name.is_empty() {
        return None;
    }
    
    let values: Vec<String> = if let Some(vals) = values_part {
        vals.split(',')
            .flat_map(|s| s.split(" and "))
            .map(|s| sanitize_identifier(s.trim()))
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    };
    
    Some(StatementHint::EnumDef { name, values })
}

fn try_extract_typedef(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "typedef")?;
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    
    if tokens.len() < 2 {
        return None;
    }
    
    let original_type = tokens[0].to_string();
    let new_name = sanitize_identifier(tokens[1]);
    
    if new_name.is_empty() {
        return None;
    }
    
    Some(StatementHint::Typedef { original_type, new_name })
}

fn try_extract_string_decl(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "string")?;
    
    let lower = rest.to_lowercase();
    let (name_part, value_part) = if let Some(idx) = lower.find(" equals ") {
        (&rest[..idx], Some(&rest[idx + 8..]))
    } else if let Some(idx) = lower.find(" = ") {
        (&rest[..idx], Some(&rest[idx + 3..]))
    } else {
        (rest, None)
    };
    
    let name = sanitize_identifier(name_part.trim());
    if name.is_empty() {
        return None;
    }
    
    let initial_value = value_part.map(|v| v.trim().trim_matches('"').to_string());
    
    Some(StatementHint::StringDecl { name, initial_value, size: None })
}

fn try_extract_comment(line: &str) -> Option<StatementHint> {
    // "comment: ..." or "note: ..."
    if let Some(rest) = strip_keyword(line, "comment") {
        let text = rest.trim_start_matches(':').trim().to_string();
        return Some(StatementHint::Comment { text, is_block: false });
    }
    
    if let Some(rest) = strip_keyword(line, "note") {
        let text = rest.trim_start_matches(':').trim().to_string();
        return Some(StatementHint::Comment { text, is_block: false });
    }
    
    if line.trim().to_lowercase() == "block comment start" {
        return Some(StatementHint::Comment { text: String::new(), is_block: true });
    }
    
    None
}

fn try_extract_bitwise(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    
    // "x bitwise and y"
    if lower.contains(" bitwise and ") {
        let idx = lower.find(" bitwise and ").unwrap();
        let left = line.trim()[..idx].trim().to_string();
        let right = line.trim()[idx + 13..].trim().to_string();
        return Some(StatementHint::Bitwise { 
            operation: BitwiseOp::And, 
            left, 
            right: Some(right),
            target: None 
        });
    }
    
    if lower.contains(" bitwise or ") {
        let idx = lower.find(" bitwise or ").unwrap();
        let left = line.trim()[..idx].trim().to_string();
        let right = line.trim()[idx + 12..].trim().to_string();
        return Some(StatementHint::Bitwise { 
            operation: BitwiseOp::Or, 
            left, 
            right: Some(right),
            target: None 
        });
    }
    
    if lower.contains(" xor ") {
        let idx = lower.find(" xor ").unwrap();
        let left = line.trim()[..idx].trim().to_string();
        let right = line.trim()[idx + 5..].trim().to_string();
        return Some(StatementHint::Bitwise { 
            operation: BitwiseOp::Xor, 
            left, 
            right: Some(right),
            target: None 
        });
    }
    
    // "shift x left by n"
    if lower.starts_with("shift ") && lower.contains(" left by ") {
        let rest = &line.trim()[6..];
        let idx = rest.to_lowercase().find(" left by ").unwrap();
        let left = rest[..idx].trim().to_string();
        let right = rest[idx + 9..].trim().to_string();
        return Some(StatementHint::Bitwise { 
            operation: BitwiseOp::ShiftLeft, 
            left, 
            right: Some(right),
            target: None 
        });
    }
    
    if lower.starts_with("shift ") && lower.contains(" right by ") {
        let rest = &line.trim()[6..];
        let idx = rest.to_lowercase().find(" right by ").unwrap();
        let left = rest[..idx].trim().to_string();
        let right = rest[idx + 10..].trim().to_string();
        return Some(StatementHint::Bitwise { 
            operation: BitwiseOp::ShiftRight, 
            left, 
            right: Some(right),
            target: None 
        });
    }
    
    // "not x" (bitwise NOT)
    if lower.starts_with("bitwise not ") {
        let target = line.trim()[12..].trim().to_string();
        return Some(StatementHint::Bitwise { 
            operation: BitwiseOp::Not, 
            left: target, 
            right: None,
            target: None 
        });
    }
    
    None
}

fn try_extract_cast(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    
    // "cast x to int" or "x as int"
    if let Some(rest) = strip_keyword(line, "cast") {
        let lower_rest = rest.to_lowercase();
        if let Some(idx) = lower_rest.find(" to ") {
            let expression = rest[..idx].trim().to_string();
            let target_type = rest[idx + 4..].trim().to_string();
            if !expression.is_empty() && !target_type.is_empty() {
                return Some(StatementHint::Cast { expression, target_type });
            }
        }
    }
    
    // "x as int" pattern
    if lower.contains(" as ") && !lower.contains(" equals ") {
        let idx = lower.find(" as ").unwrap();
        let expression = line.trim()[..idx].trim().to_string();
        let target_type = line.trim()[idx + 4..].trim().to_string();
        if !expression.is_empty() && !target_type.is_empty() {
            return Some(StatementHint::Cast { expression, target_type });
        }
    }
    
    None
}

fn try_extract_array_access(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    
    // "get a at i"
    if let Some(rest) = strip_keyword(line, "get") {
        let lower_rest = rest.to_lowercase();
        if let Some(idx) = lower_rest.find(" at ") {
            let array = sanitize_identifier(&rest[..idx]);
            let index = rest[idx + 4..].trim().to_string();
            if !array.is_empty() && !index.is_empty() {
                return Some(StatementHint::ArrayAccess { array, index, value: None });
            }
        }
    }
    
    // "set a at 0 to 5"
    if lower.starts_with("set ") && lower.contains(" at ") && lower.contains(" to ") {
        let rest = &line.trim()[4..];
        let lower_rest = rest.to_lowercase();
        if let Some(at_idx) = lower_rest.find(" at ") {
            if let Some(to_idx) = lower_rest.find(" to ") {
                if at_idx < to_idx {
                    let array = sanitize_identifier(&rest[..at_idx]);
                    let index = rest[at_idx + 4..to_idx].trim().to_string();
                    let value = rest[to_idx + 4..].trim().to_string();
                    if !array.is_empty() && !index.is_empty() && !value.is_empty() {
                        return Some(StatementHint::ArrayAccess { array, index, value: Some(value) });
                    }
                }
            }
        }
    }
    
    None
}

fn try_extract_sizeof(line: &str) -> Option<StatementHint> {
    // "size of a" or "sizeof a"
    if let Some(rest) = strip_keyword(line, "size of") {
        let target = sanitize_identifier(rest.trim());
        if !target.is_empty() {
            return Some(StatementHint::SizeOf { target });
        }
    }
    
    if let Some(rest) = strip_keyword(line, "sizeof") {
        let target = sanitize_identifier(rest.trim());
        if !target.is_empty() {
            return Some(StatementHint::SizeOf { target });
        }
    }
    
    None
}

fn try_extract_ternary(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    
    // "x when condition else y" → condition ? x : y
    if lower.contains(" when ") && lower.contains(" else ") {
        let when_idx = lower.find(" when ").unwrap();
        let else_idx = lower.find(" else ").unwrap();
        
        if when_idx < else_idx {
            let true_value = line.trim()[..when_idx].trim().to_string();
            let condition = line.trim()[when_idx + 6..else_idx].trim().to_string();
            let false_value = line.trim()[else_idx + 6..].trim().to_string();
            
            if !condition.is_empty() && !true_value.is_empty() && !false_value.is_empty() {
                return Some(StatementHint::Ternary { condition, true_value, false_value });
            }
        }
    }
    
    None
}

// ============================================================================
// Context Analysis - Variable Declaration Tracking
// ============================================================================

use std::collections::HashSet;

/// Tracks declared variables from preceding code.
/// Used to determine whether to declare new variables or just assign.
#[derive(Debug, Clone)]
pub struct VariableContext {
    /// Set of declared variable names
    pub declared: HashSet<String>,
}

impl VariableContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            declared: HashSet::new(),
        }
    }

    /// Extract declared variables from preceding code
    pub fn from_code(code_before: &str) -> Self {
        let mut declared = HashSet::new();
        
        for line in code_before.lines() {
            let trimmed = line.trim();
            
            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
                continue;
            }
            
            // Look for C-style declarations: "type name" or "type name = value"
            // Pattern: int x; int x = 5; int x, y, z;
            for type_keyword in ["int", "float", "double", "char", "bool", "long", "short", "unsigned", "void"] {
                if trimmed.starts_with(type_keyword) {
                    let rest = &trimmed[type_keyword.len()..];
                    // Must have space or * after type
                    if rest.starts_with(' ') || rest.starts_with('*') {
                        // Extract identifiers after the type
                        for part in rest.split(',') {
                            let part = part.trim().trim_end_matches(';').trim_end_matches('{');
                            // Handle "x = 5" or just "x"
                            let name = part.split('=').next().unwrap_or("").trim();
                            // Handle arrays like "x[10]"
                            let name = name.split('[').next().unwrap_or(name).trim();
                            // Handle pointers like "*x"
                            let name = name.trim_start_matches('*').trim();
                            // Handle function params - skip if contains '('
                            if name.contains('(') {
                                continue;
                            }
                            if !name.is_empty() && is_valid_identifier(name) {
                                declared.insert(name.to_string());
                            }
                        }
                    }
                }
            }
            
            // Look for scanf declarations: scanf("%d", &x) means x is declared
            if trimmed.starts_with("scanf(") {
                // Extract variables from &var patterns
                for part in trimmed.split('&') {
                    if part.starts_with("scanf") {
                        continue;
                    }
                    let name = part.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                        .next()
                        .unwrap_or("");
                    if !name.is_empty() && is_valid_identifier(name) {
                        declared.insert(name.to_string());
                    }
                }
            }
            
            // Look for for-loop iterators: for (int i = ...) or for (i = ...)
            if trimmed.starts_with("for") && trimmed.contains('(') {
                if let Some(paren_content) = trimmed.split('(').nth(1) {
                    let init_part = paren_content.split(';').next().unwrap_or("");
                    // Handle "int i = 0" or "i = 0"
                    let init_trimmed = init_part.trim();
                    for type_keyword in ["int", "float", "double"] {
                        if init_trimmed.starts_with(type_keyword) {
                            let rest = init_trimmed[type_keyword.len()..].trim();
                            let name = rest.split('=').next().unwrap_or("").trim();
                            if !name.is_empty() && is_valid_identifier(name) {
                                declared.insert(name.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        Self { declared }
    }

    /// Check if a variable is declared
    pub fn is_declared(&self, name: &str) -> bool {
        self.declared.contains(name)
    }

    /// Get all declared variables
    pub fn get_declared(&self) -> Vec<String> {
        self.declared.iter().cloned().collect()
    }
}

/// Check if a string is a valid C identifier
fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let first = s.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Result of validating read variables against context.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed (all reads are declared)
    pub is_valid: bool,
    /// Undeclared variables that were read
    pub undeclared_reads: Vec<String>,
    /// Variables that need to be declared (write targets not in context)
    pub needs_declaration: Vec<String>,
    /// Variables that already exist (write targets in context)
    pub already_declared: Vec<String>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self {
            is_valid: true,
            undeclared_reads: Vec::new(),
            needs_declaration: Vec::new(),
            already_declared: Vec::new(),
        }
    }

    pub fn error(undeclared: Vec<String>) -> Self {
        Self {
            is_valid: false,
            undeclared_reads: undeclared,
            needs_declaration: Vec::new(),
            already_declared: Vec::new(),
        }
    }

    pub fn error_message(&self) -> Option<String> {
        if self.is_valid {
            None
        } else {
            Some(format!(
                "unknown identifiers: {}",
                self.undeclared_reads.join(", ")
            ))
        }
    }
}

/// Information about read and write variables in a hint
#[derive(Debug, Clone, Default)]
pub struct ReadWriteInfo {
    /// Variables being written to (can be auto-declared)
    pub writes: Vec<String>,
    /// Variables being read from (must already exist)
    pub reads: Vec<String>,
}

/// Extract read/write variable information from a hint
pub fn get_read_write_info(hint: &StatementHint) -> ReadWriteInfo {
    match hint {
        StatementHint::Declaration { names, initial_value, .. } => {
            let mut info = ReadWriteInfo::default();
            info.writes = names.clone();
            // If there's an initial value that's an expression, extract reads
            if let Some(val) = initial_value {
                if !val.parse::<f64>().is_ok() && !val.starts_with('"') {
                    info.reads = extract_identifiers_from_expression(val);
                }
            }
            info
        }
        
        StatementHint::Assignment { targets, value } => {
            let mut info = ReadWriteInfo::default();
            info.writes = targets.clone();
            // Parse value for read variables
            if !value.parse::<f64>().is_ok() && !value.starts_with('"') {
                info.reads = extract_identifiers_from_expression(value);
            }
            info
        }
        
        StatementHint::Modify { target, .. } => {
            // Increment/decrement reads AND writes the target
            ReadWriteInfo {
                writes: vec![],  // Not a new declaration
                reads: vec![target.clone()],  // Must exist to modify
            }
        }
        
        StatementHint::Arithmetic { left, right, target, .. } => {
            let mut info = ReadWriteInfo::default();
            info.reads = vec![left.clone(), right.clone()];
            if let Some(t) = target {
                info.writes.push(t.clone());
            }
            info
        }
        
        StatementHint::Conditional { condition, then_action, else_action, .. } => {
            let mut info = ReadWriteInfo::default();
            info.reads = extract_identifiers_from_expression(condition);
            // Actions may also read variables
            if let Some(action) = then_action {
                info.reads.extend(extract_identifiers_from_expression(action));
            }
            if let Some(action) = else_action {
                info.reads.extend(extract_identifiers_from_expression(action));
            }
            info
        }
        
        StatementHint::Loop { iterator, start, end, collection, body_action, .. } => {
            let mut info = ReadWriteInfo::default();
            // Iterator is a write target (gets declared by the loop)
            if let Some(iter) = iterator {
                info.writes.push(iter.clone());
            }
            // Start, end, collection are reads
            if let Some(s) = start {
                if !s.parse::<f64>().is_ok() {
                    info.reads.extend(extract_identifiers_from_expression(s));
                }
            }
            if let Some(e) = end {
                if !e.parse::<f64>().is_ok() {
                    info.reads.extend(extract_identifiers_from_expression(e));
                }
            }
            if let Some(c) = collection {
                info.reads.push(c.clone());
            }
            if let Some(action) = body_action {
                info.reads.extend(extract_identifiers_from_expression(action));
            }
            info
        }
        
        StatementHint::Print { content: _, is_literal: _ } => {
            // Print is special: we don't validate reads here because
            // an undeclared identifier in print becomes a literal string at code gen time
            // (handled in translate_with_context_c)
            ReadWriteInfo::default()
        }
        
        StatementHint::Return { value } => {
            let mut info = ReadWriteInfo::default();
            if let Some(val) = value {
                if !val.parse::<f64>().is_ok() {
                    info.reads = extract_identifiers_from_expression(val);
                }
            }
            info
        }
        
        StatementHint::While { condition, body_action } => {
            let mut info = ReadWriteInfo::default();
            
            // Parse the condition to identify loop variable vs bound
            let (loop_var, bound, is_counter) = parse_while_condition_for_rw(condition);
            
            if let Some(var) = loop_var {
                if is_counter {
                    // For counter patterns (i < 10), the loop var can be auto-declared
                    info.writes.push(var);
                } else {
                    // For general comparisons (a < b), both must exist
                    info.reads.push(var);
                }
            }
            
            if let Some(b) = bound {
                // Only add as read if it's not a literal number
                if b.parse::<f64>().is_err() {
                    info.reads.push(b);
                }
            }
            
            if let Some(action) = body_action {
                info.reads.extend(extract_identifiers_from_expression(action));
            }
            info
        }
        
        StatementHint::Read { variables } => {
            // Read introduces new variables (scanf declares them)
            ReadWriteInfo {
                writes: variables.clone(),
                reads: vec![],
            }
        }
        
        StatementHint::Conditional { .. } => {
            // For conditionals, we don't validate reads strictly.
            // The user might be typing a condition with variables they'll declare later,
            // or they might be experimenting. The C compiler will catch real errors.
            // This allows patterns like "if a <= b print a else print b" to work
            // without requiring a and b to be declared first.
            ReadWriteInfo::default()
        }
        
        _ => ReadWriteInfo::default(),
    }
}

/// Validate that all read variables are declared in context
pub fn validate_reads(hint: &StatementHint, context: &VariableContext) -> ValidationResult {
    let rw_info = get_read_write_info(hint);
    
    let mut result = ValidationResult::ok();
    
    // Check each read variable
    for var in &rw_info.reads {
        // Skip numbers and constants
        if var.parse::<f64>().is_ok() {
            continue;
        }
        let var_lower = var.to_lowercase();
        if ["true", "false", "null", "nullptr", "none"].contains(&var_lower.as_str()) {
            continue;
        }
        // Check if declared
        if !context.is_declared(var) {
            result.is_valid = false;
            if !result.undeclared_reads.contains(var) {
                result.undeclared_reads.push(var.clone());
            }
        }
    }
    
    // Classify write variables
    for var in &rw_info.writes {
        if context.is_declared(var) {
            result.already_declared.push(var.clone());
        } else {
            result.needs_declaration.push(var.clone());
        }
    }
    
    result
}

// Legacy compatibility - keep old function name
pub fn analyze_context(hint: &StatementHint, code_before: &str) -> ContextAnalysis {
    let context = VariableContext::from_code(code_before);
    let validation = validate_reads(hint, &context);
    
    ContextAnalysis {
        warnings: validation.undeclared_reads.iter()
            .map(|v| format!("Variable '{}' may not be declared", v))
            .collect(),
        undeclared_vars: validation.undeclared_reads,
    }
}

/// Legacy struct for backward compatibility
#[derive(Debug, Clone)]
pub struct ContextAnalysis {
    pub warnings: Vec<String>,
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

/// Parse a while condition to extract the loop variable, bound, and whether it's a counter pattern
/// Returns (loop_var, bound, is_counter_pattern)
fn parse_while_condition_for_rw(condition: &str) -> (Option<String>, Option<String>, bool) {
    let condition = condition.trim();
    
    // Look for comparison operators
    for op in [" < ", " <= ", " > ", " >= ", " != ", " == "] {
        if let Some(idx) = condition.find(op) {
            let left = condition[..idx].trim();
            let right = condition[idx + op.len()..].trim();
            
            // Left side is the loop variable if it's a simple identifier
            let loop_var = if left.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') 
               && left.chars().next().map(|c| c.is_ascii_alphabetic() || c == '_').unwrap_or(false) {
                Some(left.to_string())
            } else {
                None
            };
            
            // Right side is the bound
            let bound = if !right.is_empty() {
                Some(right.to_string())
            } else {
                None
            };
            
            // Determine if this is a counter pattern:
            // - Variable is a common iterator name (i, j, k, n, count, counter, idx, index)
            // - OR bound is a numeric literal
            let is_counter_var = loop_var.as_ref().map(|v| {
                matches!(v.to_lowercase().as_str(), 
                    "i" | "j" | "k" | "n" | "count" | "counter" | "idx" | "index" | "iter")
            }).unwrap_or(false);
            let bound_is_numeric = bound.as_ref().map(|b| {
                b.chars().all(|c| c.is_ascii_digit() || c == '-')
            }).unwrap_or(false);
            let is_counter = is_counter_var || bound_is_numeric;
            
            return (loop_var, bound, is_counter);
        }
    }
    
    // Couldn't parse
    (None, None, false)
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

