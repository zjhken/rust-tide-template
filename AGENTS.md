# Global Rust Conventions

## anyhow_ext
- If using `anyhow_ext`, all `.?` must use `.dot()?` instead.

## Development Workflow
- After finishing implementing a part of the code, run `cargo check`.
- Write unit tests for the implemented code.
- Once all code is ready, run `cargo fmt`.

## Code Style
- All Rust projects use **tab** as indentation.
