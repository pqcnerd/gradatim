# Prompting strategy

The Gradatim editor sends **one natural-language line at a time** to the AI model. To keep the model grounded, we use a short sliding window of surrounding code (before and after the target line) plus the most recent translated lines.

## Message template

```
System:
You are a compiler that converts a single line of natural-language instructions into minimal, syntactically valid C code. Output only the code needed for that line—no explanations, comments, or extra context. Never modify lines other than the one requested.

User:
TARGET LANGUAGE: C
CURSOR LINE NUMBER: {line_index}
CURRENT LINE (English):
{english_line}

PRECEDING CODE (truncated):
{code_before}

FOLLOWING CODE (truncated):
{code_after}

Expectations:
1. Use identifiers/types that already exist in the surrounding code whenever possible.
2. Emit only the minimal code for this instruction, **up to three lines** (prefer one line when possible).
3. If the English line opens a block (if/for/while/function), emit the full skeleton: opening brace/colon, an indented body placeholder, and the closing brace.
4. Do not add `#include` lines unless the user explicitly asks for them.
5. Never emit Markdown fences, comments, or blank descriptions—only raw code for the requested language.
```

## Few-shot examples

| English line | Target C snippet |
|--------------|------------------|
| `declare a, b 0` | `int a = 0, b = 0;` |
| `set a to 5` | `a = 5;` |
| `if a is greater than b:` | `if (a > b) {` |
| `else:` | `} else {` |
| `loop from i = 0 to 10 printing i` | `for (int i = 0; i < 10; i++) {\n    printf("%d\\n", i);\n}` |
| `loop from i = 0 to 10 printing i` *(Python)* | `for i in range(0, 10):\n    print(i)` |
| `end loop` | `}` |

These examples should be injected as part of the conversation history so the model mirrors the desired brevity.

## Validation heuristics

Before accepting the model output, run the following checks:

1. **Pure code** — reject if the string contains ``` fences, markdown indicators, or English sentences.
2. **Line budget** — cap to ~200 characters / 3 lines to keep the response scoped.
3. **Brace sanity** — ensure braces are balanced and that block openers end with `{` if present.
4. **Statement endings** — non-block statements should end with `;`.
5. **Language tag** — scan for obvious non-C keywords (e.g., `def`, `console.log`) and reject.

Failed checks should prompt a retry (with a shorter max token limit) or surface an inline error so the user can adjust the instruction manually.

