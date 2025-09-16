# Publishing to crates.io

This document outlines the steps to publish `tagged_dispatch` to crates.io.

## Prerequisites

1. **Create a crates.io account** if you don't have one: https://crates.io
2. **Get an API token**: Go to Account Settings → API Tokens → New Token
3. **Login locally**: `cargo login YOUR_API_TOKEN`

## Publishing Steps

### Step 1: Publish the proc macro crate first

The main crate depends on `tagged_dispatch_macros`, so we need to publish it first:

```bash
cd tagged_dispatch_macros
cargo publish
```

Wait a few minutes for it to be indexed on crates.io.

### Step 2: Update main crate dependency

After the macro crate is published, the main `Cargo.toml` already has the correct dependency format:
```toml
tagged_dispatch_macros = { version = "0.1.0", path = "tagged_dispatch_macros" }
```

The `path` is optional and will be ignored when publishing.

### Step 3: Final checks

Run these commands from the root directory:

```bash
# Run all tests
cargo test --all-features

# Check clippy
cargo clippy --all-features -- -D warnings

# Build documentation
cargo doc --no-deps --all-features

# Verify what will be packaged
cargo package --list --allow-dirty

# Do a dry run
cargo publish --dry-run --allow-dirty
```

### Step 4: Create git tag

```bash
git add .
git commit -m "Prepare for v0.1.0 release"
git tag v0.1.0
git push origin main --tags
```

### Step 5: Publish the main crate

```bash
cargo publish
```

## Known Issues

⚠️ **Arena tests segfault**: The `arena_builder.rs` test has a segmentation fault with the bumpalo feature. This should be fixed before a production release, but the core functionality works fine.

## Post-Publishing

After publishing:

1. Verify on crates.io: https://crates.io/crates/tagged_dispatch
2. Check docs.rs: https://docs.rs/tagged_dispatch
3. Create a GitHub release with release notes
4. Consider announcing on:
   - Reddit r/rust
   - Twitter/X with #rustlang
   - This Week in Rust

## Version Bumping

For future releases:

1. Update version in both `Cargo.toml` files
2. Update the README if needed
3. Add entry to CHANGELOG.md (create if doesn't exist)
4. Follow the publishing steps again

## Troubleshooting

If you get "no matching package" error:
- Make sure `tagged_dispatch_macros` is published first
- Wait 1-2 minutes for crates.io indexing
- Try again

If you get "already published" error:
- Bump the version number
- Make sure you're publishing a new version