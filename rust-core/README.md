# rust-core

`rust-core` is a lightweight Axum-based microservice that exposes the `/translate-line` endpoint consumed by the Gradatim Electron client.

## Running locally

```bash
cargo run
```

By default the server binds to `127.0.0.1:4888`. Override via `RUST_CORE_ADDR`.

## API

- `POST /translate-line`  
  Request:
  ```json
  {
    "english_line": "declare a, b 0",
    "line_index": 3,
    "code_before": "",
    "code_after": "",
    "language": "c"
  }
  ```
  Response:
  ```json
  {
    "kind": "ok",
    "code": "int a = 0, b = 0;",
    "message": null
  }
  ```

The current translator relies on simple heuristics for declarations, assignments, loops, and conditionals. It acts as a placeholder until an AI-backed translator is wired in.

