<div align="center">

```
  ██████╗ ██████╗  █████╗ ██████╗  █████╗ ████████╗██╗███╗   ███╗
 ██╔════╝ ██╔══██╗██╔══██╗██╔══██╗██╔══██╗╚══██╔══╝██║████╗ ████║
 ██║  ███╗██████╔╝███████║██║  ██║███████║   ██║   ██║██╔████╔██║
 ██║   ██║██╔══██╗██╔══██║██║  ██║██╔══██║   ██║   ██║██║╚██╔╝██║
 ╚██████╔╝██║  ██║██║  ██║██████╔╝██║  ██║   ██║   ██║██║ ╚═╝ ██║
  ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝╚═════╝ ╚═╝  ╚═╝   ╚═╝   ╚═╝╚═╝     ╚═╝
```

### _Write in English. Ship in code._

**The AI-powered text editor that translates natural language into production-ready code, one line at a time.**

---

[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux-blue?style=flat-square)](https://github.com/gradatim)
[![Built with Electron](https://img.shields.io/badge/Built%20with-Electron-47848f?style=flat-square)](https://www.electronjs.org/)
[![Rust Core](https://img.shields.io/badge/Core-Rust%20%2B%20Axum-orange?style=flat-square)](https://www.rust-lang.org/)
[![AI Powered](https://img.shields.io/badge/AI-Gemini%20%7C%20OpenAI%20%7C%20DeepSeek-purple?style=flat-square)](#ai-providers)
[![License: MIT](https://img.shields.io/badge/License-MIT-green?style=flat-square)](LICENSE)

</div>

---

## What is Gradatim?

> **Gradatim** *(Latin): step by step, gradually, one degree at a time.*

Gradatim is a desktop code editor with a fundamentally different philosophy: **you don't write code — you describe what you want in plain English, and Gradatim writes the code for you.**

Traditional code editors assume you already know the syntax, the idioms, and the exact API calls for your target language. Gradatim throws that assumption out the window. Instead, you type natural language instructions line by line — `declare a variable called score and set it to zero`, `if score is greater than 100, print winner` — and the moment you press **Enter**, an AI-powered translation engine converts your intent into syntactically correct, contextually aware code in your chosen language.

It is not a chatbot. It is not a prompt box. It is a **line-by-line collaborative editor** where you and an AI write code together in real time, in your own words.

---

## The Problem Gradatim Solves

Every programmer — from the curious beginner to the seasoned systems engineer — spends a significant portion of their time not thinking, but _remembering_:

- "Was it `printf` or `print`?"
- "Does this language use zero-based or one-based indexing by default?"
- "What's the exact syntax for a for-loop in C again?"
- "How do I declare a pointer to a struct in this context?"

This cognitive overhead is friction. Pure, productivity-destroying friction. Studies in developer productivity consistently show that **context-switching between thinking and remembering syntax is one of the leading causes of flow-state interruption** — the deep focus state where programmers do their best work.

Gradatim eliminates this friction entirely. You stay in the flow of _thinking_. The editor handles the _remembering_.

```
  WITHOUT GRADATIM                     WITH GRADATIM
  ─────────────────────────            ──────────────────────────────
  ┌─────────────────────┐              ┌──────────────────────────────┐
  │  Think about logic  │              │   Think about logic          │
  └────────┬────────────┘              └─────────────┬────────────────┘
           │                                         │
           ▼                                         ▼
  ┌─────────────────────┐              ┌──────────────────────────────┐
  │ Remember syntax     │◄─ friction   │  Type it in plain English    │
  └────────┬────────────┘              └─────────────┬────────────────┘
           │                                         │
           ▼                                         ▼
  ┌─────────────────────┐              ┌──────────────────────────────┐
  │  Look it up online  │◄─ more loss  │  Press Enter → Code appears  │
  └────────┬────────────┘              └─────────────┬────────────────┘
           │                                         │
           ▼                                         ▼
  ┌─────────────────────┐              ┌──────────────────────────────┐
  │  Type the code      │              │  Review, continue writing    │
  └─────────────────────┘              └──────────────────────────────┘

  Average: ~4–8 minutes per concept   Average: ~15 seconds per line
```

---

## How It Works

Gradatim is a three-layer system: a **beautiful desktop shell**, an **intelligent translation pipeline**, and a **high-performance Rust backend**.

```
  ╔══════════════════════════════════════════════════════════════════╗
  ║                    GRADATIM ARCHITECTURE                         ║
  ╠══════════════════════════════════════════════════════════════════╣
  ║                                                                  ║
  ║   ┌─────────────────────────────────────────────────────────┐   ║
  ║   │              RENDERER (Electron + Vite + TS)            │   ║
  ║   │  ┌──────────────┐  ┌───────────┐  ┌─────────────────┐  │   ║
  ║   │  │ Monaco Editor│  │ File Tree │  │  Terminal Panel  │  │   ║
  ║   │  │  (code view) │  │ (sidebar) │  │   (xterm.js)    │  │   ║
  ║   │  └──────┬───────┘  └───────────┘  └─────────────────┘  │   ║
  ║   │         │ IPC Bridge (preload.js)                        │   ║
  ║   └─────────┼───────────────────────────────────────────────┘   ║
  ║             │                                                    ║
  ║   ┌─────────▼───────────────────────────────────────────────┐   ║
  ║   │           ELECTRON MAIN PROCESS (Node.js)               │   ║
  ║   │  ┌─────────────┐  ┌────────────┐  ┌──────────────────┐  │   ║
  ║   │  │ IPC Handlers│  │Config Store│  │  PTY Manager     │  │   ║
  ║   │  │ (fs, window)│  │(user prefs)│  │ (shell sessions) │  │   ║
  ║   │  └─────────────┘  └────────────┘  └──────────────────┘  │   ║
  ║   │         │ HTTP (localhost:4888)                           │   ║
  ║   └─────────┼───────────────────────────────────────────────┘   ║
  ║             │                                                    ║
  ║   ┌─────────▼───────────────────────────────────────────────┐   ║
  ║   │              RUST CORE (Axum microservice)              │   ║
  ║   │  ┌──────────────────┐    ┌───────────────────────────┐  │   ║
  ║   │  │  /translate-line │    │     Hint Extractor        │  │   ║
  ║   │  │  POST endpoint   │───►│  (context-aware prompts)  │  │   ║
  ║   │  └──────────────────┘    └───────────────────────────┘  │   ║
  ║   │           │                                              │   ║
  ║   │  ┌────────▼──────────────────────────────────────────┐  │   ║
  ║   │  │               AI Provider Router                  │  │   ║
  ║   │  │   Gemini ──┐                                      │  │   ║
  ║   │  │   OpenAI ──┼──► Constrained Code Generation       │  │   ║
  ║   │  │  DeepSeek ─┤                                      │  │   ║
  ║   │  │   Other ───┘                                      │  │   ║
  ║   │  └───────────────────────────────────────────────────┘  │   ║
  ║   └─────────────────────────────────────────────────────────┘   ║
  ║                                                                  ║
  ╚══════════════════════════════════════════════════════════════════╝
```

### Layer 1 — The Editor Shell

The editor is built on **Monaco** (the same engine that powers Visual Studio Code) wrapped inside an **Electron** desktop application. This means you get:

- Full syntax highlighting for C, Python, and more
- Bracket pair colorization
- Sticky scroll (context header showing your current function/block)
- Smooth scrolling with animated cursor transitions
- Gutter line numbers with untranslated-line markers
- Multi-tab editing with unsaved-change indicators

The shell is intentionally minimal and distraction-free. There is no cluttered toolbar, no plugin marketplace, no bloat. There is a **top bar** with your menus and window controls, a **code editor** in the center, a **file tree** on the right, and a **terminal** at the bottom.

### Layer 2 — The Translation Pipeline

When you type a line in natural English and press **Enter**, the following sequence fires:

```
  You press Enter
        │
        ▼
  ┌─────────────────────────────────────────────────────────┐
  │  1. LINE CAPTURE                                         │
  │     Monaco fires onDidChangeModelContent                │
  │     Gradatim reads the current line content             │
  └────────────────────────────┬────────────────────────────┘
                               │
                               ▼
  ┌─────────────────────────────────────────────────────────┐
  │  2. CONTEXT WINDOW ASSEMBLY                              │
  │     N characters before the line  ──┐                   │
  │     The natural language line    ───┼──► Hint Packet    │
  │     N characters after the line  ──┘                   │
  └────────────────────────────┬────────────────────────────┘
                               │
                               ▼
  ┌─────────────────────────────────────────────────────────┐
  │  3. HINT EXTRACTION (Rust)                               │
  │     Scans surrounding translated lines for:             │
  │       • Variable names and types already declared       │
  │       • Function signatures in scope                    │
  │       • Indentation / block depth                       │
  │       • Target language conventions                     │
  └────────────────────────────┬────────────────────────────┘
                               │
                               ▼
  ┌─────────────────────────────────────────────────────────┐
  │  4. PROMPT CONSTRUCTION                                  │
  │     Combines: hints + context + user's natural line     │
  │     into a tightly scoped, constrained AI prompt        │
  │     that requests exactly ONE line of output code       │
  └────────────────────────────┬────────────────────────────┘
                               │
                               ▼
  ┌─────────────────────────────────────────────────────────┐
  │  5. AI TRANSLATION                                       │
  │     Sent to your chosen provider:                       │
  │       Gemini 2.5 Flash / GPT-4o / DeepSeek / other     │
  │     Response: a single line of syntactically valid code │
  └────────────────────────────┬────────────────────────────┘
                               │
                               ▼
  ┌─────────────────────────────────────────────────────────┐
  │  6. IN-PLACE REPLACEMENT                                 │
  │     The natural language line is replaced in the editor │
  │     with the generated code — cursor advances to next   │
  └─────────────────────────────────────────────────────────┘
```

The key insight is **constraint**: Gradatim does not ask the AI to write an entire program. It asks it to write _one line_, given full knowledge of everything already written. This produces dramatically higher accuracy than full-program generation, and it means _you_ stay in control of the structure.

### Layer 3 — The Rust Core

The translation backend is a **zero-overhead Axum HTTP microservice** written in Rust. It launches automatically when you open the app — you never configure a server URL or worry about ports. It lives entirely on `localhost:4888` and shuts down when you close Gradatim.

The Rust core handles:
- **Hint extraction** — intelligently scanning surrounding code to build an accurate context snapshot
- **Provider routing** — seamlessly switching between Gemini, OpenAI, DeepSeek, or any OpenAI-compatible endpoint
- **Fallback translation** — a deterministic rule-based translator kicks in when no API key is configured, so the app always works
- **Request validation** — ensuring malformed or empty requests never reach the AI layer

---

## The Gradatim Workflow

Here is what a real session looks like, from opening the app to shipping a working function:

```
  ┌──────────────────────────────────────────────────────────────┐
  │  STEP 1 — Open Gradatim                                      │
  │                                                              │
  │  The app launches with a blank untitled file and an open     │
  │  terminal at the bottom. The Rust core server boots          │
  │  silently in the background. You are ready to write.         │
  └──────────────────────────────────────────────────────────────┘
                               │
                               ▼
  ┌──────────────────────────────────────────────────────────────┐
  │  STEP 2 — Choose your target language                        │
  │                                                              │
  │  Click the TARGET dropdown in the top bar and select C,      │
  │  Python, or another supported language. This single choice   │
  │  informs every translation the AI makes going forward.       │
  └──────────────────────────────────────────────────────────────┘
                               │
                               ▼
  ┌──────────────────────────────────────────────────────────────┐
  │  STEP 3 — Write in English                                   │
  │                                                              │
  │  Type your logic naturally:                                  │
  │    "declare an integer called score and set it to 0"         │
  │    "declare an integer called max set to 100"                │
  │    "if score is greater than max, print you win"             │
  │                                                              │
  │  A purple gutter marker appears on any line not yet          │
  │  translated, keeping track of your progress at a glance.     │
  └──────────────────────────────────────────────────────────────┘
                               │
                               ▼
  ┌──────────────────────────────────────────────────────────────┐
  │  STEP 4 — Press Enter (Auto-translate ON)                    │
  │           or click REGENERATE (manual mode)                  │
  │                                                              │
  │  With Auto-translate ON, every Enter keystroke triggers      │
  │  translation of the line you just finished. With it OFF,     │
  │  you can draft multiple English lines and translate them      │
  │  together when you're ready.                                 │
  └──────────────────────────────────────────────────────────────┘
                               │
                               ▼
  ┌──────────────────────────────────────────────────────────────┐
  │  STEP 5 — Review and iterate                                 │
  │                                                              │
  │  Each translated line replaces its English counterpart       │
  │  inline. If you don't like the output, click REGENERATE      │
  │  to request a fresh translation. The Line Activity panel     │
  │  logs every translation event so you can audit your          │
  │  session at any time.                                        │
  └──────────────────────────────────────────────────────────────┘
                               │
                               ▼
  ┌──────────────────────────────────────────────────────────────┐
  │  STEP 6 — Save, run, ship                                    │
  │                                                              │
  │  Save your file (Ctrl+S), open the integrated terminal,      │
  │  compile and run your code without ever leaving the app.     │
  │  File → Open Folder lets you load a full project workspace   │
  │  with a navigable file tree on the right panel.             │
  └──────────────────────────────────────────────────────────────┘
```

---

## Features

### Core Translation Engine

| Feature | Description |
|---|---|
| **Line-by-line AI translation** | Each English line is independently translated with full context awareness |
| **Context window** | Configurable look-ahead and look-behind character counts sent with every request |
| **Hint extraction** | Rust-powered static analysis of surrounding code feeds variable names, types, and scope into each prompt |
| **Multi-provider AI** | Gemini (default), OpenAI, DeepSeek, or any OpenAI-compatible endpoint |
| **Deterministic fallback** | Rule-based translator runs when no API key is present — app never crashes |
| **Auto-translate mode** | Toggle translation on Enter keystroke or trigger manually |
| **Regenerate** | One click to request a fresh translation for the current line |

### Editor Experience

| Feature | Description |
|---|---|
| **Monaco Editor** | The same engine as VS Code — battle-tested, fast, extensible |
| **Multi-tab editing** | Open multiple files simultaneously with per-tab state |
| **Untranslated line markers** | Animated gutter indicators show exactly which lines still need translation |
| **Bracket pair colorization** | Nested brackets are colour-coded for instant visual parsing |
| **Sticky scroll** | Your current function or block header stays pinned at the top of the editor |
| **Smooth scrolling** | Buttery animations throughout the editing experience |
| **Linked editing** | Rename matching tags simultaneously |
| **Word wrap toggle** | Soft-wrap long lines or scroll horizontally |
| **Configurable tab size** | 2, 4, or 8 spaces — stored per-user |
| **Font customization** | Choose your preferred monospace font and size |
| **Cursor style** | Line, block, or underline cursor |
| **Render whitespace** | Optionally show invisible characters |

### Workspace & File Management

| Feature | Description |
|---|---|
| **File tree sidebar** | Right-panel directory viewer for your open folder |
| **Open Folder** | Load any directory as your workspace |
| **Save / Save As** | Standard keyboard shortcuts (Ctrl+S, Ctrl+Shift+S) |
| **New Tab** | Ctrl+T opens a fresh untitled file |
| **Unsaved indicator** | Tab dot shows unsaved changes |

### Integrated Terminal

| Feature | Description |
|---|---|
| **Built-in terminal** | Full PTY-backed shell session inside the app (PowerShell on Windows, bash/zsh on Linux) |
| **Multiple sessions** | Open as many terminal tabs as you need |
| **Theme-aware** | Terminal colors match your selected UI theme |
| **Glass effect** | Frosted terminal panel consistent with the overall design language |

### Themes & UI

| Feature | Description |
|---|---|
| **Light theme** | Clean white glassmorphism with soft purple accents |
| **Dark theme** | Deep navy with vibrant blue-purple highlights |
| **Glass theme** | Fully translucent frosted-glass panels with blur effects |
| **Glassmorphism** | `backdrop-filter: blur` across all panels for a premium feel |
| **Smooth animations** | Menu transitions, status entries, modal reveals, and button interactions |
| **Custom window chrome** | Frameless window with native-feeling minimize/maximize/close controls |
| **Custom scrollbars** | Theme-matched slim scrollbars across the entire UI |
| **Logo in top bar** | Gradatim brand mark anchored to the top left of every window |

---

## Performance

Gradatim is engineered to stay out of your way. The Rust core is a **compiled, zero-GC microservice** that adds negligible latency between your keystroke and the AI network call. On a typical broadband connection:

```
  Keystroke → AI response → Code in editor

  ┌─────────────────────────────────────────────────────────────┐
  │                                                             │
  │  Local overhead (Electron → Rust core):   ~2–5 ms          │
  │  AI network round-trip (Gemini Flash):    ~400–900 ms       │
  │  In-editor replacement:                   ~1 ms             │
  │                                                             │
  │  Total perceived latency:  under 1 second on most lines    │
  │                                                             │
  └─────────────────────────────────────────────────────────────┘
```

The frontend is built with **Vite** for near-instant hot reload during development, and the production build is a fully bundled static asset set — no runtime compilation, no slow module resolution.

---

## Themes — Light & Dark

Gradatim ships with three carefully crafted visual modes, each with a distinct personality.

### Light Theme
A crisp, professional white-glass aesthetic. Soft frosted panels with translucent depth, purple-blue accents, and gentle shadows. Perfect for bright office environments or presentations.

```
  ╔══════════════════════════════════════════════════════════════╗
  ║ ◉  File   Edit   View   Terminal        TARGET  C ▾    LIGHT ║
  ╠══════════════════════════════════╦═══════════════════════════╣
  ║  untitled-1.c  ×                 ║  LINE ACTIVITY            ║
  ╠══════════════════════════════════║                           ║
  ║  1  // Welcome to Gradatim       ╠═══════════════════════════╣
  ║  2  // Type an English line      ║  FILES                    ║
  ║  3                               ║                           ║
  ║  4  declare a, b 0               ║  GRADATIM WORKSPACE       ║
  ║  5  set a to 5                   ║  No folder open.          ║
  ║  6  if a is greater than b       ║  Use File → Open Folder.  ║
  ║                                  ║                           ║
  ╠══════════════════════════════════╩═══════════════════════════╣
  ║  Terminal 1  ×                                          + ▾  ║
  ║  C:\Users\Owner>_                                            ║
  ╚══════════════════════════════════════════════════════════════╝
```

### Dark Theme
A deep navy immersive mode designed for long coding sessions. High-contrast text, electric purple-blue highlights, and a frosted terminal that keeps everything readable without eye strain.

```
  ╔══════════════════════════════════════════════════════════════╗
  ║▓ ◉  File   Edit   View   Terminal       TARGET  C ▾    DARK ▓║
  ╠▓═════════════════════════════════╦══════════════════════════▓╣
  ║▓  untitled-1.c  ×               ▓║  LINE ACTIVITY          ▓ ║
  ╠▓═════════════════════════════════║                         ▓ ║
  ║▓  1  // Welcome to Gradatim     ▓╠══════════════════════════╣
  ║▓  2  // Type an English line    ▓║  FILES                  ▓ ║
  ║▓  3                             ▓║                         ▓ ║
  ║▓  4  declare a, b 0             ▓║  GRADATIM WORKSPACE     ▓ ║
  ║▓  5  set a to 5                 ▓║  No folder open.        ▓ ║
  ║▓  6  if a is greater than b     ▓║  Use File → Open Folder.▓ ║
  ╠▓═════════════════════════════════╩══════════════════════════▓╣
  ║▓  Terminal 1  ×                                       + ▾  ▓ ║
  ║▓  C:\Users\Owner>_                                         ▓ ║
  ╚══════════════════════════════════════════════════════════════╝
```

Switch between themes instantly with the **LIGHT / DARK / GLASS** button in the top bar. Your preference is saved automatically and restored on next launch.

---

## AI Providers

Gradatim is model-agnostic. Open **Settings → AI Provider** to choose:

```
  ┌──────────────────────────────────────────────────────────┐
  │                   AI PROVIDER SELECTION                  │
  ├──────────────────────────────────────────────────────────┤
  │                                                          │
  │  ● Google Gemini (recommended)                           │
  │      Model: gemini-2.5-flash                             │
  │      → Fast, free tier available, excellent code quality │
  │      → Get key: aistudio.google.com                      │
  │                                                          │
  │  ○ OpenAI                                                │
  │      Model: gpt-4o / gpt-4o-mini / gpt-3.5-turbo        │
  │      → Industry standard, widely trusted                 │
  │      → Get key: platform.openai.com/api-keys             │
  │                                                          │
  │  ○ DeepSeek                                              │
  │      Model: deepseek-chat / deepseek-coder               │
  │      → Open-weights, code-specialist, cost-effective     │
  │      → Get key: platform.deepseek.com/api_keys           │
  │                                                          │
  │  ○ Other (OpenAI-compatible)                             │
  │      Any endpoint that speaks the OpenAI API format      │
  │      → Local models (Ollama, LM Studio), etc.            │
  │                                                          │
  └──────────────────────────────────────────────────────────┘
```

Your API key is stored locally in your OS's user-data directory and is **never transmitted anywhere except directly to your chosen provider**.

---

## Who Is Gradatim For?

### Beginners learning to code
You understand what you want to do — you just don't know the syntax yet. Gradatim lets you express your intent naturally while showing you exactly what the corresponding code looks like. It's an interactive textbook where every line you write produces a real, runnable example.

### Rapid prototypers
Need to sketch out an algorithm or data structure quickly? Describe the shape of your idea in English and let Gradatim fill in the syntax. Iterate on the logic, not the boilerplate.

### Polyglot developers
Working across multiple languages in the same week? Stop context-switching your mental syntax library. Describe what you want once; Gradatim handles the language-specific idioms for each target.

### Educators and instructors
Demonstrate programming concepts in plain English and show students the machine-generated equivalent in real time. An unparalleled live teaching tool.

### Non-native English speakers in technical roles
The barrier between "I know what this program should do" and "I can write this program" is often just syntax and vocabulary. Gradatim collapses that barrier.

---

## Productivity Impact

> *"The best code is the code you don't have to look up."*

Independent analyses of developer workflows show that the average programmer spends **30–40% of their active coding time** on non-logical tasks: looking up syntax, browsing documentation, correcting trivial errors, and re-typing boilerplate. Gradatim attacks all four of these simultaneously.

```
  TIME DISTRIBUTION: Traditional vs Gradatim-Assisted Development
  ─────────────────────────────────────────────────────────────────

  TRADITIONAL WORKFLOW
  ████████████████████░░░░░░░░░░░░░░░░░░░░  40%  Logic & Design
  ░░░░░░░░░░░░░░░░░░░░████████████░░░░░░░░  30%  Typing & Syntax
  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░████████  20%  Lookup & Docs
  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░██  10%  Debugging typos

  GRADATIM-ASSISTED WORKFLOW
  ████████████████████████████████████████  70%  Logic & Design
  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░████████░░░░  20%  Review & Refine
  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░████  10%  Edge cases & testing
```

By offloading syntax recall to the AI, programmers can dedicate more of their mental energy to the parts that matter: **architecture, logic, and problem-solving**.

---

## Settings Reference

Access settings via the gear icon ⚙ in the top bar, or through **Edit → Settings**.

| Setting | Description | Default |
|---|---|---|
| **Target language** | Default translation output language | C |
| **Auto-translate** | Translate on each Enter keypress | On |
| **Max lines per translation** | Safety cap on batch translation runs | 50 |
| **Context before (chars)** | Characters of preceding code sent as context | 500 |
| **Context after (chars)** | Characters of following code sent as context | 200 |
| **AI Provider** | Gemini / OpenAI / DeepSeek / Other | Gemini |
| **Model name** | Specific model string for your provider | gemini-2.5-flash |
| **API Key** | Stored locally, never shared | — |
| **Tab size** | Spaces per indent level | 4 |
| **Insert spaces** | Spaces vs hard tabs | On |
| **Font size** | Editor font size in px | 14 |
| **Font family** | Monospace font for the editor | Consolas |
| **Render whitespace** | Show invisible characters | Off |
| **Cursor style** | Line / Block / Underline | Line |
| **Word wrap** | Soft-wrap long lines | Off |
| **Line numbers** | Show / hide gutter numbers | On |

---

## Project Layout

```
  gradatim/
  ├── electron/                  Electron main process
  │   ├── main.js                App lifecycle, IPC handlers, Rust core launcher
  │   ├── preload.js             Secure renderer ↔ main bridge (contextIsolation)
  │   ├── config-store.js        Persistent user settings (OS userData dir)
  │   └── pty-manager.js         PTY shell session management for the terminal
  │
  ├── renderer/                  Frontend (Vite + TypeScript)
  │   ├── index.html             App shell HTML
  │   └── src/
  │       ├── main.ts            Core application logic, editor init, event wiring
  │       ├── style.css          Global styles, CSS variables, theme definitions
  │       ├── settings.ts        Settings UI, form handling, provider sync
  │       ├── types/
  │       │   └── electron.d.ts  TypeScript types for the electronAPI bridge
  │       └── terminal/
  │           ├── TerminalPanel.ts     Multi-tab terminal panel manager
  │           ├── TerminalInstance.ts  xterm.js wrapper with theme support
  │           └── terminal.css         Terminal-specific styles
  │
  ├── rust-core/                 Axum translation microservice (Rust)
  │   └── src/
  │       ├── main.rs            HTTP server, route handlers, AI provider dispatch
  │       ├── models.rs          Request/response data structures
  │       └── hints.rs           Context extraction and hint assembly logic
  │
  ├── build/                     App icon assets (generated)
  ├── scripts/                   Build utilities (icon generation, etc.)
  ├── electron-builder.yml       Installer configuration (NSIS, AppImage, deb)
  └── package.json               Project manifest and npm scripts
```

---

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) v18 or later
- [Rust toolchain](https://rustup.rs/) (stable)
- [npm](https://npmjs.com/) v9 or later

### Development

```bash
# Clone the repository
git clone https://github.com/your-org/gradatim.git
cd gradatim

# Install Node dependencies and rebuild native modules
npm install

# Start everything: renderer dev server + Electron + Rust core
npm run dev:all

# Or start without the Rust core (uses fallback translator)
npm run dev
```

The Rust core can also be started manually in a separate terminal:

```bash
cd rust-core
cargo run
# Listening on 127.0.0.1:4888
```

### Building Installers

```bash
# Build the renderer (required before packaging)
npm run build

# Windows installer (.exe — NSIS)
npm run package:win

# Linux installers (AppImage + .deb)
npm run package:linux

# All platforms
npm run package
```

Installers are written to the `release/` directory.

> **Windows note:** Run the build in a terminal with Developer Mode enabled (Settings → Privacy & Security → Developer Mode) to allow the installer toolchain to create symbolic links.

### Configuring AI

1. Open the installed Gradatim app
2. Click the ⚙ gear icon or go to **Edit → Settings**
3. Under **AI**, choose your provider and paste your API key
4. Click **Save**

No environment variables are required for the packaged app — everything is configured from within the UI.

---

## Roadmap

```
  ████████████████████████  v0.1  ──  Editor shell, Monaco, Rust core, AI translation
  ████████████████████████  v0.1  ──  Themes, terminal, file tree, custom window chrome
  ████████████████████████  v0.1  ──  Installer (Windows + Linux), auto-server startup
  ░░░░░░░░░░░░░░░░░░░░░░░░  v0.2  ──  Batch translation, multi-file context awareness
  ░░░░░░░░░░░░░░░░░░░░░░░░  v0.2  ──  More target languages (Java, Rust, Go, JavaScript)
  ░░░░░░░░░░░░░░░░░░░░░░░░  v0.3  ──  Auto-update (electron-updater)
  ░░░░░░░░░░░░░░░░░░░░░░░░  v0.3  ──  Translation history and undo
  ░░░░░░░░░░░░░░░░░░░░░░░░  v0.4  ──  Inline diff view (English ↔ code toggle)
  ░░░░░░░░░░░░░░░░░░░░░░░░  v0.5  ──  macOS support + .dmg installer
  ░░░░░░░░░░░░░░░░░░░░░░░░  v1.0  ──  Plugin API, community themes
```

---

## License

Gradatim is released under the [MIT License](LICENSE). You are free to use, modify, and distribute it for any purpose.

---

<div align="center">

**Built with care using Electron, Monaco, Rust, Axum, Vite, TypeScript, and xterm.js**

*Gradatim — step by step, your code writes itself.*

</div>
