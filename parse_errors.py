#!/usr/bin/env python3
"""
Rust Error Parser CLI - Parse and query cargo build errors for fast LLM iteration.

Usage:
    ./parse_errors.py build              # Run cargo build, parse errors, update JSONs
    ./parse_errors.py parse              # Parse existing errors.txt
    ./parse_errors.py summary            # Show summary stats
    ./parse_errors.py top-files [N]      # Show top N files with most errors
    ./parse_errors.py top-errors [N]     # Show top N most common error codes
    ./parse_errors.py file <path>        # Show all errors in a specific file
    ./parse_errors.py code <code>        # Show all errors with a specific code (e.g., E0283)
    ./parse_errors.py detail <index>     # Show detailed view of error at index
    ./parse_errors.py next               # Show next error to fix (from top file, top code)
"""

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
LIB_DIR = SCRIPT_DIR / "lib"
ERRORS_FILE = SCRIPT_DIR / "errors.txt"
SIMPLE_JSON = SCRIPT_DIR / "errors_simple.json"
DETAILED_JSON = SCRIPT_DIR / "errors_detailed.json"
SUMMARY_JSON = SCRIPT_DIR / "errors_summary.json"


def run_cargo_build():
    """Run cargo build in lib/ and capture stderr to errors.txt."""
    print("Running cargo build in lib/...")

    env = os.environ.copy()
    env["CARGO_TERM_COLOR"] = "never"  # Disable colors for parsing

    result = subprocess.run(
        ["cargo", "build"],
        cwd=LIB_DIR,
        capture_output=True,
        text=True,
        env=env
    )

    # Cargo outputs errors to stderr
    output = result.stderr

    with open(ERRORS_FILE, "w") as f:
        f.write(output)

    print(f"Build output saved to {ERRORS_FILE}")
    return output


def parse_errors_file(content=None):
    """Parse errors.txt and return simple and detailed entries."""
    if content is None:
        if not ERRORS_FILE.exists():
            print(f"Error: {ERRORS_FILE} not found. Run 'build' first.")
            sys.exit(1)
        with open(ERRORS_FILE, 'r') as f:
            content = f.read()

    # Split by error/warning boundaries
    pattern = r'(?=(?:^|\n)\s*(?:warning|error)(?:\[[^\]]+\])?:)'
    blocks = re.split(pattern, content)

    simple_entries = []
    detailed_entries = []

    for block in blocks:
        block = block.strip()
        if not block:
            continue

        # Extract type (error/warning) and code
        header_match = re.match(r'(warning|error)(?:\[([^\]]+)\])?:\s*(.+?)(?:\n|$)', block)
        if not header_match:
            continue

        entry_type = header_match.group(1)
        code = header_match.group(2) or ""
        description = header_match.group(3).strip()

        # Extract file location: --> file:line:column
        location_match = re.search(r'-->\s*([^:]+):(\d+):(\d+)', block)
        file_path = ""
        line_num = 0
        column = 0
        if location_match:
            file_path = location_match.group(1).strip()
            line_num = int(location_match.group(2))
            column = int(location_match.group(3))

        # Extract notes
        notes = re.findall(r'=\s*note:\s*(.+?)(?:\n|$)', block)

        # Extract help suggestions
        helps = re.findall(r'=\s*help:\s*(.+?)(?:\n|$)', block)

        # Extract code context
        code_lines = []
        for line in block.split('\n'):
            code_match = re.match(r'\s*\d+\s*\|\s*(.+)', line)
            if code_match:
                code_lines.append(code_match.group(1))
            elif re.match(r'\s*\|\s*\S', line):
                code_lines.append(line.strip().lstrip('|').strip())

        simple_entry = {
            "index": len(simple_entries),
            "type": entry_type,
            "code": code,
            "description": description,
            "file": file_path,
            "line": line_num,
            "column": column
        }

        detailed_entry = {
            **simple_entry,
            "notes": notes,
            "helps": helps,
            "code_context": code_lines,
            "raw_block": block
        }

        simple_entries.append(simple_entry)
        detailed_entries.append(detailed_entry)

    return simple_entries, detailed_entries


