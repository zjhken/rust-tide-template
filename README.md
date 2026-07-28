# rust-tide-template

Personal starter template for Rust HTTP services. Built on Tide 0.16 + SeaORM + async-std, edition 2024.

> **Note**: Tide has been archived (`http-rs/tide` is read-only). This template will be replaced once the author's own smol-based web framework is ready. New code tightly coupled to tide's specific API (auth middleware wiring, route groups, error handler middleware) is intentionally left minimal — see [Known tech debt](#known-tech-debt).

## Quick start

```bash
cp config.example.toml config.toml   # optional, defaults work out of the box
cargo run
```

Server listens on `0.0.0.0:8888` by default. Hit `http://localhost:8888/`.

## Configuration

Three sources merged with priority **CLI > env > file > default**:

| Source | Example |
|---|---|
| CLI flag | `cargo run -- --bind 127.0.0.1:9000` |
| Env var | `APP_BIND=127.0.0.1:9000 cargo run` |
| Config file | `bind = "127.0.0.1:9000"` in `config.toml` (path via `-c <FILE>`) |
| Default | `0.0.0.0:8888` |

Per [12-factor](https://12factor.net/config), **env beats file**: ops can override file values via env without editing the file. CLI beats both.

Supported keys: `bind`, `log_directive`, `db_url` (optional).

## Layout

```
src/
├── main.rs            # entry, mimalloc global allocator, bootstrap
├── cli.rs             # clap CLI + Env enum (local/uat/prd, reserved)
├── config.rs          # layered config (CLI > env > file > default)
├── database.rs        # SeaORM connection + run migrations
├── logger.rs          # tracing-subscriber, pipe formatter, runtime log-level reload
├── server.rs          # tide app + middleware stack
├── auth.rs            # HTTP Basic Auth (stub authn pending framework rewrite)
├── utils.rs           # task-local req_id, random string gen
├── entity/todo.rs     # SeaORM entity example
└── repository/
    └── todo_repo.rs   # repository pattern example (CRUD over `todo`)
```

## Known tech debt

Skipped on purpose because the tide-specific code will be rewritten on top of the new framework:

- `AuthMiddleware` shadow bug — `server.rs` registers a no-op that masks the real `auth::AuthMiddleware`
- `authn()` returns `Ok(true)` for everyone (stub)
- No `/health` or `/ready` endpoint
- No example CRUD handler consuming `todo_repo` (the repository exists but no route calls it yet)
- `ErrorHandleMiddleware` leaks `{err:?}` to the client — intentional for debug friendliness in this template; revisit per framework
- `panic = abort` in release profile without an external supervisor
- `update_global_log_level` rejects `trace` directive (async-std panics); will be re-evaluated on smol migration

## Tuning

Static musl builds and allocator tuning: see [docs/tuning.md](docs/tuning.md).

## License

Dual-licensed under MIT or Apache-2.0, at your option.
