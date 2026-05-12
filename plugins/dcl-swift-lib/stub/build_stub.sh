#!/bin/bash
# Builds the no-op GDExtension stub used on desktop platforms where the real
# Swift StoreKit framework is not built. Output lands in
# godot/<platform>/dcl-swift-lib/ to mirror the iOS framework layout.
#
# Usage:
#   ./build_stub.sh                # auto-detect host (only macOS arm64 wired today)
#   ./build_stub.sh macos.arm64    # explicit target
set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SRC="${SCRIPT_DIR}/dcl_swift_lib_stub.c"
GODOT_ROOT="${SCRIPT_DIR}/../../../godot"

TARGET="${1:-}"
if [ -z "${TARGET}" ]; then
	UNAME_S="$(uname -s)"
	UNAME_M="$(uname -m)"
	case "${UNAME_S}-${UNAME_M}" in
		Darwin-arm64) TARGET="macos.arm64" ;;
		*) echo "ERROR: no auto-detected target for ${UNAME_S}-${UNAME_M}; pass one explicitly" >&2; exit 1 ;;
	esac
fi

case "${TARGET}" in
	macos.arm64)
		OUT_DIR="${GODOT_ROOT}/macos/dcl-swift-lib"
		OUT="${OUT_DIR}/libdcl_swift_lib_stub.macos.arm64.dylib"
		mkdir -p "${OUT_DIR}"
		clang -shared -fPIC -O2 -arch arm64 \
			-Wl,-install_name,@rpath/libdcl_swift_lib_stub.macos.arm64.dylib \
			-o "${OUT}" "${SRC}"
		;;
	*)
		echo "ERROR: target ${TARGET} not implemented yet" >&2
		exit 1
		;;
esac

echo "Built stub: ${OUT}"
