# Gradatim Rust Core — Deep Technical Reference

> A complete walkthrough of how `rust-core` reads a single English sentence and produces a single line of syntactically correct code.

---

## Table of Contents

1. [Overview](#overview)
2. [The Seven-Stage Pipeline](#the-seven-stage-pipeline)
3. [Stage 1 — Intent Classification](#stage-1--intent-classification)
4. [Stage 2 — Hint Extraction](#stage-2--hint-extraction)
5. [Stage 3 — Variable Context Building](#stage-3--variable-context-building)
6. [Stage 4 — Read Validation](#stage-4--read-validation)
7. [Stage 5 — Trivial Path: Rule-Based Translation](#stage-5--trivial-path-rule-based-translation)
8. [Stage 6 — AI Path: Prompt Construction & Generation](#stage-6--ai-path-prompt-construction--generation)
9. [Stage 7 — Response Validation](#stage-7--response-validation)
10. [Provider Routing](#provider-routing)
11. [The Fallback Guarantee](#the-fallback-guarantee)
12. [Data Flow: End to End](#data-flow-end-to-end)
13. [Module Map](#module-map)

---

## Overview

The Rust core is an [Axum](https://github.com/tokio-rs/axum) HTTP microservice that exposes a single endpoint:

```
POST http://127.0.0.1:4888/translate-line
```

It receives a JSON payload containing:

| Field | Type | Description |
|---|---|---|
| `english_line` | `string` | The natural language instruction to translate |
| `code_before` | `string` | All code above the current line (context window) |
| `code_after` | `string` | All code below the current line (context window) |
| `language` | `string` | Target language: `"c"`, `"python"`, etc. |
| `line_index` | `usize` | 0-based line number of the instruction |
| `provider` | `string?` | AI provider: `"gemini"`, `"openai"`, `"deepseek"`, `"other"` |
| `api_key` | `string?` | Per-request API key (overrides server environment) |
| `model` | `string?` | Specific model override (e.g. `"gemini-2.5-flash"`) |

And it returns:

| Field | Type | Description |
|---|---|---|
| `kind` | `"ok" \| "error" \| "unhandled"` | Outcome |
| `code` | `string?` | The generated code line |
| `message` | `string?` | Error message or unhandled reason |
| `warnings` | `string[]?` | Non-fatal warnings (e.g. undeclared variable use) |

The server is intentionally **single-endpoint and stateless** — every request carries everything needed to process it completely. No session state, no global mutable translation memory.

---

## The Seven-Stage Pipeline

Every call to `/translate-line` passes through exactly this sequence:

```
  ┌─────────────────────────────────────────────────────────────────────┐
  │                   /translate-line  POST                             │
  │                   JSON payload arrives                              │
  └────────────────────────────┬────────────────────────────────────────┘
                               │
                               ▼
  ╔═════════════════════════════════════════════════════════════════════╗
  ║  STAGE 1 ── INTENT CLASSIFICATION                                  ║
  ║  Is this a statement or a project-level request?                   ║
  ╚══════════════════════════════════╦══════════════════════════════════╝
                                     │
               ┌─────────────────────┤
               │ Project-level       │ Statement-level
               ▼                     ▼
         400 BAD_REQUEST       ┌─────────────────────────────────────┐
         Rejected              │                                     │
                               ▼
  ╔═════════════════════════════════════════════════════════════════════╗
  ║  STAGE 2 ── HINT EXTRACTION                                        ║
  ║  Parse English into a typed StatementHint enum variant             ║
  ╚══════════════════════════════════╦══════════════════════════════════╝
                                     │
                                     ▼
  ╔═════════════════════════════════════════════════════════════════════╗
  ║  STAGE 3 ── VARIABLE CONTEXT BUILDING                              ║
  ║  Scan code_before → extract declared variable names & types        ║
  ╚══════════════════════════════════╦══════════════════════════════════╝
                                     │
                                     ▼
  ╔═════════════════════════════════════════════════════════════════════╗
  ║  STAGE 4 ── READ VALIDATION                                        ║
  ║  Does this hint read variables that don't exist yet?               ║
  ╚══════════════════════════════════╦══════════════════════════════════╝
                                     │
               ┌─────────────────────┤
               │ Invalid read        │ Valid
               ▼                     ▼
         400 BAD_REQUEST    ┌─────────────────────────┐
         "x undeclared"     │                         │
                       is_trivial()?                  │
                       /        \                     │
                      YES        NO                   │
                       │          │                   │
                       ▼          ▼                   │
  ╔══════════════╗   ╔══════════════════════════════╗ │
  ║  STAGE 5     ║   ║  STAGE 6                     ║ │
  ║  RULE-BASED  ║   ║  AI PROMPT CONSTRUCTION      ║ │
  ║  TRANSLATION ║   ║  + GENERATION                ║ │
  ╚══════╦═══════╝   ╚══════════════╦═══════════════╝ │
         │                          │                  │
         │                AI fails? │                  │
         │                    │     │                  │
         │                    ▼     │                  │
         │             STAGE 5 fallback                │
         │                          │                  │
         └──────────────────────────┘                  │
                                     │                  │
                                     ▼
  ╔═════════════════════════════════════════════════════════════════════╗
  ║  STAGE 7 ── RESPONSE VALIDATION                                    ║
  ║  Reject markdown, URLs, wrong language, multiple function defs     ║
  ╚══════════════════════════════════╦══════════════════════════════════╝
                                     │
                                     ▼
                              200 OK  { kind: "ok", code: "..." }
```

---

## Stage 1 — Intent Classification

**File:** `src/intent.rs`  
**Function:** `classify(english_line: &str) -> Intent`

The first gate asks: _is this a single programming statement, or is the user trying to ask for an entire program?_

Gradatim is a line-by-line tool. It is not a chatbot. If someone types `"build a calculator"` or `"make a game"`, the service must reject it with a useful message — not attempt a translation that would produce garbage.

### How classification works

The classifier runs **three pattern-matching passes** in priority order:

```
  Input: "build a website with features"
          │
          ▼
  ┌──────────────────────────────────────────────────────────┐
  │  PASS 1: Multi-component indicator scan                  │
  │  Phrases: "with features", "that can", "fully functional"│
  │  "including", "complete with", "with support for"        │
  │                                                          │
  │  "with features" FOUND → Project { reason: "..." }  ←───┘ STOP
  └──────────────────────────────────────────────────────────┘

  Input: "create a game"
          │
          ▼
  ┌──────────────────────────────────────────────────────────┐
  │  PASS 1: No multi-component indicator                    │
  └───────────────────────────────┬──────────────────────────┘
                                  │
                                  ▼
  ┌──────────────────────────────────────────────────────────┐
  │  PASS 2: Project verb × Project noun cross-match         │
  │                                                          │
  │  PROJECT_VERBS: build, create, make, develop, design,    │
  │                 implement, write, code, program...       │
  │                                                          │
  │  PROJECT_NOUNS: calculator, game, app, website, system,  │
  │                 program, server, database, api...        │
  │                                                          │
  │  "create" ∈ PROJECT_VERBS ✓                              │
  │  "game"   ∈ PROJECT_NOUNS ✓                              │
  │                                                          │
  │  Check STATEMENT_INDICATORS to avoid false positives:    │
  │    declare, set, if, loop, print, int, function...       │
  │    None found.                                           │
  │                                                          │
  │  → Project { reason: "game is a high-level concept..." } │
  └──────────────────────────────────────────────────────────┘

  Input: "create function foo"
          │
          ▼
  ┌──────────────────────────────────────────────────────────┐
  │  PASS 2: "create" ∈ PROJECT_VERBS, no noun found         │
  │  But: "function" ∈ STATEMENT_INDICATORS ✓               │
  │  Statement indicators override → Statement               │
  └──────────────────────────────────────────────────────────┘
```

### Decision table

| Input | Project verb? | Project noun? | Statement indicator? | Result |
|---|---|---|---|---|
| `"build a game"` | ✓ | ✓ | ✗ | **Rejected** |
| `"create function foo"` | ✓ | ✗ | ✓ (function) | **Allowed** |
| `"declare a variable"` | ✗ | ✗ | ✓ (declare) | **Allowed** |
| `"make an app with features"` | ✓ | ✓ | ✗ | **Rejected** (pass 1) |
| `"if x > 5"` | ✗ | ✗ | ✓ (if) | **Allowed** |
| `"print game over"` | ✗ | ✓ | ✓ (print) | **Allowed** |

---

## Stage 2 — Hint Extraction

**File:** `src/hints.rs`  
**Function:** `extract(payload: &TranslateLineRequest) -> StatementHint`

This is the intellectual core of the entire system. The hint extractor reads the English sentence and converts it into one of **40+ typed enum variants**, each carrying structured data about programmer intent.

### The StatementHint enum

Rather than passing raw English to the AI and hoping for the best, the hint extractor first attempts to _understand_ what the user wants at a structural level — before any AI call happens.

```
  StatementHint
  │
  ├── Declaration      → "declare x", "create an integer called count"
  │     names: ["x"]
  │     type_hint: Some("int")
  │     initial_value: None
  │     is_array: false
  │
  ├── Assignment       → "set x to 5", "set a, b, c 0"
  │     targets: ["x"]
  │     value: "5"
  │
  ├── Loop             → "loop from 0 to 10", "for i in items"
  │     iterator: Some("i")
  │     start: Some("0")
  │     end: Some("10")
  │     collection: None
  │
  ├── While            → "while i < n"
  │     condition: "i < n"
  │
  ├── Conditional      → "if x > 5", "when count equals zero"
  │     condition: "x > 5"
  │     then_action: None
  │     else_action: None
  │     is_else_if: false
  │
  ├── Else             → "else", "otherwise"
  │
  ├── EndBlock         → "end", "end if", "end loop"
  │
  ├── Print            → "print hello world"
  │     content: "hello world"
  │     is_literal: true
  │
  ├── Read             → "read n", "input x"
  │     variables: ["n"]
  │
  ├── Return           → "return 0", "return result"
  │     value: Some("0")
  │
  ├── Arithmetic       → "add a and b", "multiply x by 2"
  │     operation: Add / Subtract / Multiply / Divide / Modulo
  │     left: "a"
  │     right: "b"
  │     target: None
  │
  ├── Modify           → "increment x", "decrement counter"
  │     target: "x"
  │     delta: +1 / -1
  │
  ├── FunctionDef      → "function foo taking int x returns int"
  │     name: "foo"
  │     parameters: [("int", "x")]
  │     return_type: Some("int")
  │
  ├── StructDef        → "struct Point with x and y"
  │     name: "Point"
  │     fields: [("int","x"), ("int","y")]
  │
  ├── FunctionCall     → "call foo with x, y"
  │     name: "foo"
  │     arguments: ["x", "y"]
  │
  ├── Include          → "include stdio"
  │     header: "stdio.h"
  │     is_system: true
  │
  ├── Define           → "define MAX 100"
  │     name: "MAX"
  │     value: "100"
  │
  ├── PointerDecl      → "pointer to int x"
  ├── Dereference      → "dereference p"
  ├── AddressOf        → "address of x"
  ├── Malloc           → "allocate n integers"
  ├── Free             → "free p"
  ├── EnumDef          → "enum Color with red, green, blue"
  ├── Typedef          → "typedef int Number"
  ├── StringDecl       → "string s equals hello"
  ├── Comment          → "comment this does X"
  ├── Bitwise          → "x bitwise and y"
  ├── Cast             → "cast x to int"
  ├── Switch           → "switch expression"
  ├── Case             → "case 1"
  ├── Break / Continue → "break", "skip iteration"
  ├── MainFunction     → "create main", "entry point"
  └── Unknown          → Anything the extractor doesn't recognize
        original: "do something weird"
```

### How extraction works: a worked example

Input: `"declare an integer array called scores of size 10"`

```
  Token scan: ["declare", "an", "integer", "array", "called", "scores", "of", "size", "10"]
                │           │      │          │                   │                      │
                │           │      │          │                   │                      │
             TRIGGER      skip   type_hint  is_array=true       name                array_size
             keyword                ↓
           "declare"           "integer"

  Result: StatementHint::Declaration {
      names: ["scores"],
      type_hint: Some("integer"),   ← language-agnostic!
      qualifiers: [],
      initial_value: None,
      is_array: true,
      array_size: Some("10"),
  }
```

The type_hint `"integer"` is intentionally language-agnostic. When the rule-based translator or AI sees this, it will emit `int scores[10]` for C or `scores = [0] * 10` for Python.

### Hint extraction is purely Rust — no AI involved

This is a crucial architectural decision. The extractor uses:
- **Regex patterns** for multi-word phrases (`once_cell::Lazy<Regex>`)
- **Keyword prefix scanning** (checking for known trigger words like `"declare"`, `"set"`, `"if"`, etc.)
- **Positional token parsing** (what comes after the trigger word becomes the variable name, value, etc.)
- **Synonym expansion** via `src/synonyms.rs` (e.g. `"create"` → treated like `"declare"`, `"equals"` → treated like `"to"`)
- **Standard library matching** via `src/func_matcher.rs` (e.g. `"print formatted"` recognized as `printf`)

---

## Stage 3 — Variable Context Building

**File:** `src/hints.rs`  
**Struct:** `VariableContext`  
**Method:** `VariableContext::from_code(code_before: &str)`

Before validating the extracted hint, the pipeline builds a snapshot of what variables already exist in the surrounding code. This is done by scanning `code_before` — everything above the current line.

```
  code_before:
  ┌──────────────────────────────────────┐
  │  int score = 0;                      │  ─── declares "score" (int)
  │  int max = 100;                      │  ─── declares "max" (int)
  │  char name[50];                      │  ─── declares "name" (char[])
  │  for (int i = 0; i < max; i++) {    │  ─── declares "i" (int)
  └──────────────────────────────────────┘
                   │
                   ▼ VariableContext::from_code()
  ┌──────────────────────────────────────┐
  │  declared: {                         │
  │    "score" → "int",                  │
  │    "max"   → "int",                  │
  │    "name"  → "char",                 │
  │    "i"     → "int",                  │
  │  }                                   │
  └──────────────────────────────────────┘
```

The scanner uses regex to recognize common declaration patterns across C and Python:
- `int x`, `float y = 3.14`, `char buf[256]`
- `x = 5`, `x, y = 0, 1` (Python-style)
- `for (int i = ...`, `for i in ...`
- Struct field declarations
- Function parameter lists

---

## Stage 4 — Read Validation

**File:** `src/hints.rs`  
**Function:** `validate_reads(hint: &StatementHint, context: &VariableContext) -> ValidationResult`

Given the extracted hint and the variable context, the validator checks whether the hint attempts to **read** a variable that has not been declared yet in the visible code.

```
  Hint: Assignment { targets: ["result"], value: "score + bonus" }
  Context declared: { "score" }

  Reading: ["score", "bonus"]
            │           │
            ✓           ✗ "bonus" not in declared set
            │           │
            │           └─── ValidationResult {
            │                   is_valid: false,
            │                   error: "Variable 'bonus' used before declaration"
            │                }
            └───────────────
```

### What counts as a "read"

The validator inspects the hint variant to determine which identifiers are being **consumed** rather than **produced**:

| Hint Type | Variables checked as reads |
|---|---|
| `Assignment { value }` | All identifiers found in `value` |
| `Conditional { condition }` | All identifiers in `condition` |
| `Arithmetic { left, right }` | `left` and `right` |
| `Print { content }` | Identifiers in `content` (if not a string literal) |
| `Return { value }` | Identifiers in `value` |
| `Loop { start, end }` | `start` and `end` |
| `FunctionCall { arguments }` | All arguments |

Declarations are never validated as reads — `Declaration { names: ["x"] }` _produces_ `x`, it does not _consume_ it.

### Validation response

```
  is_valid: true  → Pipeline continues
  is_valid: false → 400 BAD_REQUEST
                    { kind: "error", message: "ERROR: Variable 'bonus' used before declaration" }
```

This catches the most common logical errors _before_ wasting an AI call on a request that is structurally invalid.

---

## Stage 5 — Trivial Path: Rule-Based Translation

**File:** `src/translate.rs`  
**Functions:** `translate_with_context()`, `translate_from_hint()`

When the extracted hint is **trivial** — meaning the rule-based translator can handle it with complete accuracy — the pipeline bypasses the AI entirely and generates code directly from the hint data.

### What makes a hint trivial?

```
  StatementHint::is_trivial() → true for:
  ┌─────────────────────────────────────────┐
  │  Else          → always trivial         │  "else"     → "} else {"
  │  EndBlock      → always trivial         │  "end"      → "}"
  │  Break         → always trivial         │  "break"    → "break;"
  │  Continue      → always trivial         │  "continue" → "continue;"
  │  DoWhileStart  → always trivial         │  "do"       → "do {"
  │  MainFunction  → always trivial         │  "main"     → "int main() {"
  │  Return(None)  → trivial                │  "return"   → "return;"
  │  Include       → trivial                │  "include stdio" → "#include <stdio.h>"
  │  Define        → trivial                │  "define MAX 100" → "#define MAX 100"
  └─────────────────────────────────────────┘

  is_trivial() → false for:
  ┌─────────────────────────────────────────┐
  │  Declaration with complex type hints    │
  │  Conditional with non-trivial condition │
  │  Loop with computed bounds              │
  │  Arithmetic with multiple operations    │
  │  FunctionDef with parameters            │
  │  Unknown (anything unrecognized)        │
  └─────────────────────────────────────────┘
```

### Context-aware rule translation

The rule-based translator is not naive. `translate_with_context()` receives the full `VariableContext` and uses it to make informed decisions:

```
  Hint: Declaration { names: ["x"], type_hint: Some("int"), initial_value: Some("0") }
  Language: "c"
  Context declared: { "x" → "int" }

  → "x" already declared! Emit assignment instead of re-declaration:
    x = 0;         ← context-aware, not:  int x = 0;  (which would be a redeclaration error)

  ─────────────────────────────────────────────────────────

  Hint: Assignment { targets: ["a", "b", "c"], value: "0" }
  Language: "c"

  → Multiple targets, same value:
    a = b = c = 0;   ← chained assignment

  ─────────────────────────────────────────────────────────

  Hint: Loop { iterator: Some("i"), start: Some("0"), end: Some("n") }
  Language: "c"

  → for (int i = 0; i < n; i++) {

  Hint: Loop { iterator: Some("i"), start: Some("0"), end: Some("n") }
  Language: "python"

  → for i in range(0, n):
```

The same `StatementHint` produces language-correct output for every target — the hint is the universal intermediate representation.

---

## Stage 6 — AI Path: Prompt Construction & Generation

**Files:** `src/prompt.rs`, `src/providers.rs`

When a hint is _not_ trivial, the pipeline sends it to an AI. But rather than sending raw English, it constructs a carefully engineered prompt that includes the structured hint data as explicit context.

### The PromptContext

```rust
pub struct PromptContext<'a> {
    pub english_line: &'a str,    // Original instruction
    pub hints: Option<StatementHint>,  // Extracted structural hints
    pub code_before: String,      // Last 1200 chars of preceding code
    pub code_after: String,       // First 600 chars of following code
    pub line_index: usize,        // Line number
    pub language: String,         // "c" / "python" / etc.
}
```

Notice: `code_before` is **tail-trimmed to 1200 chars** and `code_after` is **head-trimmed to 600 chars**. This is intentional — what came _immediately before_ the current line is more relevant than what came before that, so the tail of `code_before` is the most valuable context. The 1200:600 ratio weights recent history heavily without ballooning token counts.

```
  code_before (full):                     code_before (trimmed to tail 1200 chars):
  ┌───────────────────────┐               ┌───────────────────────┐
  │  // File header       │               │  ...                  │
  │  #include <stdio.h>   │               │  int score = 0;       │◄─ most relevant
  │  #include <string.h>  │               │  int max = 100;       │◄─
  │  ...50 lines...       │   ──────────► │  for (int i = 0;      │◄─
  │  int score = 0;       │               │    i < max; i++) {    │◄─ cursor is here
  │  int max = 100;       │               └───────────────────────┘
  │  for (int i = 0;      │               Kept: 1200 chars from the END
  │    i < max; i++) {    │
  └───────────────────────┘
```

### Prompt structure

The final prompt sent to the AI has two messages: a **system message** and a **user message**.

**System message** (constant, always the same):

```
You are a precise code translator that converts natural language instructions into code.

RULES:
1. Output ONLY the code for the given instruction - no explanations, comments, or markdown.
2. Use the EXTRACTED HINTS to understand the user's intent precisely.
3. Generate a single logical statement or construct.
4. Match the style and indentation of the surrounding code.
5. Never generate entire programs, multiple functions, or project-level code.
6. If the instruction is ambiguous, prefer the simplest interpretation.
7. Respect the target language syntax exactly.
```

**User message** (constructed per-request):

```
TARGET LANGUAGE: C
LINE NUMBER: 6

EXTRACTED HINTS:
  type: Conditional
  condition: "a > b"
  then_action: Some("print winner")
  else_action: None
  is_else_if: false

INSTRUCTION:
if a is greater than b, print winner

PRECEDING CODE:
int a = 5;
int b = 0;

FOLLOWING CODE:
```

The `EXTRACTED HINTS` section is generated by `StatementHint::to_prompt_hints()`, which serializes the typed enum data as human-readable key-value pairs. This tells the AI:

- **What kind of thing** it should generate (a Conditional, not a loop, not a declaration)
- **What variables are involved** (`a`, `b` — already known to be declared)
- **What the action should be** (`print winner` as the then-branch)

Without hints, the AI has to infer all of this from the English string alone, which introduces unnecessary ambiguity. With hints, the AI's job reduces to: _"generate the C syntax for this structured description."_

### Why hints dramatically improve AI accuracy

```
  Without hints:
  ──────────────
  AI sees:  "if a is greater than b, print winner"
  AI must decide:
    • Is "a" a variable or something else?     ← guessing
    • Is "winner" a variable or a string?      ← guessing
    • What type of construct is this?          ← guessing
    • Does "print" mean printf or puts?        ← guessing

  With hints:
  ───────────
  AI sees the hint:
    type: Conditional
    condition: "a > b"
    then_action: "print winner"
  AI knows:
    • This is an if-statement                  ← certain
    • Condition is a > b                       ← certain
    • Then-branch prints "winner"              ← certain
    • Variables a, b are already declared      ← certain (from context)
  AI only needs to decide:
    • printf("%s\n", ...) vs puts(...)         ← much smaller decision space
```

The accuracy improvement is particularly significant for:
- Ambiguous variable vs. literal detection (`"print hello"` — is `hello` a variable or a string literal?)
- Multi-name operations (`"set a, b, c to 0"`)
- Array operations with computed sizes
- Pointer and memory operations

---

## Stage 7 — Response Validation

**File:** `src/prompt.rs`  
**Function:** `validate_candidate(candidate: &str, language: &str) -> Result<()>`

The AI response is not trusted blindly. Before it is returned to the editor, it passes through a validation filter:

```
  AI response arrives
        │
        ▼
  ┌─────────────────────────────────────────────────────────────┐
  │  CHECK 1: Empty response?                                   │
  │  "" → Err("empty response")                                 │
  └──────────────────────────────┬──────────────────────────────┘
                                 │ pass
                                 ▼
  ┌─────────────────────────────────────────────────────────────┐
  │  CHECK 2: Markdown fences?                                  │
  │  Contains "```" → Err("response contains markdown fences")  │
  │  (AI sometimes wraps code in ```c ... ```)                  │
  └──────────────────────────────┬──────────────────────────────┘
                                 │ pass
                                 ▼
  ┌─────────────────────────────────────────────────────────────┐
  │  CHECK 3: Length limit?                                     │
  │  > 500 chars → Err("response exceeds 500 characters")       │
  │  (prevents runaway generation of entire programs)           │
  └──────────────────────────────┬──────────────────────────────┘
                                 │ pass
                                 ▼
  ┌─────────────────────────────────────────────────────────────┐
  │  CHECK 4: URLs?                                             │
  │  Contains "://" → Err("response contains URLs")             │
  │  (hallucination signal — real code doesn't have URLs)       │
  └──────────────────────────────┬──────────────────────────────┘
                                 │ pass
                                 ▼
  ┌─────────────────────────────────────────────────────────────┐
  │  CHECK 5: Language-specific validation                      │
  │                                                             │
  │  For C:                                                     │
  │    • Balanced braces { }                                    │
  │    • No console.log / console.error (JavaScript bleed)      │
  │    • No "def " without "#define" (Python bleed)             │
  │    • At most 1 function definition                          │
  │    • At most 1 struct definition                            │
  │                                                             │
  │  For Python:                                                │
  │    • No printf() or int main() (C bleed)                    │
  │    • At most 1 function definition                          │
  │    • At most 1 class definition                             │
  └──────────────────────────────┬──────────────────────────────┘
                                 │ pass
                                 ▼
                          Response accepted
                          → returned to editor
```

If any check fails, the result is `Err`, which causes `try_ai_translation_with_hints()` to return `None`, and the pipeline falls back to the rule-based translator.

---

## Provider Routing

**File:** `src/providers.rs`

The service initializes two provider instances at startup — one Gemini, one OpenAI — and holds them in shared `AppState`. Per-request, the correct provider is selected from the `provider` field in the payload:

```
  Request payload:
    provider: "deepseek"
    api_key:  "sk-..."
    model:    "deepseek-chat"
         │
         ▼
  ┌──────────────────────────────────────────────────────────────┐
  │  selected_provider = payload.provider                        │
  │      OR infer from model name:                               │
  │        "gpt-*" → "openai"                                    │
  │        other   → "gemini"                                    │
  └──────────────────────────────┬───────────────────────────────┘
                                 │
                ┌────────────────┴──────────────────┐
                │                                   │
         "openai"                              "gemini"
         "deepseek"   → openai_provider        → gemini_provider
         "other"
                │                                   │
                ▼                                   ▼
       OpenAI-compatible                     Google Gemini
       HTTP client                           HTTP client
       (POST /v1/chat/completions)           (generateContent API)
                │                                   │
                └────────────────┬──────────────────┘
                                 │
                          Per-request options
                          applied on top:
                          ┌───────────────────┐
                          │ api_key override  │  ← user's key from settings
                          │ model override    │  ← e.g. "deepseek-coder"
                          └───────────────────┘
```

DeepSeek uses the OpenAI-compatible provider because DeepSeek's API is a drop-in replacement for the OpenAI `/v1/chat/completions` format — the only difference is the base URL and the API key. "Other" also routes through the same path, allowing any OpenAI-compatible local or hosted model (Ollama, LM Studio, etc.).

---

## The Fallback Guarantee

One of the core design principles is: **the service never returns nothing useful**.

```
  Translation attempt flow:
  ─────────────────────────

  1. is_trivial?
       YES → Rule-based → Done. Never fails for recognized patterns.

  2. NOT trivial → Try AI
       AI succeeds → validate_candidate() passes → Done.
       AI fails (network, timeout, bad key) → continue to step 3.
       validate_candidate() fails (garbage output) → continue to step 3.

  3. Fallback: translate_with_context() (rule-based with context)
       Recognized hint variant → emit code → Done.
       Unrecognized (Unknown variant) → continue to step 4.

  4. Last resort: todo_placeholder()
       Language == "python" → "# TODO: <original instruction>"
       Language == "c"      → "// TODO: <original instruction>"
       Always succeeds. The user at least sees their original instruction
       preserved as a comment with a TODO marker.
```

The editor always gets _something_ back. The user is never left staring at a silent failure.

---

## Data Flow: End to End

Putting it all together with a concrete example:

**User types:** `"if a is greater than b, print winner"`  
**Target language:** C  
**code_before:** `int a = 5;\nint b = 0;\n`

```
  ┌─────────────────────────────────────────────────────────────────────┐
  │  POST /translate-line                                               │
  │  {                                                                  │
  │    english_line: "if a is greater than b, print winner",           │
  │    code_before: "int a = 5;\nint b = 0;\n",                        │
  │    code_after: "",                                                  │
  │    language: "c",                                                   │
  │    provider: "gemini",                                              │
  │    api_key: "AIza...",                                              │
  │    model: "gemini-2.5-flash"                                        │
  │  }                                                                  │
  └────────────────────────────┬────────────────────────────────────────┘
                               │
  Stage 1: Intent Classification
  classify("if a is greater than b, print winner")
    → "if" ∈ STATEMENT_INDICATORS ✓
    → Intent::Statement
                               │
  Stage 2: Hint Extraction
  hints::extract(payload)
    → Triggered by: "if" keyword
    → Condition extracted: "a > b" (via comparison synonym "greater than")
    → Then-action extracted: "print winner"
    → StatementHint::Conditional {
          condition: "a > b",
          then_action: Some("print winner"),
          else_action: None,
          is_else_if: false,
      }
                               │
  Stage 3: Variable Context
  VariableContext::from_code("int a = 5;\nint b = 0;\n")
    → declared: { "a" → "int", "b" → "int" }
                               │
  Stage 4: Validation
  validate_reads(Conditional { condition: "a > b" }, context)
    → reads: ["a", "b"]
    → "a" ∈ declared ✓
    → "b" ∈ declared ✓
    → ValidationResult { is_valid: true }
                               │
  Stage 5: Trivial check
  Conditional.is_trivial() → false (condition is non-trivial)
    → Skip rule-based path, proceed to AI
                               │
  Stage 6: AI Prompt Construction
  PromptContext::with_hints(payload, hint, 1200, 600)
    → code_before trimmed to last 1200 chars: "int a = 5;\nint b = 0;\n"
    → code_after trimmed to first 600 chars: ""
    → Prompt assembled:

      SYSTEM: "You are a precise code translator..."

      USER:
        TARGET LANGUAGE: C
        LINE NUMBER: 2

        EXTRACTED HINTS:
        type: conditional
        condition: "a > b"
        then_action: "print winner"

        INSTRUCTION:
        if a is greater than b, print winner

        PRECEDING CODE:
        int a = 5;
        int b = 0;

        FOLLOWING CODE:
        (empty)

  → Sent to Gemini Flash via HTTPS
  → Response: "if (a > b) { printf(\"winner\\n\"); }"
                               │
  Stage 7: Response Validation
  validate_candidate("if (a > b) { printf(\"winner\\n\"); }", "c")
    → Not empty ✓
    → No markdown fences ✓
    → 42 chars < 500 ✓
    → No URLs ✓
    → Braces balanced: 1 open, 1 close ✓
    → No console.log, no "def " ✓
    → 0 function definitions ✓
    → VALID

  ┌─────────────────────────────────────────────────────────────────────┐
  │  200 OK                                                             │
  │  {                                                                  │
  │    "kind": "ok",                                                    │
  │    "code": "if (a > b) { printf(\"winner\\n\"); }"                  │
  │  }                                                                  │
  └─────────────────────────────────────────────────────────────────────┘

  Editor replaces:  "if a is greater than b, print winner"
  With:             "if (a > b) { printf("winner\n"); }"
```

---

## Module Map

```
  rust-core/src/
  │
  ├── main.rs           Entry point. Starts Axum server. Owns AppState.
  │                     Routes POST /translate-line to handle_translate_line().
  │                     Orchestrates the full 7-stage pipeline.
  │
  ├── models.rs         Data transfer objects.
  │                     TranslateLineRequest  — incoming JSON shape
  │                     TranslateLineResponse — outgoing JSON shape
  │                     ResponseKind          — ok / error / unhandled enum
  │
  ├── intent.rs         Stage 1: Intent classifier.
  │                     classify() → Intent::Statement | Intent::Project
  │                     Keyword lists: PROJECT_VERBS, PROJECT_NOUNS,
  │                     STATEMENT_INDICATORS, MULTI_COMPONENT_INDICATORS
  │
  ├── hints.rs          Stage 2–4: The largest module (~7000 lines).
  │                     StatementHint enum — 40+ typed variants
  │                     extract() — English → StatementHint
  │                     VariableContext — tracks declared variables
  │                     validate_reads() — catches undeclared reads
  │                     is_trivial() — decides AI vs rule-based path
  │                     to_prompt_hints() — serializes hint for AI prompt
  │
  ├── translate.rs      Stage 5: Rule-based code generator.
  │                     translate_with_context() — hint + context → code
  │                     translate_from_hint() — hint → code (legacy)
  │                     CodeStyle — indent style, brace style, naming
  │                     Handles: C and Python output for all hint variants
  │
  ├── prompt.rs         Stage 6–7: AI prompt construction + validation.
  │                     PromptContext — assembles the full AI message pair
  │                     SYSTEM_PROMPT — constant instruction to the AI
  │                     to_user_prompt() — builds structured user message
  │                     validate_candidate() — rejects bad AI output
  │
  ├── providers.rs      AI provider abstraction.
  │                     Provider enum — Gemini | OpenAI | None
  │                     GeminiProvider — Google generateContent API client
  │                     OpenAIProvider — OpenAI /v1/chat/completions client
  │                     generate() — async trait method
  │                     AiRequestOptions — per-request key + model
  │
  ├── synonyms.rs       Synonym expansion table.
  │                     Maps natural language variants to canonical forms:
  │                       "create" → "declare"
  │                       "greater than" → ">"
  │                       "equals" → "="
  │                       "times" → "*"  etc.
  │
  ├── func_matcher.rs   Standard library function recognition.
  │                     Matches natural descriptions to stdlib functions:
  │                       "print formatted" → printf
  │                       "string length" → strlen
  │                       "open file" → fopen
  │
  ├── stdlib_db.rs      Static database of standard library functions.
  │                     FunctionSpec entries with parameter signatures,
  │                     return types, and required headers.
  │
  └── manpage_parser.rs Man page parsing utilities (future use).
                        Reserved for dynamic stdlib documentation lookup.
```

---

*Gradatim Rust Core — written in Rust, zero GC, sub-5ms local latency.*  
*Every English line becomes a typed fact before any AI ever sees it.*
