//! Synonym normalization for English input.
//!
//! This module maps many English synonyms and phrases to canonical keywords
//! understood by the rule-based extractor. It is intentionally extensive but
//! tries to preserve meaning by only replacing whole words/phrases.

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

/// Canonical terms and their synonyms. Canonical terms align with the keywords
/// expected by the hint extraction layer (e.g., "include", "declare",
/// "pointer", "add").
const SYNONYM_GROUPS: &[(&str, &[&str])] = &[
    // Preprocessor / includes
    (
        "include",
        &[
            "import",
            "require",
            "import header",
            "import library",
            "import file",
            "bring in header",
            "bring in library",
            "bring in file",
            "use header",
            "use library",
            "use include",
            "load header",
            "bring in",
            "pull in",
            "add header",
            "add include",
            "add library",
            "include file",
            "include header",
            "bring header",
            "load library",
            "reference header",
            "reference library",
        ],
    ),
    (
        "define",
        &[
            "define macro",
            "macro",
            "macro define",
            "macro definition",
            "const macro",
        ],
    ),
    ("pragma", &["compiler hint", "compiler directive"]),
    ("undef", &["undefine", "remove macro"]),
    ("ifdef", &["if defined"]),
    ("ifndef", &["if not defined", "ifndef"]),
    ("endif", &["end if defined", "end ifdef"]),

    // Declarations and assignment
    (
        "declare",
        &[
            "create",
            "make",
            "establish",
            "instantiate",
            "spin up",
            "form",
            "build",
            "introduce",
            "set up variable",
            "allocate variable",
            "set up",
            "setup",
            "initialize variable",
            "initialize",
            "init variable",
            "add variable",
            "new variable",
            "declare variable",
            "define variable",
            "prepare variable",
        ],
    ),
    (
        "set",
        &[
            "assign",
            "store",
            "put",
            "place",
            "make equal",
            "equals",
            "let",
            "give",
            "set to",
            "set as",
            "set equal to",
            "give value",
            "assign value",
            "bind",
        ],
    ),
    ("const", &["constant", "immutable", "readonly", "fixed"]),
    ("static", &["persistent", "global", "shared"]),
    ("typedef", &["type alias", "alias type", "define type"]),

    // Control flow
    (
        "if",
        &["when", "whenever", "in case", "provided that", "assuming", "should"],
    ),
    ("else if", &["otherwise if", "elsewhen", "else when"]),
    ("else", &["otherwise", "or else", "alternatively", "if not"]),
    (
        "loop",
        &[
            "iterate",
            "repeat",
            "cycle",
            "go through",
            "traverse",
            "loop through",
            "loop over",
            "walk through",
            "step through",
            "run through",
            "go over",
        ],
    ),
    (
        "for",
        &[
            "for each",
            "foreach",
            "for every",
            "for all",
            "for loop",
            "for each element",
            "for every item",
            "for each item",
            "for each value",
        ],
    ),
    (
        "while",
        &[
            "as long as",
            "during",
            "until not",
            "so long as",
            "while condition holds",
            "keep doing while",
        ],
    ),
    ("break", &["exit loop", "stop loop", "leave loop", "break out"]),
    ("continue", &["skip", "next iteration", "skip iteration"]),
    ("switch", &["match", "case of", "select", "choose"]),
    ("case", &["when value", "option", "choice"]),
    ("default", &["otherwise", "fallback", "catch all"]),
    ("goto", &["jump to", "go to", "branch to"]),
    ("return", &["give back", "send back", "yield", "hand back"]),
    ("end", &["close", "finish", "terminate", "stop", "done", "end block"]),

    // Functions and types
    (
        "function",
        &["func", "method", "procedure", "routine", "subroutine", "fn", "def"],
    ),
    ("struct", &["structure", "record", "class", "type", "composite"]),
    ("enum", &["enumeration", "enumerated type", "variant type"]),
    ("pointer", &["ptr", "reference", "address of", "memory address"]),
    (
        "array",
        &[
            "list",
            "vector",
            "collection",
            "arr",
            "sequence",
            "series",
            "bunch",
            "group",
            "set",
            "bag",
        ],
    ),

    // Types
    (
        "int",
        &[
            "integer",
            "whole number",
            "signed integer",
            "number",
            "numeric",
            "count",
            "counter",
        ],
    ),
    ("long", &["long int", "extended", "big int"]),
    ("short", &["short int", "small int", "compact int"]),
    ("unsigned", &["positive only", "non-negative", "uint"]),
    ("signed", &["can be negative", "int signed"]),
    (
        "float",
        &[
            "decimal",
            "real",
            "floating point",
            "real number",
            "fractional",
            "float number",
        ],
    ),
    ("double", &["double precision", "long float", "double float"]),
    ("char", &["character", "letter", "single character", "character type"]),
    (
        "string",
        &[
            "text",
            "str",
            "character array",
            "char array",
            "c string",
            "word",
            "sentence",
            "line of text",
        ],
    ),
    ("bool", &["boolean", "true or false"]),
    ("void", &["nothing", "no return", "empty return"]),
    ("size_t", &["size type", "usize", "memory size type", "size integer"]),

    // Memory
    (
        "malloc",
        &[
            "allocate",
            "allocate memory",
            "alloc",
            "memory allocate",
            "heap allocate",
            "heap alloc",
        ],
    ),
    ("calloc", &["allocate zeroed", "allocate cleared", "clear alloc"]),
    (
        "realloc",
        &["resize alloc", "resize allocation", "re allocate", "grow alloc", "shrink alloc"],
    ),
    (
        "free",
        &[
            "deallocate",
            "release",
            "release memory",
            "free memory",
            "free alloc",
            "free allocation",
        ],
    ),
    ("null", &["nil", "nothing", "none", "empty", "void pointer"]),
    (
        "sizeof",
        &["size of", "byte size", "memory size", "measure size", "size in bytes"],
    ),

    // I/O and string ops
    (
        "print",
        &[
            "output",
            "display",
            "show",
            "write",
            "log",
            "echo",
            "put",
            "dump",
            "emit",
            "emit text",
        ],
    ),
    ("read", &["input", "get", "scan", "receive", "fetch", "accept", "take in"]),
    ("open", &["file open", "open file", "create file handle", "open handle"]),
    ("close", &["file close", "close file", "close handle"]),
    ("write", &["write file", "file write", "write to file", "output to file"]),
    ("readfile", &["read file", "file read", "read from file"]),
    ("append", &["append file", "file append", "append to file", "add to file"]),
    ("strcpy", &["copy string", "string copy", "copy c string"]),
    ("strcat", &["concat string", "string concatenate", "append string"]),
    ("strlen", &["string length", "length of string", "len string"]),

    // Operators / comparisons / logic
    (
        "greater",
        &[
            "bigger",
            "larger",
            "huger",
            "higher",
            "more",
            "above",
            "over",
            "exceeding",
            "greater in size",
            "greater in value",
            "bigger in size",
            "larger in size",
        ],
    ),
    (
        "less",
        &[
            "smaller",
            "lower",
            "tinier",
            "fewer",
            "under",
            "below",
            "lesser",
            "beneath",
            "less in size",
            "lesser in value",
            "smaller in size",
        ],
    ),
    (
        "equal",
        &[
            "same",
            "equivalent",
            "identical",
            "matching",
            "alike",
            "equal to",
            "equivalent to",
            "same as",
        ],
    ),
    ("equals", &["is equal to", "is same as", "matches", "==", "equals to"]),
    (
        "not equals",
        &[
            "is not equal",
            "differs from",
            "not equal",
            "!=",
            "is different from",
            "is unlike",
            "not the same as",
        ],
    ),
    ("and", &["also", "as well as", "along with", "both", "&&"]),
    ("or", &["either", "alternatively", "||"]),
    ("not", &["negation", "inverse", "opposite", "!"]),
    (
        "add",
        &[
            "plus",
            "sum",
            "combine",
            "increase by",
            "+",
            "addition",
            "add together",
            "total",
            "add up",
        ],
    ),
    (
        "subtract",
        &[
            "minus",
            "take away",
            "decrease by",
            "remove",
            "-",
            "subtraction",
            "deduct",
            "deduct from",
        ],
    ),
    (
        "multiply",
        &[
            "times",
            "product",
            "*",
            "multiplied by",
            "multiplication",
            "times by",
            "scale by",
        ],
    ),
    (
        "divide",
        &[
            "divided by",
            "split",
            "/",
            "quotient",
            "division",
            "over",
            "per",
            "ratio",
        ],
    ),
    ("modulo", &["mod", "remainder", "%", "modulus", "remainder of"]),
    ("increment", &["increase", "bump", "add one", "++", "raise"]),
    ("decrement", &["decrease", "reduce", "subtract one", "--", "lower"]),
    ("assign", &["set to", "store in", "put into"]),

    // Misc helpers
    ("main", &["entry point", "program start"]),
    ("comment", &["note", "remark", "annotation"]),
];

