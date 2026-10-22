#!/usr/bin/env python3
"""Generate iris masks for the base-avatar eyes (#2439).

Base eyes ship only a colour texture and no `_mask.png`, so the eye colour had nothing to tint.
This writes one mask per base-avatar eye, keyed by the texture's content hash:

- Most eyes: the mask is the texture's own darkness, so every iris (drawn in greys) is tinted while
  the white of the eye, the catch-lights and the eyeliner keep their art.
- A couple of eyes draw the iris as a solid black disc with no shading to tint. Multiplying black by
  the eye colour stays black, so those get an ellipse over the disc and `BASE_EYE_MASKS_REPLACE`,
  which makes the shader paint the iris with the picked colour instead of multiplying it.

Mask convention matches a wearable `_mask.png` for eyes: black = tinted, white = untouched.
`avatar.gd` looks the mask up by the eye texture's content hash, so a redeployed texture simply
stops matching instead of getting a misaligned mask.

Usage: python3 tools/eye_masks/generate.py   (needs numpy + Pillow and network access)
It prints the GDScript tables to paste into `godot/src/decentraland_components/avatar/avatar.gd`.
"""

import io
import os
import urllib.request

import numpy as np
from PIL import Image

CATALOGUE_URL = (
    "https://peer.decentraland.org/lambdas/collections/wearables"
    "?collectionId=urn:decentraland:off-chain:base-avatars"
)
CONTENT_URL = "https://peer.decentraland.org/content/contents/"
OUT_DIR = os.path.join(os.path.dirname(__file__), "../../godot/assets/avatar/eye_masks")

# Full tint below FULL_TINT_LUMA, none above NO_TINT_LUMA: the eye white sits at 1.0, irises between
# 0.1 and 0.6, and the anti-aliased rim in between gets a partial tint.
FULL_TINT_LUMA = 0.60
NO_TINT_LUMA = 0.88

# Eyes whose iris is a solid black disc: multiplying it can only ever stay black, so the shader
# replaces the disc with the eye colour (see BASE_EYE_MASKS_REPLACE) and the mask is its outline.
SOLID_BLACK_IRIS = ("eyes_03", "f_eyes_03")


def fetch_eye_hashes() -> dict[str, str]:
    request = urllib.request.Request(CATALOGUE_URL, headers={"User-Agent": "Mozilla/5.0"})
    catalogue = json_load(urllib.request.urlopen(request).read())
    hashes = {}
    for wearable in catalogue["wearables"]:
        if wearable["data"]["category"] != "eyes":
            continue
        name = wearable["id"].rsplit(":", 1)[-1]
        contents = wearable["data"]["representations"][0]["contents"]
        texture = next(c for c in contents if "mask" not in c["key"].lower())
        hashes[name] = texture["url"]
    return hashes


def json_load(raw: bytes):
    import json

    return json.loads(raw)


def blob_ellipses(texture: Image.Image) -> list[tuple[float, float, float, float]]:
    """Fit one ellipse per eye over the largest solid black blob on each side of the texture."""
    rgba = np.asarray(texture.convert("RGBA")).astype(np.float64) / 255.0
    dark = (rgba[..., :3].max(-1) < 0.15) & (rgba[..., 3] > 0.5)
    height, width = dark.shape
    seen = np.zeros_like(dark)
    ellipses = []
    for y in range(height):
        for x in range(width):
            if not dark[y, x] or seen[y, x]:
                continue
            stack, points = [(y, x)], []
            seen[y, x] = True
            while stack:
                cy, cx = stack.pop()
                points.append((cy, cx))
                for ny, nx in ((cy + 1, cx), (cy - 1, cx), (cy, cx + 1), (cy, cx - 1)):
                    if 0 <= ny < height and 0 <= nx < width and dark[ny, nx] and not seen[ny, nx]:
                        seen[ny, nx] = True
                        stack.append((ny, nx))
            points = np.array(points, dtype=np.float64)
            top, left = points.min(0)
            bottom, right = points.max(0)
            if len(points) < 300 or min(bottom - top, right - left) < 12:
                continue
            # Moments rather than the bounding box: the iris disc shares a blob with the lid line,
            # which would otherwise stretch the ellipse past the eye.
            cy, cx = points.mean(0)
            # 1.8σ keeps the ellipse on the iris disc: any spill lands on the white of the eye, and
            # the replace path paints the mask flat, so an oversized ellipse shows as a colour halo.
            ellipses.append(
                (cx, cy, 1.8 * points[:, 1].std(), 1.8 * points[:, 0].std())
            )
    return ellipses


def make_mask(texture: Image.Image, name: str) -> Image.Image:
    rgba = np.asarray(texture.convert("RGBA")).astype(np.float64) / 255.0
    luminance = rgba[..., :3].mean(-1)
    if name in SOLID_BLACK_IRIS:
        height, width = luminance.shape
        yy, xx = np.mgrid[0:height, 0:width].astype(np.float64)
        weight = np.zeros((height, width))
        for cx, cy, rx, ry in blob_ellipses(texture):
            distance = np.hypot((xx - cx) / rx, (yy - cy) / ry)
            weight = np.maximum(weight, np.clip((1.0 - distance) * min(rx, ry) + 0.5, 0.0, 1.0))
    else:
        weight = np.clip(
            (NO_TINT_LUMA - luminance) / (NO_TINT_LUMA - FULL_TINT_LUMA), 0.0, 1.0
        )
    weight *= rgba[..., 3] > 0.2
    return Image.fromarray(np.round(255.0 * (1.0 - weight)).astype(np.uint8))


def main() -> None:
    os.makedirs(OUT_DIR, exist_ok=True)
    hashes = fetch_eye_hashes()
    for name in sorted(hashes):
        request = urllib.request.Request(hashes[name], headers={"User-Agent": "Mozilla/5.0"})
        with urllib.request.urlopen(request) as response:
            texture = Image.open(io.BytesIO(response.read()))
        path = os.path.join(OUT_DIR, f"{name}_mask.png")
        make_mask(texture, name).save(path, optimize=True)
        print(f"{name} -> {os.path.relpath(path)}")

    content_hash = {name: hashes[name].rsplit("/", 1)[-1] for name in hashes}
    print("\nconst BASE_EYE_MASKS: Dictionary[String, String] = {")
    for name in sorted(content_hash):
        print(f'\t"{content_hash[name]}":')
        print(f'\t"res://assets/avatar/eye_masks/{name}_mask.png",')
    print("}\n")
    print("const BASE_EYE_MASKS_REPLACE: PackedStringArray = [")
    for name in SOLID_BLACK_IRIS:
        print(f'\t"{content_hash[name]}",')
    print("]")


if __name__ == "__main__":
    main()
