#!/usr/bin/env python3
"""Pick N diverse real profiles from the latest catalyst profile deployments (issue #3015).

Writes godot/assets/bench/filler_avatars.json, the fixed crowd that `bench-avatars=N` tops the
plaza up with so device runs carry the same avatar load. Greedy pick: each next profile is the
most different from those already picked: new wearables and collections, skin and hair
colour distance, both body shapes balanced. Looks are deduplicated like profile-images does.

Usage: python3 scripts/bench/fetch_filler_avatars.py [--count 30] [--scan 600]
"""
import argparse
import json
import urllib.request

CATALYST = "https://peer-ec1.decentraland.org"
OUT = "godot/assets/bench/filler_avatars.json"


def get_json(url, body=None):
    headers = {"User-Agent": "godot-explorer-bench/1.0"}
    if body:
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(url, data=json.dumps(body).encode() if body else None, headers=headers)
    with urllib.request.urlopen(req, timeout=60) as r:
        return json.load(r)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--count", type=int, default=30)
    ap.add_argument("--scan", type=int, default=2000)
    args = ap.parse_args()

    addresses, url = [], f"{CATALYST}/content/pointer-changes?entityType=profile&sortingOrder=DESC&limit=500"
    while url and len(addresses) < args.scan:
        page = get_json(url)
        for d in page["deltas"]:
            a = d["pointers"][0].lower()
            if a not in addresses:
                addresses.append(a)
        nxt = page.get("pagination", {}).get("next")
        url = f"{CATALYST}/content/pointer-changes{nxt}" if nxt else None
    profiles = []
    for i in range(0, len(addresses), 100):
        res = get_json(f"{CATALYST}/lambdas/profiles", {"ids": addresses[i:i + 100]})
        for p in res:
            if p.get("avatars"):
                profiles.append(p["avatars"][0])

    def wearables(p):
        # Collection item URN without the token id, so two copies of one item count once.
        return {":".join(w.lower().split(":")[:7]) for w in (p["avatar"].get("wearables") or [])}

    def non_base(p):
        return {w for w in wearables(p) if "base-avatars" not in w}

    def collections(p):
        return {":".join(w.split(":")[:5]) for w in non_base(p)}

    def color(p, key):
        c = (p["avatar"].get(key) or {}).get("color") or {}
        return (c.get("r", 0.0), c.get("g", 0.0), c.get("b", 0.0))

    def canonical(p):
        # Same visual key as decentraland/profile-images avatar-comparison.ts.
        a = p["avatar"]
        cols = tuple(round(x, 4) for k in ("eyes", "hair", "skin") for x in color(p, k))
        return (a.get("bodyShape", "").lower(), tuple(sorted(wearables(p))),
                tuple(sorted(f.lower() for f in (a.get("forceRender") or []))), cols)

    def dist(c1, c2):
        return sum((x - y) ** 2 for x, y in zip(c1, c2)) ** 0.5

    unique, keys = [], set()
    for p in profiles:
        k = canonical(p)
        if k not in keys and len(non_base(p)) >= 3:
            keys.add(k)
            unique.append(p)
    candidates = unique
    seen_w, seen_c, picked, bodies = set(), set(), [], {}
    while candidates and len(picked) < args.count:
        def score(p):
            body = p["avatar"].get("bodyShape", "")
            balance = 0 if bodies.get(body, 0) <= len(picked) / 2 else -6
            skin = min((dist(color(p, "skin"), color(q, "skin")) for q in picked), default=1.0)
            hair = min((dist(color(p, "hair"), color(q, "hair")) for q in picked), default=1.0)
            return (len(non_base(p) - seen_w) + 2 * len(collections(p) - seen_c)
                    + 8 * skin + 6 * hair + balance)
        best = max(candidates, key=score)
        candidates.remove(best)
        picked.append(best)
        seen_w |= non_base(best)
        seen_c |= collections(best)
        body = best["avatar"].get("bodyShape", "")
        bodies[body] = bodies.get(body, 0) + 1
    seen = seen_w
    # Keep only the look; names, addresses and bios of real users don't belong in the repo.
    keep = ("bodyShape", "wearables", "forceRender", "emotes", "eyes", "hair", "skin")
    crowd = [{"name": f"Bench{i + 1:02d}", "hasClaimedName": False, "hasConnectedWeb3": True, "version": 1,
              "avatar": {k: p["avatar"][k] for k in keep if k in p["avatar"]}} for i, p in enumerate(picked)]
    with open(OUT, "w") as fh:
        json.dump(crowd, fh, indent=1)
    print(f"{len(picked)} of {len(unique)} distinct looks ({len(profiles)} profiles): {len(seen)} wearables, {len(seen_c)} collections, bodies {bodies} -> {OUT}")


if __name__ == "__main__":
    main()