/// Regex replacements for multi-word synonyms (longest first).
static PHRASE_RULES: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    let mut phrases: Vec<(usize, &'static str, &'static str)> = SYNONYM_GROUPS
        .iter()
        .flat_map(|(canonical, synonyms)| {
            synonyms
                .iter()
                .filter(|s| s.contains(' '))
                .map(|syn| (syn.len(), *syn, *canonical))
        })
        .collect();

    phrases.sort_by(|a, b| b.0.cmp(&a.0));

    phrases
        .into_iter()
        .map(|(_, syn, canonical)| {
            let pattern = format!(r"(?i)\b{}\b", regex::escape(syn));
            let re = Regex::new(&pattern).expect("valid synonym regex");
            (re, canonical)
        })
        .collect()
});

/// Map of single-word (case-insensitive) synonyms to canonical terms.
static WORD_MAP: Lazy<HashMap<String, &'static str>> = Lazy::new(|| {
    let mut map: HashMap<String, &'static str> = HashMap::new();
    for (canonical, synonyms) in SYNONYM_GROUPS.iter() {
        for &syn in *synonyms {
            if !syn.contains(' ') {
                map.insert(syn.to_lowercase(), *canonical);
            }
        }
        // Also map the canonical term to itself for pass-through.
        map.insert((*canonical).to_lowercase(), *canonical);
    }
    map
});

