import os
import hashlib
import unicodedata

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

def main():
    # Get the directory where the script is located
    script_dir = os.path.dirname(os.path.realpath(__file__))
    base = os.path.join(script_dir, "lib", "src")

    files = list(find_files(base))
    rel_paths = [
        unicodedata.normalize("NFC", os.path.relpath(f, base)).replace("\\", "/")
        for f in files
    ]
    sorted_paths = sorted(rel_paths)

    lines = []
    for rel in sorted_paths:
        full_path = os.path.join(base, rel)
        h = hash_file(full_path)
        lines.append(f"{h}  {rel}")

    joined = "\n".join(lines).encode("utf-8")
    folder_hash = hashlib.sha256(joined).hexdigest()

    print(folder_hash)

if __name__ == "__main__":
    main()
