#!/usr/bin/env python3
"""Fails when a native library in an APK has a LOAD segment aligned below 16 KB.

Google Play requires 16 KB page-size support for apps targeting Android 15+; a library linked
with a smaller `p_align` is rejected on 16 KB devices. Usage: check-16kb-alignment.py app.apk
"""
import struct
import sys
import zipfile

MIN_ALIGN = 0x4000
PT_LOAD = 1
ARM64_ABIS = ("lib/arm64-v8a/", "lib/x86_64/")


def load_aligns(data: bytes):
    if data[:4] != b"\x7fELF" or data[4] != 2:
        return []
    little = data[5] == 1
    end = "<" if little else ">"
    phoff, = struct.unpack_from(end + "Q", data, 0x20)
    phentsize, phnum = struct.unpack_from(end + "HH", data, 0x36)
    aligns = []
    for i in range(phnum):
        off = phoff + i * phentsize
        p_type, = struct.unpack_from(end + "I", data, off)
        if p_type == PT_LOAD:
            aligns.append(struct.unpack_from(end + "Q", data, off + 0x30)[0])
    return aligns


def main(apk_path: str) -> int:
    bad = []
    checked = 0
    with zipfile.ZipFile(apk_path) as apk:
        for info in apk.infolist():
            if not info.filename.endswith(".so") or not info.filename.startswith(ARM64_ABIS):
                continue
            checked += 1
            aligns = load_aligns(apk.read(info))
            if aligns and min(aligns) < MIN_ALIGN:
                bad.append((info.filename, min(aligns)))
    if checked == 0:
        print(f"no 64-bit native libraries found in {apk_path}")
        return 1
    for name, align in bad:
        print(f"{name}: LOAD alignment {align:#x} < {MIN_ALIGN:#x}")
    print(f"checked {checked} libraries, {len(bad)} below 16 KB")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
