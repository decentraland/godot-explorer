#!/usr/bin/env python3
import os
import hashlib
import unicodedata
import sys

def find_files(base):
    for root, dirs, files in os.walk(base):
        if 'target' in dirs:
            dirs.remove('target')
        for f in files:
            if f.endswith('.rs') or f.endswith('.toml'):
                yield os.path.join(root, f)

def hash_file(path):
    with open(path, 'rb') as f:
        return hashlib.sha256(f.read()).hexdigest()

def get_base_dir():
    # Always hash the `lib/src/` folder relative to the script
    script_dir = os.path.dirname(os.path.realpath(__file__))
    return os.path.join(script_dir, "lib", "src")

def warn_if_autocrlf_not_input():
    try:
        import subprocess
        result = subprocess.run(
            ["git", "config", "--get", "core.autocrlf"],
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True
        )
        value = result.stdout.strip().lower()
        if value != "input":
            print("⚠️  WARNING: Git core.autocrlf is '%s' — this can cause inconsistent file contents across systems." % (value or "unset"))
            print("   To fix, run:")
            print("   git config --global core.autocrlf input")
    except Exception as e:
        print("⚠️  Git not detected or not a Git repository.")


def main():
    base = get_base_dir()

    files = list(find_files(base))

    # Normalize and sort relative paths
    rel_paths = [
        unicodedata.normalize("NFC", os.path.relpath(f, base)).replace("\\", "/")
        for f in files
    ]
    sorted_paths = sorted(rel_paths)

    lines = []
    for rel in sorted_paths:
        full_path = os.path.join(base, rel)
        file_hash = hash_file(full_path)
        line = f"{file_hash}  {rel}"
        lines.append(line)

    # Join all lines and encode to UTF-8 with Unix line endings
    joined = "\n".join(lines).encode("utf-8")

    folder_hash = hashlib.sha256(joined).hexdigest()
    print(folder_hash)

if __name__ == "__main__":
    main()
    warn_if_autocrlf_not_input()
