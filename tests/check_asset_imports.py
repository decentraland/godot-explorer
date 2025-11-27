#!/usr/bin/env python3
"""
Validates that Godot assets are imported with correct settings:
- SVG files must be imported as DPITexture (importer="svg", type="DPITexture")
- PNG/Image files must be VRAM compressed (vram_texture=true or compress/mode=2)

Usage:
  python3 check_asset_imports.py         # Check only
  python3 check_asset_imports.py --fix   # Fix import files
"""

import argparse
import hashlib
import re
import sys
from pathlib import Path
from typing import List, Tuple

# Image extensions to check for VRAM compression
IMAGE_EXTENSIONS = {'.png', '.jpg', '.jpeg', '.webp', '.bmp', '.tga'}

# Directories to exclude from checking
EXCLUDED_DIRS = {
    'android/build',
    '.godot',
    'addons/godot-xr-tools',  # Third-party addon
    'exports',  # Export output folder
}


def should_exclude(path: Path, godot_dir: Path) -> bool:
    """Check if a path should be excluded from validation."""
    rel_path = path.relative_to(godot_dir)
    for excluded in EXCLUDED_DIRS:
        if str(rel_path).startswith(excluded):
            return True
    return False


def parse_import_file(import_path: Path) -> dict:
    """Parse a Godot .import file and return key settings."""
    content = import_path.read_text()
    settings = {'_content': content}

    # Extract importer type
    importer_match = re.search(r'importer="([^"]+)"', content)
    if importer_match:
        settings['importer'] = importer_match.group(1)

    # Extract type
    type_match = re.search(r'type="([^"]+)"', content)
    if type_match:
        settings['type'] = type_match.group(1)

    # Extract uid
    uid_match = re.search(r'uid="([^"]+)"', content)
    if uid_match:
        settings['uid'] = uid_match.group(1)

    # Extract source_file
    source_match = re.search(r'source_file="([^"]+)"', content)
    if source_match:
        settings['source_file'] = source_match.group(1)

    # Extract vram_texture from metadata
    vram_match = re.search(r'"vram_texture":\s*(true|false)', content)
    if vram_match:
        settings['vram_texture'] = vram_match.group(1) == 'true'

    # Extract compress/mode
    compress_match = re.search(r'compress/mode=(\d+)', content)
    if compress_match:
        settings['compress_mode'] = int(compress_match.group(1))

    return settings


def check_svg_import(import_path: Path, settings: dict) -> Tuple[bool, str]:
    """Check if an SVG file is correctly imported as DPITexture."""
    importer = settings.get('importer', '')
    texture_type = settings.get('type', '')

    if importer == 'svg' and texture_type == 'DPITexture':
        return True, ""

    issues = []
    if importer != 'svg':
        issues.append(f'importer="{importer}" (expected "svg")')
    if texture_type != 'DPITexture':
        issues.append(f'type="{texture_type}" (expected "DPITexture")')

    return False, ', '.join(issues)


def check_image_import(import_path: Path, settings: dict) -> Tuple[bool, str]:
    """Check if an image file is VRAM compressed."""
    vram_texture = settings.get('vram_texture', False)
    compress_mode = settings.get('compress_mode', 0)

    # VRAM compression is indicated by either:
    # - vram_texture: true in metadata
    # - compress/mode=2 (VRAM Compressed)
    if vram_texture or compress_mode == 2:
        return True, ""

    return False, f'vram_texture={vram_texture}, compress/mode={compress_mode} (expected vram_texture=true or compress/mode=2)'


def generate_import_hash(source_file: str) -> str:
    """Generate a hash for the imported file path (mimics Godot's behavior)."""
    return hashlib.md5(source_file.encode()).hexdigest()


def fix_svg_import(import_path: Path, settings: dict) -> bool:
    """Fix an SVG import file to use DPITexture."""
    source_file = settings.get('source_file', '')
    uid = settings.get('uid', '')

    if not source_file:
        return False

    # Generate the import hash
    import_hash = generate_import_hash(source_file)
    filename = Path(source_file).name

    # Create the new import file content
    new_content = f'''[remap]

importer="svg"
type="DPITexture"
uid="{uid}"
path="res://.godot/imported/{filename}-{import_hash}.dpitex"

[deps]

source_file="{source_file}"
dest_files=["res://.godot/imported/{filename}-{import_hash}.dpitex"]

[params]

base_scale=1.0
saturation=1.0
color_map={{}}
compress=true
'''

    import_path.write_text(new_content)
    return True


