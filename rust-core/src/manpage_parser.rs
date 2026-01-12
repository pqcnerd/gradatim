//! Optional man page parser (stubbed for now).
//! On non-Unix platforms or when `man` is unavailable, this returns None.

use std::process::Command;

use crate::stdlib_db::FunctionSpec;

pub fn parse_manpage(_func_name: &str) -> Option<FunctionSpec> {
    // Stub: only attempt on Unix with `man` available.
    if !cfg!(unix) {
        return None;
    }

    // Basic availability check
    let Ok(status) = Command::new("man").arg("--version").status() else {
        return None;
    };
    if !status.success() {
        return None;
    }

    // TODO: Implement real parsing of `man 3 func_name`
    None
}
