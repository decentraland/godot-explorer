# Rust Error Parser CLI

Tool to parse and query `cargo build` errors for fast iteration.

## Quick Start

```bash
./parse_errors.py build    # Build + parse + update JSONs
./parse_errors.py summary  # Quick overview
./parse_errors.py fix-plan # Prioritized fix plan
```

## Commands

| Command | Description |
|---------|-------------|
| `build` | Run cargo build in lib/, save to errors.txt, parse |
| `parse` | Re-parse existing errors.txt |
| `summary` | Stats + top 5 files + codes |
| `fix-plan` | Prioritized plan for fixing |
| `top-files [N] [-e]` | Top N files by error count |
| `top-errors [N] [-e]` | Top N error codes |
| `file <path> [-e] [-d]` | Errors in file (partial match) |
| `code <code> [-d] [-l N]` | Errors by code (e.g., E0283) |
| `detail <index>` | Full details for error at index |
| `next` | Next error to fix (from top file) |

## Flags

- `-e, --only-errors`: Exclude warnings
- `-d, --detailed`: Show full error details
- `-l, --limit N`: Limit results (default: 20)

## Workflow

1. `./parse_errors.py build` - build and parse
2. `./parse_errors.py fix-plan` - see priorities
3. `./parse_errors.py file <file> -e -d` - get file details
4. Fix errors, repeat

## Output Files

- `errors.txt` - Raw cargo build output
- `errors_simple.json` - type, code, description, file, line
- `errors_detailed.json` - Above + notes, helps, code_context
- `errors_summary.json` - Aggregated stats by file and code
