


# Build
## static binary
### use cross
```bash
cross build --release --target x86_64-unknown-linux-musl
```
### use musl-gcc
```bash
sudo apt install musl-tools -y
cargo build --release --target x86_64-unknown-linux-musl
```

## even faster
```bash
MALLOC_CONF="thp:always,metadata_thp:always" cargo build --release
```
this will make jemalloc to be configured to use transparent huge pages (THP). This can further speed up programs, possibly at the cost of higher memory usage.
But The system running the compiled program also has to be configured to support THP. See this blog post for more details.