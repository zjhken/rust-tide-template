# Tuning notes

## Static binary (musl)

### With `cross`

```bash
cross build --release --target x86_64-unknown-linux-musl
```

### With system musl-gcc

```bash
sudo apt install musl-tools -y
cargo build --release --target x86_64-unknown-linux-musl
```

## Allocator

This template uses [`mimalloc`](https://github.com/microsoft/mimalloc) as the global allocator (see `src/main.rs`). An earlier version used `jemalloc`; if you switch back to jemalloc, you can enable transparent huge pages to squeeze out a bit more throughput at the cost of higher memory:

```bash
MALLOC_CONF="thp:always,metadata_thp:always" cargo build --release
```

The system running the compiled binary also needs THP support enabled at the kernel level.