def build_summary(entries):
    """Build summary with files and error codes sorted by count."""
    files_count = {}
    for e in entries:
        f = e["file"]
        if f:
            if f not in files_count:
                files_count[f] = {"total": 0, "errors": 0, "warnings": 0}
            files_count[f]["total"] += 1
            files_count[f][e["type"] + "s"] += 1

    codes_count = {}
    for e in entries:
        key = f"{e['type']}[{e['code']}]" if e['code'] else e['type']
        if key not in codes_count:
            codes_count[key] = {"count": 0, "files": set()}
        codes_count[key]["count"] += 1
        if e["file"]:
            codes_count[key]["files"].add(e["file"])

    files_sorted = [
        {"file": f, **counts}
        for f, counts in sorted(files_count.items(), key=lambda x: -x[1]["total"])
    ]

    codes_sorted = [
        {"code": code, "count": data["count"], "affected_files": sorted(data["files"])}
        for code, data in sorted(codes_count.items(), key=lambda x: -x[1]["count"])
    ]

    return {
        "total_entries": len(entries),
        "total_errors": sum(1 for e in entries if e["type"] == "error"),
        "total_warnings": sum(1 for e in entries if e["type"] == "warning"),
        "files_by_count": files_sorted,
        "codes_by_count": codes_sorted
    }


def save_json_files(simple, detailed, summary):
    """Save all JSON files."""
    with open(SIMPLE_JSON, "w") as f:
        json.dump(simple, f, indent=2)
    with open(DETAILED_JSON, "w") as f:
        json.dump(detailed, f, indent=2)
    with open(SUMMARY_JSON, "w") as f:
        json.dump(summary, f, indent=2)


def load_data():
    """Load parsed data from JSON files."""
    if not SIMPLE_JSON.exists():
        print("No parsed data found. Run 'build' or 'parse' first.")
        sys.exit(1)

    with open(SIMPLE_JSON) as f:
        simple = json.load(f)
    with open(DETAILED_JSON) as f:
        detailed = json.load(f)
    with open(SUMMARY_JSON) as f:
        summary = json.load(f)

    return simple, detailed, summary


def format_error_simple(e):
    """Format a single error in simple one-line format."""
    code_str = f"[{e['code']}]" if e['code'] else ""
    return f"#{e['index']:3d} {e['type']}{code_str}: {e['file']}:{e['line']} - {e['description'][:80]}"


def format_error_detailed(e):
    """Format a single error with full details."""
    lines = [
        f"{'='*60}",
        f"Index: {e['index']}",
        f"Type: {e['type'].upper()}",
        f"Code: {e['code'] or 'N/A'}",
        f"File: {e['file']}",
        f"Line: {e['line']}, Column: {e['column']}",
        f"",
        f"Description:",
        f"  {e['description']}",
    ]

    if e.get('code_context'):
        lines.append("")
        lines.append("Code context:")
        for ctx in e['code_context']:
            lines.append(f"  {ctx}")

    if e.get('notes'):
        lines.append("")
        lines.append("Notes:")
        for note in e['notes']:
            lines.append(f"  - {note}")

    if e.get('helps'):
        lines.append("")
        lines.append("Help:")
        for h in e['helps']:
            lines.append(f"  - {h}")

    lines.append(f"{'='*60}")
    return "\n".join(lines)


# CLI Commands

def cmd_build(args):
    """Run cargo build and parse output."""
    run_cargo_build()
    cmd_parse(args)


def cmd_parse(args):
    """Parse errors.txt and update JSON files."""
    simple, detailed = parse_errors_file()
    summary = build_summary(simple)
    save_json_files(simple, detailed, summary)

    print(f"\nParsed {summary['total_entries']} entries:")
    print(f"  - {summary['total_errors']} errors")
    print(f"  - {summary['total_warnings']} warnings")
    print(f"\nFiles updated: errors_simple.json, errors_detailed.json, errors_summary.json")