def fix_image_import(import_path: Path, settings: dict) -> bool:
    """Fix an image import file to use VRAM compression."""
    content = settings.get('_content', '')

    if not content:
        return False

    # Change compress/mode from 0 to 2
    new_content = re.sub(r'compress/mode=\d+', 'compress/mode=2', content)

    # Update detect_3d/compress_to to 0 (disabled, since we're already VRAM compressed)
    new_content = re.sub(r'detect_3d/compress_to=\d+', 'detect_3d/compress_to=0', new_content)

    import_path.write_text(new_content)
    return True


def find_and_check_assets(godot_dir: Path, fix: bool = False) -> Tuple[List[str], List[str], int, int]:
    """Find all asset import files and validate them."""
    svg_errors = []
    image_errors = []
    svg_fixed = 0
    image_fixed = 0

    for import_path in godot_dir.rglob('*.import'):
        if should_exclude(import_path, godot_dir):
            continue

        # Get the source file name (remove .import suffix)
        source_file = import_path.stem
        source_ext = Path(source_file).suffix.lower()

        settings = parse_import_file(import_path)
        rel_path = import_path.relative_to(godot_dir)

        if source_ext == '.svg':
            is_valid, issue = check_svg_import(import_path, settings)
            if not is_valid:
                if fix:
                    if fix_svg_import(import_path, settings):
                        svg_fixed += 1
                    else:
                        svg_errors.append(f"  {rel_path}: {issue} (could not fix)")
                else:
                    svg_errors.append(f"  {rel_path}: {issue}")

        elif source_ext in IMAGE_EXTENSIONS:
            is_valid, issue = check_image_import(import_path, settings)
            if not is_valid:
                if fix:
                    if fix_image_import(import_path, settings):
                        image_fixed += 1
                    else:
                        image_errors.append(f"  {rel_path}: {issue} (could not fix)")
                else:
                    image_errors.append(f"  {rel_path}: {issue}")

    return svg_errors, image_errors, svg_fixed, image_fixed


def main():
    parser = argparse.ArgumentParser(
        description='Validate and fix Godot asset import settings'
    )
    parser.add_argument(
        '--fix',
        action='store_true',
        help='Fix incorrect import settings'
    )
    args = parser.parse_args()

    # Find the godot directory relative to script location
    script_dir = Path(__file__).parent
    godot_dir = script_dir.parent / 'godot'

    if not godot_dir.exists():
        print(f"Error: Godot directory not found at {godot_dir}", file=sys.stderr)
        sys.exit(1)

    if args.fix:
        print(f"Fixing asset imports in {godot_dir}...")
    else:
        print(f"Checking asset imports in {godot_dir}...")
    print()

    svg_errors, image_errors, svg_fixed, image_fixed = find_and_check_assets(godot_dir, fix=args.fix)

    if args.fix:
        if svg_fixed > 0:
            print(f"Fixed {svg_fixed} SVG file(s) to use DPITexture")
        if image_fixed > 0:
            print(f"Fixed {image_fixed} image file(s) to use VRAM compression")
        if svg_fixed > 0 or image_fixed > 0:
            print()
            print("NOTE: Run 'cargo run -- import-assets' to reimport the fixed assets")
            print()

    has_errors = False

    if svg_errors:
        has_errors = True
        print(f"SVG files not imported as DPITexture ({len(svg_errors)} errors):")
        for error in sorted(svg_errors):
            print(error)
        print()

    if image_errors:
        has_errors = True
        print(f"Image files not VRAM compressed ({len(image_errors)} errors):")
        for error in sorted(image_errors):
            print(error)
        print()

    if has_errors:
        print("Asset import validation FAILED")
        print()
        print("To fix automatically, run:")
        print("  python3 tests/check_asset_imports.py --fix")
        print()
        print("Or fix manually in Godot editor:")
        print()
        print("To fix SVG imports:")
        print("  1. Select the SVG file in Godot editor")
        print("  2. Go to Import tab")
        print("  3. Change import type to 'SVG' (DPITexture)")
        print("  4. Click 'Reimport'")
        print()
        print("To fix image imports:")
        print("  1. Select the image file in Godot editor")
        print("  2. Go to Import tab")
        print("  3. Change Compress Mode to 'VRAM Compressed'")
        print("  4. Click 'Reimport'")
        sys.exit(1)
    elif args.fix and (svg_fixed > 0 or image_fixed > 0):
        print("All assets fixed successfully!")
        sys.exit(0)
    else:
        print("All assets imported correctly!")
        sys.exit(0)


if __name__ == '__main__':
    main()
