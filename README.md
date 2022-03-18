# Build source

```
cargo build --release
```

# Run node as development mode

- Only Alith is validator (produce and finalize block)
- Expose RPC port to all

```
# If you have already build source
./target/release/substrate --dev

# Or
cargo run --release -- --dev
```