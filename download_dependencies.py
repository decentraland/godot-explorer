import os
import urllib.request
import zipfile
import subprocess
import sys
import platform
import glob
from folder_hash import get_rust_folder_hash

sys.stdout.reconfigure(line_buffering=True)

ANDROID_DEP_URL = "https://godot-artifacts.kuruk.net/android_deps.zip"
LIB_DEP_URL_TEMPLATE = "https://godot-artifacts.kuruk.net/{hash}/libdclgodot.zip"
FFMPEG_WINDOWS_URL = "https://godot-artifacts.kuruk.net/ffmpeg_windows.zip"

SCRIPT_DIR = os.path.dirname(os.path.realpath(__file__))

# Paths
BIN_FOLDER = os.path.join(SCRIPT_DIR, ".bin")
ANDROID_ZIP = os.path.join(BIN_FOLDER, "android_dependencies.zip")
ANDROID_TARGET = os.path.join(SCRIPT_DIR, "../android/build/libs/debug/arm64-v8a/deps/")
RUST_TARGET = os.path.join(SCRIPT_DIR, "lib/target/")
RUST_VERSION_FILE = os.path.join(RUST_TARGET, "downloaded_rust_version.txt")
FOLDER_HASH_SCRIPT = os.path.join(SCRIPT_DIR, "folder_hash.py")

FFMPEG_ZIP = os.path.join(BIN_FOLDER, "ffmpeg_windows.zip")
FFMPEG_TARGET = os.path.join(RUST_TARGET, "libdclgodot_windows/")

def read_version_file(path):
    if os.path.exists(path):
        with open(path, 'r') as f:
            return f.read().strip()
    return ""

def write_version_file(path, value):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(value)

def download_file(url, dest_path):
    os.makedirs(os.path.dirname(dest_path), exist_ok=True)
    print(f"Downloading: {url}")

    req = urllib.request.Request(
        url,
        headers={"User-Agent": "Mozilla/5.0 (compatible; Python downloader)"}
    )

    with urllib.request.urlopen(req) as response:
        total = int(response.headers.get("Content-Length", 0))
        downloaded = 0
        with open(dest_path, 'wb') as out:
            while True:
                chunk = response.read(8192)
                if not chunk:
                    break
                out.write(chunk)
                downloaded += len(chunk)
                if total > 0:
                    percent = int(downloaded * 100 / total)
                    print(f"\rProgress: {percent}% ({downloaded}/{total} bytes)", end='', flush=True)

    print("\nDownload complete.")

def unzip_file(zip_path, extract_to):
    print(f"Extracting {zip_path} -> {extract_to}")
    with zipfile.ZipFile(zip_path, 'r') as zip_ref:
        zip_ref.extractall(extract_to)
    print("Extraction complete.")

def download_android_dependencies():
    if not os.path.exists(ANDROID_ZIP):
        print("Android dependencies missing. Downloading...")
        download_file(ANDROID_DEP_URL, ANDROID_ZIP)
        print("✅ Android dependency downloaded")
    else:
        print("✅ Android dependency OK")

def download_ffmpeg_windows():
    if platform.system().lower() != "windows":
        return

    # Check if any av*.dll files are already extracted
    dll_pattern = os.path.join(FFMPEG_TARGET, "av*.dll")
    existing_dlls = glob.glob(dll_pattern)

    if existing_dlls:
        print("✅ FFMPEG DLLs already present:", [os.path.basename(f) for f in existing_dlls])
        return

    print("⚠️  FFMPEG DLLs missing. Downloading...")

    if not os.path.exists(FFMPEG_ZIP):
        print("FFMPEG Windows dependency missing. Downloading...")
        download_file(FFMPEG_WINDOWS_URL, FFMPEG_ZIP)
    else:
        print("✅ FFMPEG ZIP already downloaded")

    unzip_file(FFMPEG_ZIP, FFMPEG_TARGET)
    os.remove(FFMPEG_ZIP)
    print(f"✅ FFMPEG extracted to {FFMPEG_TARGET}")

def download_rust_lib():
    rust_hash = get_rust_folder_hash()
    if not rust_hash:
        print("❌ Failed to compute rust folder hash.")
        return

    current_version = read_version_file(RUST_VERSION_FILE)
    if current_version == "" and os.path.exists(RUST_TARGET):
        print("⚠️  Self-build detected. Please delete lib/target folder to allow downloading")
        return        
    if current_version == rust_hash:
        print(f"✅ Rust lib already up-to-date (version {rust_hash})")
        return

    lib_url = LIB_DEP_URL_TEMPLATE.format(hash=rust_hash)
    zip_path = os.path.join(BIN_FOLDER, "libdclgodot.zip")

    print(f"Rust lib version changed. Downloading {lib_url}...")
    download_file(lib_url, zip_path)
    unzip_file(zip_path, RUST_TARGET)
    os.remove(zip_path)

    write_version_file(RUST_VERSION_FILE, rust_hash)
    print(f"✅ Rust lib updated to version {rust_hash}")

if __name__ == "__main__":
    download_android_dependencies()
    download_rust_lib()

    if platform.system().lower() == "windows":
        download_ffmpeg_windows()