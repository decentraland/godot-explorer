#!/usr/bin/env python3
"""
Checks for snapshot test differences and outputs a markdown report.

Usage:
  python3 check_snapshot_diffs.py              # Print report to stdout
  python3 check_snapshot_diffs.py --json       # Output JSON for GitHub Actions
"""

import argparse
import json
import sys
from pathlib import Path


# Snapshot directories to check
SNAPSHOT_DIRS = [
    'tests/snapshots/scenes',
    'tests/snapshots/avatar-image-generation',
    'tests/snapshots/client',
]


def find_snapshot_diffs(base_dir: Path) -> list:
    """Find all snapshot differences by looking at comparison folders."""
    diffs = []

    for snapshot_dir in SNAPSHOT_DIRS:
        comparison_dir = base_dir / snapshot_dir / 'comparison'
        original_dir = base_dir / snapshot_dir

        if not comparison_dir.exists():
            continue

        # Find all .diff.png files (these indicate a difference)
        for diff_file in comparison_dir.glob('*.diff.png'):
            test_name = diff_file.stem.replace('.diff', '')

            # Check if the generated and original files exist
            generated_file = comparison_dir / f'{test_name}.png'
            original_file = original_dir / f'{test_name}.png'

            if generated_file.exists():
                category = Path(snapshot_dir).name
                diffs.append({
                    'category': category,
                    'test_name': test_name,
                    'original': str(original_file.relative_to(base_dir)) if original_file.exists() else None,
                    'generated': str(generated_file.relative_to(base_dir)),
                    'diff': str(diff_file.relative_to(base_dir)),
                    'original_exists': original_file.exists(),
                })

    return diffs


def generate_markdown_report(diffs: list, run_url: str = None) -> str:
    """Generate a markdown report of snapshot differences."""
    if not diffs:
        return ""

    lines = [
        "## 📸 Snapshot Test Differences\n",
        f"Found **{len(diffs)}** snapshot(s) with differences.\n",
    ]

    if run_url:
        lines.append(f"Download the [coverage-snapshots artifact]({run_url}) to review the full images.\n")

    lines.append("")
    lines.append("| Category | Test Name | Status |")
    lines.append("|----------|-----------|--------|")

    for diff in sorted(diffs, key=lambda x: (x['category'], x['test_name'])):
        status = "⚠️ Changed" if diff['original_exists'] else "🆕 New"
        lines.append(f"| {diff['category']} | `{diff['test_name']}` | {status} |")

    lines.append("")
    lines.append("### How to review")
    lines.append("1. Download the `coverage-snapshots` artifact from this workflow run")
    lines.append("2. The comparison folder contains:")
    lines.append("   - `<name>.png` - Generated snapshot from this run")
    lines.append("   - `<name>.diff.png` - Visual difference highlighting changes")
    lines.append("3. Original snapshots are in the parent folder with the same name")
    lines.append("")
    lines.append("### To update snapshots")
    lines.append("If the changes are expected, copy the generated snapshots to replace the originals.")

    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description='Check for snapshot test differences')
    parser.add_argument('--json', action='store_true', help='Output JSON format')
    parser.add_argument('--run-url', type=str, help='GitHub Actions run URL for artifact link')
    args = parser.parse_args()

    # Find the base directory (repository root)
    script_dir = Path(__file__).parent
    base_dir = script_dir.parent

    diffs = find_snapshot_diffs(base_dir)

    if args.json:
        output = {
            'has_diffs': len(diffs) > 0,
            'count': len(diffs),
            'diffs': diffs,
            'markdown': generate_markdown_report(diffs, args.run_url) if diffs else "",
        }
        print(json.dumps(output))
    else:
        if diffs:
            print(generate_markdown_report(diffs, args.run_url))
            sys.exit(1)
        else:
            print("No snapshot differences found.")
            sys.exit(0)


if __name__ == '__main__':
    main()