def cmd_summary(args):
    """Show summary statistics."""
    _, _, summary = load_data()

    print(f"Total: {summary['total_entries']} ({summary['total_errors']} errors, {summary['total_warnings']} warnings)")
    print(f"\nTop 5 files:")
    for f in summary['files_by_count'][:5]:
        print(f"  {f['total']:3d} | {f['file']}")
    print(f"\nError codes:")
    for c in summary['codes_by_count'][:5]:
        print(f"  {c['count']:3d} | {c['code']}")


def cmd_top_files(args):
    """Show top N files with most errors."""
    n = args.count or 10
    _, _, summary = load_data()

    files = summary['files_by_count']
    if args.only_errors:
        files = [f for f in files if f['errors'] > 0]
        files = sorted(files, key=lambda x: -x['errors'])

    print(f"Top {n} files by error count:\n")
    print(f"{'Total':>5} {'Err':>4} {'Warn':>4}  File")
    print("-" * 60)
    for f in files[:n]:
        print(f"{f['total']:5d} {f['errors']:4d} {f['warnings']:4d}  {f['file']}")


def cmd_top_errors(args):
    """Show top N most common error codes."""
    n = args.count or 10
    _, _, summary = load_data()

    codes = summary['codes_by_count']
    if args.only_errors:
        codes = [c for c in codes if 'error' in c['code']]

    print(f"Top {n} error codes:\n")
    print(f"{'Count':>5} {'Files':>5}  Code")
    print("-" * 40)
    for c in codes[:n]:
        print(f"{c['count']:5d} {len(c['affected_files']):5d}  {c['code']}")


def cmd_file(args):
    """Show all errors in a specific file."""
    simple, detailed, _ = load_data()

    # Partial match on file path
    matches = [e for e in detailed if args.path in e['file']]

    if args.only_errors:
        matches = [e for e in matches if e['type'] == 'error']

    if not matches:
        print(f"No errors found in files matching '{args.path}'")
        return

    print(f"Found {len(matches)} errors in files matching '{args.path}':\n")

    if args.detailed:
        for e in matches:
            print(format_error_detailed(e))
            print()
    else:
        for e in matches:
            print(format_error_simple(e))


def cmd_code(args):
    """Show all errors with a specific error code."""
    simple, detailed, _ = load_data()

    # Match code (e.g., "E0283" or "error[E0283]")
    code_pattern = args.code.upper().replace("ERROR[", "").replace("]", "").replace("WARNING[", "")
    matches = [e for e in detailed if code_pattern in (e['code'] or "").upper()]

    if not matches:
        print(f"No errors found with code '{args.code}'")
        return

    print(f"Found {len(matches)} errors with code '{code_pattern}':\n")

    if args.detailed:
        for e in matches[:args.limit]:
            print(format_error_detailed(e))
            print()
    else:
        for e in matches[:args.limit]:
            print(format_error_simple(e))

    if len(matches) > args.limit:
        print(f"\n... and {len(matches) - args.limit} more. Use --limit to see more.")


def cmd_detail(args):
    """Show detailed view of a specific error by index."""
    _, detailed, _ = load_data()

    if args.index < 0 or args.index >= len(detailed):
        print(f"Invalid index. Valid range: 0-{len(detailed)-1}")
        return

    print(format_error_detailed(detailed[args.index]))


def cmd_next(args):
    """Show the next error to fix (from file with most errors, most common code)."""
    simple, detailed, summary = load_data()

    if not summary['files_by_count']:
        print("No errors found!")
        return

    # Get first error from file with most errors
    top_file = summary['files_by_count'][0]['file']
    file_errors = [e for e in detailed if e['file'] == top_file and e['type'] == 'error']

    if not file_errors:
        file_errors = [e for e in detailed if e['file'] == top_file]

    if file_errors:
        print(f"Next error to fix (from top file: {top_file}):\n")
        print(format_error_detailed(file_errors[0]))
    else:
        print("No actionable errors found.")


