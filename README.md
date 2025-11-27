# Gradatim Editor (Prototype)

This repository hosts the first prototype of the Gradatim editor: an Electron application that converts natural-language lines into C code, one line at a time. The current milestone focuses on getting the editor shell up and running with Monaco and the right hooks in place to trigger translations.

## Project layout

- `electron/` &mdash; Electron main and preload scripts. The preload file currently exposes a stubbed `translateLine` bridge.
- `renderer/` &mdash; Vite + TypeScript front-end with a Monaco editor and UI chrome for line activity.
- `rust-core/` &mdash; Axum-based microservice that exposes the `/translate-line` HTTP endpoint with simple rule-based translations.

## Getting started

```bash
# Install dependencies
npm install

# (Optional) configure AI credentials
setx GEMINI_API_KEY "AIzaSyCj43mdreAx0F_DIhVMMHLK8W"
setx GEMINI_MODEL "gemini-2.5-flash"

# Run the Rust translator (in a separate shell)
npm run dev:rust

# Run the renderer dev server + Electron simultaneously
npm run dev

# Build static assets for production
npm run build

# Bundle the Electron app (outputs to /release)
npm run package
```

> **Note:** The editor now calls the `rust-core` microservice for every translated line. If no AI credentials are available, the backend falls back to the rule-based translator so you still get deterministic snippets.

## Next steps

1. Connect the renderer to the Rust backend via Electron's IPC/bridge.
2. Integrate a fast LLM (e.g., Gemini Flash) to perform constrained line translations.
3. Polish the UX (status indicators, regenerate controls) and add settings for API configuration.

## Rust core service

The translation microservice lives in `rust-core/`. Run it independently with:

```bash
cd rust-core
cargo run
```

By default it listens on `127.0.0.1:4888`. Set `RUST_CORE_ADDR` to override.

## Settings and API keys

- Open **Settings** in the editor header to configure:
  - Rust core endpoint URL
  - Auto-translate preference
  - Default target language (C or Python)
  - Context window sizes (characters sent before/after the current line)
  - Gemini model + API key
- Settings are saved to the OS-specific Electron user-data directory (`gradatim-settings.json`).
- The Gemini API key is forwarded to `rust-core` on each translation request. If absent, the service falls back to the `GEMINI_API_KEY` environment variable.

## Target languages

- Use the header dropdown to switch between **C** and **Python** on the fly.
- The renderer passes the chosen language to `rust-core`, which adjusts both the rule-based translator and the AI prompt.
- The selected language persists via the settings modal and is stored in `gradatim-settings.json`.

