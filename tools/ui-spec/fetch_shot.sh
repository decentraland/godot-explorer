#!/usr/bin/env bash
# fetch_shot.sh <manifest-id> <image_url>   - download a get_screenshot URL into docs/rn-ui-spec/shots and validate it
set -euo pipefail
R="$(cd "$(dirname "$0")/../.." && pwd)/docs/rn-ui-spec"
f=$(python3 -I - "$R/manifest.json" "$1" <<'PY'
import json,sys
print(next(e['file'] for e in json.load(open(sys.argv[1]))['screens'] if e['id']==sys.argv[2]))
PY
)
mkdir -p "$R/$(dirname "$f")"
curl -sfL -o "$R/$f" "$2"
python3 -I -c "from PIL import Image;import sys;i=Image.open(sys.argv[1]);print('ok',sys.argv[1],i.size)" "$R/$f"
