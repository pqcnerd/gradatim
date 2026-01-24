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
    ("volatile", &["volatile", "volatile variable", "volatile value"]),
    ("static", &["persistent", "global", "shared"]),
    ("extern", &["external", "external variable", "externally defined"]),
    ("register", &["register variable"]),
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
    ("xor", &["exclusive or", "bitwise xor"]),
    ("neither", &["neither...nor", "not x and not y"]),
    ("any of", &["at least one of", "one or more of", "some of"]),
    ("all of", &["every one of", "each of", "every"]),
    ("none of", &["not any of", "zero of"]),
    ("exactly one", &["precisely one", "one and only one"]),
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
    ("doubled", &["double", "times two", "multiply by two", "twice"]),
    ("halved", &["half", "divide by two", "cut in half"]),
    ("negated", &["negate", "flip sign", "opposite sign", "negative of"]),
    ("absolute", &["absolute value", "abs", "magnitude"]),
    ("squared", &["square", "power of 2", "to the second"]),
    ("cubed", &["cube", "power of 3", "to the third"]),
    ("rounded", &["round", "round to nearest", "round off"]),
    ("floor", &["round down", "truncate down"]),
    ("ceiling", &["ceil", "round up"]),
    ("between", &["in range", "within range", "from x to y inclusive"]),
    ("at least", &["minimum", "no less than", "greater than or equal"]),
    ("at most", &["maximum", "no more than", "less than or equal"]),
    ("approximately", &["about", "roughly", "close to", "near"]),
    ("is null", &["is nil", "is nothing", "is empty pointer", "points to null"]),
    ("is empty", &["has no elements", "length is zero", "contains nothing"]),
    ("is valid", &["is not null", "has value", "exists", "is defined"]),
    ("contains", &["has", "includes", "holds", "encloses"]),
    ("is initialized", &["has been set", "is set up"]),
    ("remove", &["delete", "erase", "eliminate", "take out"]),
    ("insert", &["put in", "place in", "add at position", "inject"]),
    ("prepend", &["add to front", "insert at beginning", "add at start"]),
    ("clear", &["empty", "reset to empty", "wipe", "make empty"]),
    ("replace", &["substitute", "swap", "exchange", "change to"]),
    ("find", &["search", "locate", "discover", "detect"]),
    ("sort", &["order", "arrange", "organize"]),
    ("reverse", &["flip", "invert order", "turn around"]),
    ("merge", &["combine", "join", "unite"]),
    ("split", &["divide", "separate", "break apart"]),
    ("filter", &["select", "choose matching", "keep only"]),
    ("map", &["transform", "convert each", "apply to each"]),
    ("after", &["following", "subsequent to", "once", "when finished"]),
    ("before", &["prior to", "preceding", "earlier than"]),
    ("then", &["next", "subsequently", "afterward"]),
    ("finally", &["at the end", "lastly", "ultimately"]),
    ("immediately", &["right away", "instantly", "at once"]),
    ("increment", &["increase", "bump", "add one", "++", "raise"]),
    ("decrement", &["decrease", "reduce", "subtract one", "--", "lower"]),
    ("assign", &["set to", "store in", "put into"]),
    ("cast", &["convert to", "convert into", "cast as", "treat as", "interpret as"]),

    // Misc helpers
    ("first", &["1st"]),
    ("second", &["2nd"]),
    ("third", &["3rd"]),
    ("fourth", &["4th"]),
    ("fifth", &["5th"]),
    ("sixth", &["6th"]),
    ("seventh", &["7th"]),
    ("eighth", &["8th"]),
    ("ninth", &["9th"]),
    ("tenth", &["10th"]),
    ("last", &["final"]),
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
