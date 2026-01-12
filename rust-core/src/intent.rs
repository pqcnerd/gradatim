//! Intent classification for incoming English instructions.
//!
//! This module determines whether an instruction is:
//! - **Statement-level**: A single programming construct (variable, loop, condition, etc.)
//! - **Project-level**: A high-level request that would require generating an entire program
//!
//! Project-level requests are rejected with guidance to break them down.

#![allow(dead_code)]

/// Result of intent classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    /// A single statement or small construct - allowed.
    Statement,
    /// A project-level request - should be rejected.
    Project { reason: String },
}

impl Intent {
    /// Returns true if this is a project-level (disallowed) intent.
    pub fn is_project_level(&self) -> bool {
        matches!(self, Intent::Project { .. })
    }

    /// Returns an error message if project-level, None otherwise.
    pub fn rejection_message(&self) -> Option<&str> {
        match self {
            Intent::Project { reason } => Some(reason.as_str()),
            Intent::Statement => None,
        }
    }
}

/// High-level conceptual nouns that indicate a project, not a statement.
const PROJECT_NOUNS: &[&str] = &[
    "calculator",
    "game",
    "app",
    "application",
    "website",
    "webpage",
    "system",
    "program",
    "project",
    "tool",
    "utility",
    "server",
    "client",
    "database",
    "interface",
    "gui",
    "menu",
    "dashboard",
    "api",
    "service",
    "bot",
    "chatbot",
    "editor",
    "compiler",
    "interpreter",
    "simulator",
    "emulator",
    "browser",
    "player",
    "manager",
    "tracker",
    "scheduler",
    "planner",
    "organizer",
    "converter",
    "generator",
    "builder",
    "framework",
    "library",
    "engine",
    "platform",
];

/// Imperative verbs that, when combined with project nouns, indicate project-level intent.
const PROJECT_VERBS: &[&str] = &[
    "build",
    "create",
    "make",
    "develop",
    "design",
    "implement",
    "write",
    "code",
    "program",
    "construct",
    "assemble",
    "craft",
    "produce",
    "generate",
];

/// Keywords that indicate statement-level programming constructs.
const STATEMENT_INDICATORS: &[&str] = &[
    // Variables and types
    "variable",
    "int",
    "float",
    "double",
    "char",
    "string",
    "bool",
    "array",
    "list",
    "pointer",
    // Control flow
    "if",
    "else",
    "loop",
    "for",
    "while",
    "switch",
    "case",
    "break",
    "continue",
    "return",
    // Operations
    "declare",
    "set",
    "assign",
    "increment",
    "decrement",
    "add",
    "subtract",
    "multiply",
    "divide",
    "print",
    "read",
    "input",
    "output",
    // Structures
    "struct",
    "function",
    "parameter",
    "argument",
    "macro",
    "pragma",
    "goto",
    "label",
    "assert",
    "malloc",
    "realloc",
    "calloc",
    "memcpy",
    "memset",
    "strcmp",
    "strlen",
    "strcpy",
    "sprintf",
    "fopen",
    "fclose",
    "fprintf",
    "fscanf",
    // Specific identifiers (lowercase letters followed by numbers or underscores)
    "x",
    "y",
    "z",
    "i",
    "j",
    "k",
    "n",
    "count",
    "sum",
    "total",
    "result",
    "value",
    "index",
    "temp",
];

/// Words that indicate multiple components or features (project-level).
const MULTI_COMPONENT_INDICATORS: &[&str] = &[
    "with features",
    "that can",
    "which can",
    "including",
    "with support for",
    "complete with",
    "full",
    "fully functional",
    "working",
    "functional",
];

