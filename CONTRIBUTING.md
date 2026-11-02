# CONTRIBUTING

## No GDScript

The engine project has no GDScript: gameplay, UI nodes and tools are Rust in `lib/crates/dcl_engine`. CI fails on any `.gd` under `godot/src`.

## Format Rust

Format rust
```bash
cd lib
cargo fmt --all
cargo clippy -- -D warnings
```

## Git Hooks

You can add the following hooks at `.git/hooks/pre-commit`

! Remember to add executable permissions

```bash
chmod +x .git/hooks/pre-commit
```

Script:
```bash
#!/bin/bash

## FORMAT RUST

# Change to the specific Rust directory
cd lib

# Check if cargo fmt would make changes
if ! cargo fmt -- --check
then
  echo "Code formatting in 'lib' differs from cargo fmt's style"
  echo "Run 'cargo fmt --all' inside 'lib' to format the code."
  exit 1
fi

echo "Code formatted"

# If everything is okay, proceed with the commit
exit 0
```