/// Normalize an English line by replacing known synonyms with their canonical
/// terms. Multi-word phrases are processed before single-word replacements.
pub fn normalize(input: &str) -> String {
    let mut out = input.to_string();

    // Replace multi-word phrases first (longest first).
    for (regex, canonical) in PHRASE_RULES.iter() {
        out = regex.replace_all(&out, *canonical).into_owned();
    }

    // Replace single words while preserving surrounding punctuation.
    let tokens: Vec<String> = out
        .split_whitespace()
        .map(|tok| normalize_token(tok))
        .collect();

    tokens.join(" ")
}

fn normalize_token(token: &str) -> String {
    let (prefix, core, suffix) = split_token(token);
    if core.is_empty() {
        return token.to_string();
    }

    let lookup = core.to_lowercase();
    let replacement = WORD_MAP.get(lookup.as_str()).copied().unwrap_or(&core);
    format!("{prefix}{replacement}{suffix}")
}

fn split_token(token: &str) -> (String, String, String) {
    let bytes = token.as_bytes();
    let mut start = 0;
    let mut end = bytes.len();

    while start < bytes.len() && !is_word_char(bytes[start] as char) {
        start += 1;
    }
    while end > start && !is_word_char(bytes[end - 1] as char) {
        end -= 1;
    }

    let prefix = token[..start].to_string();
    let core = if start < end {
        token[start..end].to_string()
    } else {
        String::new()
    };
    let suffix = token[end..].to_string();
    (prefix, core, suffix)
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