/// Classify the intent of an English instruction.
///
/// Returns `Intent::Statement` for allowed single-statement instructions,
/// or `Intent::Project` for high-level requests that should be rejected.
pub fn classify(english_line: &str) -> Intent {
    let line = english_line.trim().to_lowercase();

    if line.is_empty() {
        return Intent::Statement;
    }

    // Check for multi-component indicators first (strong project signal)
    for indicator in MULTI_COMPONENT_INDICATORS {
        if line.contains(indicator) {
            return Intent::Project {
                reason: format!(
                    "This looks like a multi-feature request. \
                     Please break it down into individual statements."
                ),
            };
        }
    }

    // Check for project verb + project noun combinations
    let has_project_verb = PROJECT_VERBS.iter().any(|verb| {
        line.starts_with(verb) || line.contains(&format!(" {} ", verb))
    });

    let found_project_noun = PROJECT_NOUNS.iter().find(|noun| {
        line.contains(*noun)
    });

    // Check for statement-level indicators
    let has_statement_indicator = STATEMENT_INDICATORS.iter().any(|indicator| {
        // Match as whole word
        let pattern_space = format!(" {} ", indicator);
        let pattern_start = format!("{} ", indicator);
        let pattern_end = format!(" {}", indicator);
        line == *indicator
            || line.contains(&pattern_space)
            || line.starts_with(&pattern_start)
            || line.ends_with(&pattern_end)
    });

    // If we have statement indicators, it's likely a statement even with project-ish words
    if has_statement_indicator {
        return Intent::Statement;
    }

    // Project verb + project noun without statement indicators = project-level
    if has_project_verb {
        if let Some(noun) = found_project_noun {
            return Intent::Project {
                reason: format!(
                    "\"{}\" sounds like a complete project. \
                     Try breaking it into specific statements like variable declarations, \
                     loops, or conditions.",
                    noun
                ),
            };
        }
    }

    // Check for standalone project nouns with articles (a/an/the calculator)
    for noun in PROJECT_NOUNS {
        let patterns = [
            format!("a {} ", noun),
            format!("an {} ", noun),
            format!("the {} ", noun),
            format!("a {}", noun),
            format!("an {}", noun),
            format!("the {}", noun),
        ];
        for pattern in &patterns {
            if line.contains(pattern) && !has_statement_indicator {
                return Intent::Project {
                    reason: format!(
                        "\"{}\" is a high-level concept. \
                         Please specify the individual operations you need \
                         (e.g., \"declare variables\", \"add two numbers\", \"loop through array\").",
                        noun
                    ),
                };
            }
        }
    }

    // Check for very short lines with only project verbs (ambiguous)
    let word_count = line.split_whitespace().count();
    if word_count <= 2 && has_project_verb && found_project_noun.is_some() {
        return Intent::Project {
            reason: "This request is too vague. Please be specific about what you want to do."
                .to_string(),
        };
    }

    // Default to statement-level (allow AI to handle)
    Intent::Statement
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_statement_level_intents() {
        assert_eq!(classify("declare x"), Intent::Statement);
        assert_eq!(classify("set x to 5"), Intent::Statement);
        assert_eq!(classify("loop from 0 to 10"), Intent::Statement);
        assert_eq!(classify("if x is greater than 5"), Intent::Statement);
        assert_eq!(classify("print hello world"), Intent::Statement);
        assert_eq!(classify("increment counter"), Intent::Statement);
        assert_eq!(classify("add a and b"), Intent::Statement);
        assert_eq!(classify("create function foo"), Intent::Statement);
        assert_eq!(classify("declare an array of 10 integers"), Intent::Statement);
    }

    #[test]
    fn test_project_level_intents() {
        assert!(classify("make a calculator").is_project_level());
        assert!(classify("build a todo app").is_project_level());
        assert!(classify("create a game").is_project_level());
        assert!(classify("write a program that can do everything").is_project_level());
        assert!(classify("build a website with features").is_project_level());
        assert!(classify("create a fully functional system").is_project_level());
    }

    #[test]
    fn test_edge_cases() {
        // Empty line
        assert_eq!(classify(""), Intent::Statement);
        
        // Statement with project-ish words but clear programming context
        assert_eq!(classify("declare a variable called calculator"), Intent::Statement);
        assert_eq!(classify("print game over"), Intent::Statement);
    }
}

