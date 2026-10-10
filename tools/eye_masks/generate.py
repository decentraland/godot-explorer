#!/usr/bin/env python3
"""Generate iris masks for the base-avatar eyes (#2439).

Base eyes ship only a color texture, no `_mask.png`, so no client can tint the iris and the
eye color picker does nothing on them. For every base eye with a drawn iris, this paints an
anti-aliased ellipse over each iris (tuned by hand below), clipped so catch-lights and the
sclera stay untinted, and writes `godot/assets/avatar/eye_masks/<name>_mask.png`.

Mask convention matches a wearable `_mask.png` for eyes: black = tinted with the eye color,
white = untouched. `avatar.gd` picks the mask by the eye texture's content hash, so a
redeployed texture simply stops matching instead of getting a misaligned mask.

Usage: python3 tools/eye_masks/generate.py   (needs numpy + Pillow and network access)
"""

import io
import os
import urllib.request

import numpy as np
from PIL import Image

CONTENT_URL = "https://peer.decentraland.org/content/contents/"
OUT_DIR = os.path.join(os.path.dirname(__file__), "../../godot/assets/avatar/eye_masks")

# name: (texture content hash, [(cx, cy, rx, ry) per eye], iris drawn white)
# Coordinates are in the 256x256 texture. The other base eyes have no iris to tint
# (dots, lines, spirals, solid black pupils).
EYES = {
    "eyes_02": ("bafkreidbeafduyssl7y2gdjoxq3ouagn6beivty4v6eovpl5iz2z72jeqm",
                [(77, 60, 10, 10), (177, 60, 10, 10)], False),
    "eyes_04": ("bafkreiddfvmi3kvxgjqcwjev6ot4rkeswbwv7nt5pmpr5du3dywpbiiegq",
                [(76, 59, 14.5, 14.5), (178, 59, 14.5, 14.5)], False),
    "eyes_06": ("bafkreigmp6kksur4nln6fsnt4bx3nwmjtztw22ti47zpqxlhihw7x3s5bm",
                [(77.5, 62, 10, 10), (178.5, 62, 10, 10)], False),
    "eyes_08": ("bafkreiekdf7ryiigiairg2zz3vc6xcddlcrzotygtupyafjzv4ahvunjaq",
                [(78, 57, 12, 12), (176, 57, 12, 12)], False),
    "eyes_11": ("bafkreifwflrbyaq4rlisyrzgmjunm4ah55fznzt4mvcadgt5tp3iriv6nu",
                [(78, 60, 8.5, 13), (179, 60, 9, 13)], True),
    "eyes_12": ("bafkreifzn5vsmpmu46zow4agsccsaimdywxr5a5cc4lrq3ldepkypfse54",
                [(77, 63, 7.5, 7.5), (178, 63, 7.5, 7.5)], False),
    "eyes_14": ("bafkreic5ileafhebzefbcojhoc63yt5m7chpq7jsvng54pvxxtcyl6dg6e",
                [(78, 62.8, 12.5, 12.5), (173, 62.8, 12.5, 12.5)], False),
    "eyes_17": ("bafkreictmrdsxaxzifrldbu2cdgtpxjtwwcx2hxpksgrhqi2tavdopcjua",
                [(80.4, 64.6, 10.5, 10.5), (176, 64.6, 10.5, 10.5)], False),
    "f_eyes_01": ("bafkreihyo4jsqjsgfwdwucdfyvbkkww7mgkc4dsypbarxpwuhgpygxjmaq",
                  [(79, 62, 12, 13), (177.4, 62, 12, 13)], True),
    "f_eyes_02": ("bafkreidm6bs6vixgdi5obils34ujmhittgf43dwie6gjzlrc644gzkhjsu",
                  [(80.5, 60.5, 12, 12), (175, 60.5, 12, 12)], False),
    "f_eyes_04": ("bafkreigmm5zfnzu4egbiajlx4iivfy36stx243zytg5fvk2jfgmlka7zci",
                  [(82.5, 59.75, 11, 11), (174, 59.75, 11, 11)], False),
    "f_eyes_06": ("bafkreiho5cykajxijt3bss7h6s7pobrspzfave2wu6gfj2ndzreynz5bum",
                  [(78, 62, 12.5, 12.5), (177, 62, 12.5, 12.5)], False),
    "f_eyes_07": ("bafkreihsngpwvcdmgrcj5tmmfzwpppwmrsc6r3b2d5zvy4ud4zcaxdvjw4",
                  [(77.5, 56, 11, 11), (177, 56, 11, 11)], False),
    "f_eyes_08": ("bafkreicqdnzkngyjm25eyd2mgfzk4rmoaeakd3bjcyh6i277q2rstktxza",
                  [(75.5, 71, 14, 14), (175, 70.5, 14, 14)], False),
    "f_eyes_09": ("bafkreif7kkkztcw2ts6ijo7pqc7k2sgc5kvab46t566hjtd7slxjicxste",
                  [(80, 59, 11, 11), (175, 59, 11, 11)], False),
    "f_eyes_10": ("bafkreifin5nam6vkl25mugq7rtjaslxmkc2bxjksr56teirlpl3vv2ahhy",
                  [(82.5, 59.75, 11.5, 11.5), (173, 59.75, 11.5, 11.5)], False),
    "f_eyes_11": ("bafkreicdjq7mi4cs2lcnzbiwnkxg3pbhzlie4yethzvcekbekqvto32ewu",
                  [(80, 62.25, 12.5, 16), (175, 62.25, 12.5, 16)], False),
}


def make_mask(texture: Image.Image, irises, white_iris: bool) -> Image.Image:
    rgba = np.asarray(texture.convert("RGBA")).astype(np.float64) / 255.0
    lum = rgba[..., :3].mean(-1)
    height, width = lum.shape
    yy, xx = np.mgrid[0:height, 0:width].astype(np.float64)
    weight = np.zeros((height, width))
    for cx, cy, rx, ry in irises:
        dist = np.hypot((xx - cx) / rx, (yy - cy) / ry)  # 1.0 on the ellipse
        inside_px = (1.0 - dist) * min(rx, ry)
        weight = np.maximum(weight, np.clip(inside_px + 0.5, 0.0, 1.0))  # 1 px AA rim
    if not white_iris:
        weight *= np.clip((0.95 - lum) / 0.08, 0.0, 1.0)  # keep catch-lights and sclera white
    return Image.fromarray(np.round(255.0 * (1.0 - weight)).astype(np.uint8))  # 2D uint8 -> "L"


def main() -> None:
    os.makedirs(OUT_DIR, exist_ok=True)
    for name, (content_hash, irises, white_iris) in EYES.items():
        request = urllib.request.Request(CONTENT_URL + content_hash, headers={"User-Agent": "Mozilla/5.0"})
        with urllib.request.urlopen(request) as response:
            texture = Image.open(io.BytesIO(response.read()))
        path = os.path.join(OUT_DIR, f"{name}_mask.png")
        make_mask(texture, irises, white_iris).save(path, optimize=True)
        print(f"{name} -> {os.path.relpath(path)}")


if __name__ == "__main__":
    main()