def cmd_fix_plan(args):
    """Generate a fix plan for LLM consumption."""
    _, detailed, summary = load_data()

    if not summary['codes_by_count']:
        print("No errors to fix!")
        return

    print("# Error Fix Plan\n")
    print(f"Total: {summary['total_errors']} errors, {summary['total_warnings']} warnings\n")

    print("## Priority by Error Code (fix one type across all files):\n")
    for i, c in enumerate(summary['codes_by_count'][:5], 1):
        if 'error' in c['code']:
            print(f"{i}. **{c['code']}** - {c['count']} occurrences in {len(c['affected_files'])} files")
            # Show example
            example = next((e for e in detailed if e['code'] and c['code'].endswith(f"[{e['code']}]")), None)
            if example:
                print(f"   Example: {example['description'][:70]}...")
            print()

    print("\n## Priority by File (fix all errors in one file):\n")
    for i, f in enumerate(summary['files_by_count'][:5], 1):
        print(f"{i}. **{f['file']}** - {f['errors']} errors, {f['warnings']} warnings")


def main():
    parser = argparse.ArgumentParser(
        description="Rust Error Parser CLI - Parse and query cargo build errors",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )

    subparsers = parser.add_subparsers(dest="command", help="Command to run")

    # build
    p_build = subparsers.add_parser("build", help="Run cargo build and parse errors")
    p_build.set_defaults(func=cmd_build)

    # parse
    p_parse = subparsers.add_parser("parse", help="Parse existing errors.txt")
    p_parse.set_defaults(func=cmd_parse)

    # summary
    p_summary = subparsers.add_parser("summary", help="Show summary statistics")
    p_summary.set_defaults(func=cmd_summary)

    # top-files
    p_top_files = subparsers.add_parser("top-files", help="Show top N files with most errors")
    p_top_files.add_argument("count", type=int, nargs="?", default=10, help="Number of files to show")
    p_top_files.add_argument("-e", "--only-errors", action="store_true", help="Only count errors, exclude warnings")
    p_top_files.set_defaults(func=cmd_top_files)

    # top-errors
    p_top_errors = subparsers.add_parser("top-errors", help="Show top N most common error codes")
    p_top_errors.add_argument("count", type=int, nargs="?", default=10, help="Number of codes to show")
    p_top_errors.add_argument("-e", "--only-errors", action="store_true", help="Only show errors, exclude warnings")
    p_top_errors.set_defaults(func=cmd_top_errors)

    # file
    p_file = subparsers.add_parser("file", help="Show all errors in a specific file")
    p_file.add_argument("path", help="File path (partial match)")
    p_file.add_argument("-d", "--detailed", action="store_true", help="Show detailed view")
    p_file.add_argument("-e", "--only-errors", action="store_true", help="Only show errors, exclude warnings")
    p_file.set_defaults(func=cmd_file)

    # code
    p_code = subparsers.add_parser("code", help="Show all errors with a specific code")
    p_code.add_argument("code", help="Error code (e.g., E0283)")
    p_code.add_argument("-d", "--detailed", action="store_true", help="Show detailed view")
    p_code.add_argument("-l", "--limit", type=int, default=20, help="Max errors to show")
    p_code.set_defaults(func=cmd_code)

    # detail
    p_detail = subparsers.add_parser("detail", help="Show detailed view of error at index")
    p_detail.add_argument("index", type=int, help="Error index")
    p_detail.set_defaults(func=cmd_detail)

    # next
    p_next = subparsers.add_parser("next", help="Show next error to fix")
    p_next.set_defaults(func=cmd_next)

    # fix-plan
    p_plan = subparsers.add_parser("fix-plan", help="Generate fix plan for LLM")
    p_plan.set_defaults(func=cmd_fix_plan)

    args = parser.parse_args()

    if args.command is None:
        parser.print_help()
        sys.exit(1)

    args.func(args)


if __name__ == "__main__":
    main()
