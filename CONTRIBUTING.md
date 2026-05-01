# CONTRIBUTING

## Format GDScript

When GD files are modified, they must be well-formated.
It requires [godot-gdscript-toolkit](https://github.com/dcl-regenesislabs/godot-gdscript-toolkit) installed

Installation:
```bash
pip3 uninstall "gdtoolkit==4.*"
pip3 install git+https://github.com/dcl-regenesislabs/godot-gdscript-toolkit.git
```

You can autoformat all files running:
```bash
gdformat godot/
```

You can run the linter with:
```bash
gdlint godot/
```

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

## FORMAT GDSCRIPT

# Get modified .gd files
MODIFIED_GD_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep '\.gd$')

# Check if there are .gd files to lint
if [ ! -z "$MODIFIED_GD_FILES" ]; then

  # Run gdlint on modified files
  echo "Running gdlint on modified files..."
  gdlint $MODIFIED_GD_FILES

  # Save the exit status of gdlint
  GDLINT_EXIT=$?

  # If gdlint finds issues, cancel the commit
  if [ $GDLINT_EXIT -ne 0 ]; then
    echo "gdlint found issues, please fix them before committing."
    exit 1
  fi

  # Run gdformat on modified files
  echo "Running gdformat on modified files..."
  gdformat -d $MODIFIED_GD_FILES

  # Save the exit status of gdlint
  GDFORMAT_EXIT=$?

  # If gdlint finds issues, cancel the commit
  if [ $GDFORMAT_EXIT -ne 0 ]; then
    echo "gdformat found issues, please fix them before committing."
    exit 1
  fi
fi

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