# Gradatim — Miscellaneous Technical Notes

> Everything that was built but didn't fit neatly into the README or RUSTCORE.md.  
> These are the decisions, subsystems, and hidden layers that make Gradatim work the way it does.

---

## Table of Contents

1. [The Synonym Engine](#the-synonym-engine)
2. [The Standard Library Database](#the-standard-library-database)
3. [Natural Language → stdlib Function Matching](#natural-language--stdlib-function-matching)
4. [Code Style Detection](#code-style-detection)
5. [The Monaco Editor Configuration](#the-monaco-editor-configuration)
6. [Untranslated Line Markers](#untranslated-line-markers)
7. [IPC Security Model](#ipc-security-model)
8. [PTY Terminal: A Real Shell](#pty-terminal-a-real-shell)
9. [Packaging Decisions](#packaging-decisions)
10. [The Automatic Rust Core Lifecycle](#the-automatic-rust-core-lifecycle)
11. [Settings Persistence](#settings-persistence)
12. [UI Theme System](#ui-theme-system)
13. [Portal-Style Dropdowns](#portal-style-dropdowns)
14. [Translated Line State Tracking](#translated-line-state-tracking)

---

## The Synonym Engine

**File:** `rust-core/src/synonyms.rs`

Before hint extraction even begins, every English instruction passes through a **synonym normalizer**. This is what allows Gradatim to understand an enormous range of natural language phrasings as the same underlying intent.

The engine is built around a static table of `(canonical, [synonyms...])` pairs — 476 lines of carefully curated mappings. Every synonym is mapped to a single canonical keyword that the hint extractor knows how to handle.

### Scale of the synonym table

| Category | Examples of synonyms accepted |
|---|---|
| **Declarations** | create, make, establish, instantiate, spin up, form, build, introduce, set up, allocate variable, initialize, new variable, prepare variable |
| **Assignments** | assign, store, put, place, make equal, let, give, set equal to, bind |
| **Control flow** | when / whenever / in case / provided that / assuming / should → `if` |
| **Loops** | iterate, repeat, cycle, go through, traverse, walk through, step through, run through, go over → `loop` |
| **Arithmetic** | plus, sum, combine, increase by, +, add together → `add` |
| **Comparisons** | bigger, larger, huger, higher, more, above, over, exceeding → `greater` |
| **Types** | integer, whole number, signed integer, number, numeric, count, counter → `int` |
| **Memory** | allocate, allocate memory, alloc, memory allocate, heap allocate → `malloc` |
| **Output** | output, display, show, write, log, echo, put, dump, emit → `print` |
| **Qualifiers** | constant, immutable, readonly, fixed → `const` |
| **Return** | give back, send back, yield, hand back → `return` |
| **Null** | nil, nothing, none, empty, void pointer → `null` |
| **Boolean** | true or false → `bool` |

### How the replacement works

Synonyms are not applied with a simple string replace. The engine uses **compiled regex patterns**, processed **longest-phrase-first** to prevent shorter synonyms from corrupting longer phrases:

```
  Input: "increase by 5"
          │
          ▼
  Phrase rules sorted by length (longest first):
    "increase by" → "add"       (10 chars)  ← matches, applied
    "increase"    → "increment" ( 8 chars)  ← would have matched incorrectly

  Without longest-first ordering:
    "increase" replaced first → "increment by 5"  ← wrong
  With longest-first ordering:
    "increase by" replaced first → "add 5"         ← correct

  Result: "add 5" → hint extractor sees "add"
```

Single-word synonyms use whole-word matching (`\b`) to prevent partial-word corruption:

```
  "assign" → "set"   with \bassign\b

  Input: "reassign x"  ← "reassign" contains "assign" but is not the whole word
    → NO match (correct — "reassign" would be an AI case anyway)
```

### Ordinal number normalization

Synonyms also handle ordinals, which appear in phrases like `"the first element"` or `"the 3rd item"`:

```
  "1st" → "first"
  "2nd" → "second"
  "3rd" → "third"
  ...through "10th" → "tenth"
```

This ensures `"get the 1st element"` and `"get the first element"` produce the same hint.

---

## The Standard Library Database

**File:** `rust-core/src/stdlib_db.rs`

Gradatim contains a hand-crafted static database of C standard library functions with full metadata. This is used by the function matcher to recognize natural English descriptions as calls to specific stdlib functions.

### Coverage

The database covers four headers with ~50+ functions:

**`stdio.h`**

| Function | Natural patterns recognized |
|---|---|
| `printf` | `print formatted`, `print format`, `print` |
| `fprintf` | `print to file`, `write formatted to file` |
| `scanf` | `scan input`, `read formatted` |
| `fscanf` | `read formatted from file` |
| `sscanf` | `scan from string`, `parse formatted string` |
| `fopen` | `open file`, `fopen` |
| `fclose` | `close file` |
| `fread` | `read file buffer` |
| `fwrite` | `write file buffer` |
| `fgets` | `read line`, `read line from file` |
| `getchar` | `read char`, `read character` |
| `putchar` | `write char`, `print character` |
| `fseek` | `seek file`, `move file pointer` |
| `ftell` | `file position`, `tell file position` |
| `rewind` | `rewind file`, `reset file position` |
| `fflush` | `flush file`, `flush output` |
| `feof` | `end of file`, `eof check` |
| `ferror` | `file error`, `error check` |

**`stdlib.h`**

| Function | Natural patterns recognized |
|---|---|
| `malloc` | `allocate`, `allocate bytes` |
| `calloc` | `allocate and zero` |
| `realloc` | `reallocate` |
| `free` | `deallocate`, `release memory` |
| `exit` | `exit with`, `quit with` |
| `rand` | `random`, `random number` |
| `srand` | `seed random` |
| `atoi` | `convert to int` |
| `atof` | `convert to float` |
| `strtol` | `convert to long` |
| `strtod` | `convert to double` |
| `qsort` | `sort array`, `quick sort` |
| `bsearch` | `binary search` |
| `system` | `run command`, `execute command` |
| `getenv` | `get environment`, `read env` |
| `abort` | `abort program`, `terminate immediately` |
| `atexit` | `on exit`, `register exit handler` |

**`string.h`**

| Function | Natural patterns recognized |
|---|---|
| `strlen` | `length of`, `string length` |
| `strcpy` | `copy string`, `copy string to` |
| `strncpy` | `copy n chars` |
| `strcat` | `concatenate`, `append string` |
| `strcmp` | `compare strings`, `string compare` |
| `strchr` | `find char`, `find character in` |
| `strstr` | `find substring`, `search string` |
| `memcpy` | `copy memory`, `copy bytes` |
| `memset` | `fill memory`, `set bytes` |
| `memmove` | `move memory`, `safe copy bytes` |

**`math.h`**

| Function | Natural patterns recognized |
|---|---|
| `sqrt` | `square root`, `root of` |
| `pow` | `power`, `raise to power` |
| `fabs` | `absolute value`, `magnitude` |
| `ceil` | `ceiling`, `round up` |
| `floor` | `floor`, `round down` |
| `round` | `round to nearest` |
| `log` | `natural log`, `logarithm` |
| `log10` | `log base 10` |
| `sin`, `cos`, `tan` | `sine`, `cosine`, `tangent` |
| `exp` | `exponential`, `e to the power` |

Each `FunctionSpec` entry carries the function name, required header, return type, parameter list with types and descriptions, and the list of natural-language patterns it responds to.

---

## Natural Language → stdlib Function Matching

**File:** `rust-core/src/func_matcher.rs`

The function matcher is what connects the synonym-expanded English instruction to a concrete stdlib call. It uses the database above plus a scan of the `#include` directives in `code_before` to determine which functions are in scope.

```
  Input: "print formatted: score is %d"
  Included headers (from code_before scan): { "stdio.h" }
         │
         ▼
  ┌────────────────────────────────────────────────────────────┐
  │  For each FunctionSpec where spec.header ∈ included:       │
  │    printf.header == "stdio.h" ✓                            │
  │                                                            │
  │  Check patterns:                                           │
  │    "print formatted" → line starts with "print formatted" ✓│
  │                                                            │
  │  → match_natural(printf_spec, line)                        │
  │    parse_printf("score is %d")                             │
  │    → fmt: "score is %d", args: ["score"]                   │
  │                                                            │
  │  FunctionMatch { name: "printf", args: ["score is %d", "score"] }
  └────────────────────────────────────────────────────────────┘
         │
         ▼
  Rule-based emits: printf("score is %d", score);
```

This means instructions like:
- `"print formatted: hello %s"` → `printf("hello %s", ...)`
- `"read formatted"` → `scanf(...)`
- `"open file"` → `fopen(...)`
- `"binary search"` → `bsearch(...)`
- `"sort array"` → `qsort(...)`

...all resolve _without an AI call_, directly from the static database. Only functions whose required header is actually included in `code_before` are eligible — preventing nonsense matches from functions the user hasn't set up.

---

## Code Style Detection

**File:** `rust-core/src/translate.rs`

The rule-based translator does not blindly apply one formatting style. It exposes a `CodeStyle` struct that can adapt output to match the surrounding code:

```rust
pub struct CodeStyle {
    pub indent_style: IndentStyle,    // Tabs or Spaces
    pub indent_width: usize,          // 2, 4, 8...
    pub brace_style: BraceStyle,      // K&R, Allman, GNU, Whitesmiths
    pub max_line_length: usize,       // soft wrap hint
    pub naming_convention: NamingConvention, // camelCase, snake_case, PascalCase
}
```

### Brace styles understood

```
  K&R (default):          Allman:                GNU:
  if (x > 0) {            if (x > 0)             if (x > 0)
      ...                 {                        {
  }                           ...                      ...
                          }                        }

  Whitesmiths:
    if (x > 0)
        {
        ...
        }
```

The default style is **K&R with 4-space indentation and snake_case naming**, matching the most common C convention. In a future version, style detection will scan `code_before` to infer which style the existing code uses and match it automatically — the infrastructure is already in place.

---

## The Monaco Editor Configuration

**File:** `renderer/src/main.ts`

Gradatim enables a non-default set of Monaco features that go beyond a vanilla install. Every one of these was explicitly turned on:

| Option | Value | Effect |
|---|---|---|
| `bracketPairColorization.enabled` | `true` | Each nesting depth of `()`, `{}`, `[]` gets its own color |
| `smoothScrolling` | `true` | Animated scroll instead of instant jump |
| `stickyScroll.enabled` | `true` | Current function/block signature pins to the top of the viewport |
| `linkedEditing` | `true` | Rename both sides of a matching pair simultaneously |
| `wordWrap` | `"off"` | Horizontal scroll by default (user-configurable) |
| `lineNumbers` | `"on"` | Gutter line numbers (user-configurable) |
| `renderWhitespace` | `"none"` | Hidden by default (user-configurable) |
| `cursorStyle` | `"line"` | Thin vertical cursor (user-configurable) |
| `tabSize` | `4` | Default indent width (user-configurable) |
| `insertSpaces` | `true` | Tab key inserts spaces (user-configurable) |
| `fontFamily` | `"Consolas"` | Default monospace font (user-configurable) |
| `fontSize` | `14` | Default font size (user-configurable) |

### Three custom Monaco themes

Gradatim defines three custom Monaco themes — not just light/dark system toggles, but hand-tuned color palettes designed to match the UI glass aesthetic:

- **`gradatim-light`** — White background with purple/blue token colors, soft contrast
- **`gradatim-dark`** — Deep navy background with electric blue/purple highlights, matching the dark UI panels
- **`gradatim-glass`** — Transparent background (`rgba(0,0,0,0)`) so the frosted glass panel shows through, with high-contrast tokens

When the user switches the UI theme, `applyTheme()` calls `monaco.editor.setTheme()` in sync — the editor and the surrounding shell always match.

---

## Untranslated Line Markers

**File:** `renderer/src/main.ts`

Every line that contains natural English text (i.e., has not yet been translated) gets a **pulsing purple gutter decoration** in the editor. This is implemented using Monaco's Decoration API, not any external plugin.

### How the tracking works

Each open tab carries a `translatedLines: Set<number>` — a set of line numbers that have already been successfully translated. Lines in the file that are:

1. Not in `translatedLines`
2. Not empty
3. Not "ignorable" (punctuation-only lines like `{`, `}`, `;`, `//...`, `#...`)

...get a decoration applied. The gutter shows an animated purple dot (CSS `@keyframes pulseGutter`).

### Intelligent line number remapping

When the user inserts or deletes lines, the translated line set does not just get cleared. Instead, Monaco's `onDidChangeModelContent` event provides the exact range of changes, and the set is **remapped** to compensate:

```
  Before edit:            translatedLines = { 3, 5, 7 }

  User inserts 2 lines at line 4:

  After edit:             Lines 3 stays at 3 (above insertion)
                          Lines 5 and 7 shift up by 2 → { 7, 9 }

  Result:                 translatedLines = { 3, 7, 9 }
```

This means previously translated lines don't re-acquire markers just because new lines were added above them — the markers stay accurate as the file evolves.

### Ignorable line detection

The following patterns are never marked as untranslated, even if they don't appear in `translatedLines`, because they are structural scaffolding rather than translatable instructions:

- Lines containing only `{` or `}` (block delimiters)
- Lines starting with `//` (single-line comments)
- Lines starting with `#` (preprocessor directives already emitted)
- Blank lines

---

## IPC Security Model

**File:** `electron/preload.js`

Gradatim follows Electron's security best practices by running the renderer with **`contextIsolation: true`** and **`nodeIntegration: false`**. The renderer process has no direct access to Node.js APIs.

All renderer-to-main communication goes through a tightly scoped `electronAPI` bridge exposed via `contextBridge.exposeInMainWorld`:

```
  Renderer (untrusted)           Preload (bridge)            Main process (trusted)
  ─────────────────              ──────────────              ──────────────────────
  window.electronAPI             contextBridge               ipcMain.handle(...)
    .translateLine(payload)  ──► .exposeInMainWorld ──────► 'translate-line'
    .listDirectory()         ──► 'electronAPI'      ──────► 'fs:list'
    .readFile(path)          ──►                    ──────► 'fs:read-file'
    .writeFile(...)          ──►                    ──────► 'fs:write-file'
    .saveFileAs(...)         ──►                    ──────► 'fs:save-as'
    .openFolderDialog()      ──►                    ──────► 'dialog:open-folder'
    .openFileDialog()        ──►                    ──────► 'dialog:open-file'
    .loadSettings()          ──►                    ──────► 'settings:load'
    .saveSettings(data)      ──►                    ──────► 'settings:save'
    .window.minimize()       ──►                    ──────► 'window:minimize'
    .window.maximize()       ──►                    ──────► 'window:maximize'
    .window.close()          ──►                    ──────► 'window:close'
```

The renderer cannot call arbitrary Node.js functions. It can only call the named functions in the bridge, and the main process validates each one before acting.

### Path traversal protection

Every file-system IPC handler runs incoming paths through `resolveWorkspacePath()`, which:

1. Resolves the path relative to the current `workspaceRoot`
2. Checks that the resolved absolute path still starts with `workspaceRoot`
3. Throws `"Path escapes workspace boundary"` if it doesn't

```
  workspaceRoot: /home/user/project

  Attempt: readFile("../../etc/passwd")
    resolved: /etc/passwd
    /etc/passwd.startsWith("/home/user/project") → false
    → BLOCKED
```

This prevents a malicious or buggy renderer from reading files outside the open workspace.

---

## PTY Terminal: A Real Shell

**Files:** `electron/pty-manager.js`, `renderer/src/terminal/`

The Gradatim terminal is **not a fake terminal**. It uses [`node-pty`](https://github.com/microsoft/node-pty) to create genuine PTY (pseudoterminal) sessions — the same mechanism real terminal emulators use. This means:

- The shell (`cmd.exe` / PowerShell on Windows, `bash`/`zsh` on Linux/Mac) runs as a real child process
- `Ctrl+C` sends `SIGINT` to the foreground process, not to Gradatim
- Programs that check whether stdout is a TTY (like `ls --color`, `git log`, compilers with colored output) see a real terminal and behave accordingly
- Arrow-key history navigation, tab completion, and readline work as expected

### Multiple sessions

Each terminal tab is a separate `TerminalInstance` wrapping a separate `node-pty` process. Sessions are tracked in `ptyManager` on the main process and cleaned up on window close:

```
  Tab 1 → pty session #1 (PowerShell)   cwd: C:\Users\Owner\project
  Tab 2 → pty session #2 (PowerShell)   cwd: C:\Users\Owner\project
  Tab 3 → pty session #3 (PowerShell)   cwd: C:\Users\Owner\project

  All killed on app.on('window-all-closed')
```

### xterm.js + FitAddon + WebLinksAddon

The visual terminal is rendered with [`xterm.js`](https://xtermjs.org/) with two addons:

- **FitAddon** — automatically resizes the terminal's column/row count to match the panel size when the user drags the splitter
- **WebLinksAddon** — makes URLs in terminal output clickable

### Theme-aware color palettes

`TerminalInstance` implements `setTheme(theme: 'light' | 'dark' | 'glass')` which updates xterm's color palette in real time when the user switches UI themes. Three hand-tuned palettes are defined in `getThemePalette()`:

- **Light**: Off-white background, dark text, blue cursor, purple selection
- **Dark**: Deep navy background, bright white text, cyan cursor
- **Glass**: Near-transparent background with high-contrast text for readability over the frosted panel

---

## Packaging Decisions

**Files:** `electron-builder.yml`, `package.json`

Several non-obvious decisions were made during the packaging setup:

### `asar: false`

By default electron-builder packs all app files into an `.asar` archive (similar to a zip). This was deliberately disabled:

```yaml
asar: false
```

Reason: the Rust binary (`rust-core.exe`) is spawned as a child process using `spawn()`. On Windows, you **cannot `spawn()` a process from inside an asar archive** — the OS needs the binary to be a real file on disk. Setting `asar: false` keeps all files as loose files in the installed `resources/` directory.

### `extraResources` for the Rust binary

```yaml
extraResources:
  - from: rust-core/target/release/rust-core.exe
    to: rust-core/rust-core.exe
```

This tells electron-builder to copy the compiled Rust binary into the installed app's `resources/rust-core/` directory. The main process then looks for it at `process.resourcesPath + "/rust-core/rust-core.exe"`, which resolves correctly regardless of where the user installs the app.

### `electron-rebuild` for `node-pty`

`node-pty` is a native Node.js addon — it contains compiled C++ code that must match the exact Electron version being used. The `postinstall` script handles this automatically:

```json
"postinstall": "electron-rebuild --force --only node-pty"
```

This runs immediately after `npm install` and recompiles `node-pty` against the installed Electron version. Without it, the terminal would crash on launch with a "native module was compiled against a different version" error.

### Build pipeline

```
  npm run package:win
        │
        ├── npm run build:renderer   (Vite → renderer/dist/)
        │
        ├── npm run build:rust       (cargo build --release → rust-core.exe)
        │
        ├── npm run build:icon       (PNG → ICO via png-to-ico)
        │
        └── electron-builder --win nsis
              │
              ├── Copies renderer/dist/ → resources/renderer/dist/
              ├── Copies electron/*.js  → resources/electron/
              ├── Copies rust-core.exe  → resources/rust-core/rust-core.exe
              ├── Converts build/icon.ico → embeds in .exe
              └── Produces release/Gradatim Setup 0.1.0.exe
```

---

## The Automatic Rust Core Lifecycle

**File:** `electron/main.js`

The Rust core is not a separate thing you run manually. It is managed entirely by the Electron main process.

### Startup sequence

```
  app.whenReady()
        │
        ├── configStore initialized
        │
        ├── void ensureRustCoreReady()
        │     │
        │     ├── isRustCoreResponsive()?
        │     │     │ POST / with 1200ms timeout
        │     │     │
        │     │     ├── YES → already running (dev mode reuse) → done
        │     │     │
        │     │     └── NO → startRustCoreProcess()
        │     │               │
        │     │               ├── isDev?
        │     │               │     YES: spawn powershell build.ps1 run
        │     │               │     NO:  resolvePackagedRustCoreBinary()
        │     │               │            → process.resourcesPath/rust-core/rust-core.exe
        │     │               │
        │     │               └── spawn binary, unref() so it doesn't keep Electron alive
        │     │
        │     └── waitForRustCoreReady()
        │           polls isRustCoreResponsive() every 500ms
        │           up to 30 seconds
        │           returns true/false
        │
        └── createMainWindow() → renderer loads → user sees the app
```

### Every translation waits for readiness

Before any `translate-line` IPC call is forwarded to the Rust service, the handler checks:

```js
const ready = await ensureRustCoreReady();
if (!ready) {
  return { kind: 'error', message: 'Rust core is starting. Please try again in a moment.' };
}
```

If the server is still starting (e.g., the user pressed Enter within the first second of app launch), the call gracefully fails with a human-readable message rather than a connection error.

### Shutdown

When the app window closes, the Rust core process is explicitly killed:

```js
app.on('window-all-closed', () => {
  if (rustCoreProcess && !rustCoreProcess.killed) {
    rustCoreProcess.kill();
  }
});
```

No orphaned processes are left running after the app closes.

---

## Settings Persistence

**File:** `electron/config-store.js`

User settings are stored in the OS-native user data directory using Electron's `app.getPath('userData')`. The exact path is platform-specific:

| Platform | Path |
|---|---|
| Windows | `%APPDATA%\gradatim-editor\config.json` |
| Linux | `~/.config/gradatim-editor/config.json` |
| macOS | `~/Library/Application Support/gradatim-editor/config.json` |

The config file uses a `deepMerge` strategy: when loading, the stored settings are merged over the defaults rather than replacing them. This means **new settings added in future versions of the app automatically get their default values** for existing users — no migration scripts needed.

### What is persisted

```json
{
  "targetLanguage": "c",
  "autoTranslate": true,
  "maxLinesPerTranslation": 50,
  "context": {
    "beforeChars": 500,
    "afterChars": 200
  },
  "ai": {
    "provider": "gemini",
    "apiKey": "...",
    "model": "gemini-2.5-flash"
  },
  "editor": {
    "tabSize": 4,
    "insertSpaces": true,
    "fontSize": 14,
    "fontFamily": "Consolas",
    "renderWhitespace": "none",
    "cursorStyle": "line",
    "wordWrap": "off",
    "lineNumbers": "on"
  },
  "ui": {
    "theme": "light"
  }
}
```

Settings are saved every time the user clicks **Save** in the settings modal and reloaded on next launch.

---

## UI Theme System

**File:** `renderer/src/style.css`, `renderer/src/main.ts`

All three themes (Light, Dark, Glass) are driven entirely by **CSS custom properties** (variables). Switching themes sets `data-theme` on `<body>`, and the CSS cascade handles everything else automatically — no JavaScript style manipulation, no class toggling per-element.

```
  :root {                              ← Light theme (default)
    --bg: rgba(255,255,255,0.85);
    --panel-bg: rgba(240,243,255,0.75);
    --text: #1a1a2e;
    --accent: #6c63ff;
    ...
  }

  body[data-theme='dark'] {           ← Dark overrides
    --bg: rgba(10,12,28,0.92);
    --panel-bg: rgba(20,23,45,0.82);
    --text: #e8eaf6;
    --accent: #7c6fff;
    ...
  }

  body[data-theme='glass'] {          ← Glass overrides
    --bg: rgba(255,255,255,0.08);
    --panel-bg: rgba(255,255,255,0.05);
    --text: rgba(255,255,255,0.92);
    ...
  }
```

Every color in the app — borders, shadows, scrollbars, terminal backgrounds, button states, input fields — references one of these variables. Adding a fourth theme would require changing only one CSS block.

`backdrop-filter: blur(20px) saturate(180%)` is applied to all major panels, producing the frosted glass effect in all three themes (most visible in Glass mode where the background is nearly transparent).

---

## Portal-Style Dropdowns

**File:** `renderer/src/main.ts`, `renderer/src/style.css`

The target language dropdown and the top-bar File/Edit/View/Terminal menus are rendered using a **portal pattern**: they are dynamically moved to `document.body` rather than staying as children of their trigger buttons.

This was necessary because Monaco Editor creates its own stacking context with a high `z-index`. Any dropdown rendered _inside_ the editor's DOM ancestor would appear beneath the editor canvas, no matter how high its own `z-index` was.

```
  Problem without portals:
  ┌─────────────────────────────────────────┐
  │  .editor-panel  (z-index: 1)            │
  │  ┌────────────────────────────────┐     │
  │  │  Monaco canvas  (z-index: 999) │     │
  │  │                                │◄────── Dropdown rendered INSIDE editor-panel
  │  │  [language-menu]               │     │  appears BEHIND this canvas
  │  └────────────────────────────────┘     │
  └─────────────────────────────────────────┘

  Solution with portals:
  ┌─────────────────────────────────────────┐
  │  document.body                          │
  │  ┌────────────────────────────────┐     │
  │  │  Monaco canvas  (z-index: 999) │     │
  │  └────────────────────────────────┘     │
  │  [language-menu]  (position: fixed;     │◄── Appended directly to body
  │                    z-index: 10000)      │    appears ABOVE everything
  └─────────────────────────────────────────┘
```

When a dropdown opens:
1. The menu element is `document.body.appendChild(menu)`
2. `getBoundingClientRect()` is called on the trigger button to get its screen position
3. The menu is positioned with `style.top/left` to align with the button
4. A global `click` listener closes it if the user clicks anywhere else
5. A `resize` listener repositions it if the window is resized while open

On close, the element is removed from `body` to avoid polluting the DOM.

---

## Translated Line State Tracking

This goes beyond what RUSTCORE.md covered — here is the full state machine for line tracking in the renderer.

Each `EditorTab` object carries:

```typescript
interface EditorTab {
  // ...
  translatedLines: Set<number>;  // 1-based line numbers that have been translated
}
```

### State transitions

```
  ┌─────────────────────────────────────────────────────────────────┐
  │  User types a line and presses Enter                            │
  │  → autoTranslate fires → translateCurrentLine()                 │
  │    → API call succeeds                                          │
  │    → markTranslatedLineRange(startLine, endLine)               │
  │    → translatedLines.add(lineNumber)                           │
  │    → refreshUntranslatedDecorations()   ← purple dot removed   │
  └─────────────────────────────────────────────────────────────────┘

  ┌─────────────────────────────────────────────────────────────────┐
  │  User edits a previously translated line                        │
  │  → onDidChangeModelContent fires                                │
  │  → invalidateTranslatedLines(changes)                           │
  │    → For each changed range:                                    │
  │        translatedLines.delete(changedLineNumber)               │
  │    → refreshUntranslatedDecorations()   ← purple dot re-appears │
  └─────────────────────────────────────────────────────────────────┘

  ┌─────────────────────────────────────────────────────────────────┐
  │  User inserts new lines above translated content                │
  │  → invalidateTranslatedLines(changes)                           │
  │    → Remap: lines below insertion point shift up by delta       │
  │    → translatedLines stays accurate for all existing content    │
  └─────────────────────────────────────────────────────────────────┘

  ┌─────────────────────────────────────────────────────────────────┐
  │  User switches tabs                                             │
  │  → Current tab's translatedLines preserved in EditorTab object  │
  │  → New tab's decorations applied from its own translatedLines   │
  └─────────────────────────────────────────────────────────────────┘
```

This means the marker state is **per-tab** and **survives tab switches** — switching to a different file and back does not reset which lines have been marked as translated.
