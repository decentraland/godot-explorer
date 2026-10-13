#!/usr/bin/env python3
"""Post a daily PR review report to Slack. Stdlib only.

Env: GITHUB_TOKEN, GITHUB_REPOSITORY, SLACK_WEBHOOK_URL, SLACK_USER_MAP, DRY_RUN ("true" = print only).
"""
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone

# Keep in sync with the dev team in .github/CODEOWNERS.
DEV_REVIEWERS = {"EibrielInv", "manuelmaceira", "leanmendoza", "kuruk-mm", "sdilauro"}
BOT_REVIEWERS = {"regenesis-claw"}
MAX_CHARS = 3500
TITLE_MAX = 70
TRIM_ORDER = ["APPROVED", "NO REVIEWER", "WAITING FOR OTHER REVIEW"]
GROUPS = [
    "CHANGES REQUESTED",
    "WAITING FOR DEV REVIEW",
    "WAITING FOR OTHER REVIEW",
    "NO REVIEWER",
    "APPROVED",
]

API = "https://api.github.com"


def gh_get(path):
    """GET a GitHub API path, following pagination. Exits non-zero on failure."""
    url = f"{API}{path}{'&' if '?' in path else '?'}per_page=100"
    items = []
    while url:
        req = urllib.request.Request(
            url,
            headers={
                "Authorization": f"Bearer {os.environ['GITHUB_TOKEN']}",
                "Accept": "application/vnd.github+json",
                "X-GitHub-Api-Version": "2022-11-28",
            },
        )
        for attempt in range(3):
            try:
                with urllib.request.urlopen(req, timeout=30) as resp:
                    items.extend(json.load(resp))
                    link = resp.headers.get("Link", "")
                break
            except urllib.error.HTTPError as e:
                if e.code in (403, 429) and attempt < 2:
                    time.sleep(min(int(e.headers.get("Retry-After") or 30), 60))
                    continue
                sys.exit(f"GitHub API error: HTTP {e.code} on GET {path}")
            except urllib.error.URLError as e:
                if attempt < 2:
                    time.sleep(5)
                    continue
                sys.exit(f"GitHub API error: {type(e.reason).__name__} on GET {path}")
        m = re.search(r'<([^>]+)>;\s*rel="next"', link)
        url = m.group(1) if m else None
    return items


def is_bot(login):
    """Bot approvals never satisfy CODEOWNERS review, so they must not mark a PR as approved."""
    return login.endswith("[bot]") or login in BOT_REVIEWERS


def esc(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def days_since(iso, now):
    return (now - datetime.fromisoformat(iso.replace("Z", "+00:00"))).days


def latest_reviews(reviews):
    """login -> latest APPROVED/CHANGES_REQUESTED state. DISMISSED clears it; others are ignored."""
    latest = {}
    for r in sorted(reviews, key=lambda r: r.get("submitted_at") or ""):
        login = (r.get("user") or {}).get("login")
        if not login:
            continue
        if r["state"] in ("APPROVED", "CHANGES_REQUESTED"):
            latest[login] = r["state"]
        elif r["state"] == "DISMISSED":
            latest.pop(login, None)
    return latest


def classify(pr, reviews):
    """Return (group, kind, people) where people are GitHub logins/team slugs."""
    requested = [u["login"] for u in pr.get("requested_reviewers", [])]
    teams = [t["slug"] for t in pr.get("requested_teams", [])]
    latest = latest_reviews(reviews)
    changes = [u for u, s in latest.items() if s == "CHANGES_REQUESTED"]
    approvals = [u for u, s in latest.items() if s == "APPROVED" and not is_bot(u)]

    if changes:
        again = [u for u in changes if u in requested]
        return "CHANGES REQUESTED", "re-review" if again else "", again or changes
    dev = [u for u in requested if u in DEV_REVIEWERS]
    if dev:
        return "WAITING FOR DEV REVIEW", "", dev
    if requested or teams:
        return "WAITING FOR OTHER REVIEW", "", requested + [f"team:{t}" for t in teams]
    if approvals:
        return "APPROVED", "", approvals
    return "NO REVIEWER", "", []


def main():
    repo = os.environ["GITHUB_REPOSITORY"]
    dry_run = os.environ.get("DRY_RUN", "false").lower() == "true"
    webhook = os.environ.get("SLACK_WEBHOOK_URL", "")
    if not webhook and not dry_run:
        sys.exit("SLACK_WEBHOOK_URL secret is missing; cannot post to Slack.")
    try:
        user_map = json.loads(os.environ.get("SLACK_USER_MAP") or "{}")
    except json.JSONDecodeError:
        sys.exit("SLACK_USER_MAP is not valid JSON.")

    def mention(login):
        # Dry-run logs are public: keep Slack IDs out of them.
        return f"<@{user_map[login]}>" if login in user_map and not dry_run else login

    now = datetime.now(timezone.utc)
    prs = [p for p in gh_get(f"/repos/{repo}/pulls?state=open") if not p.get("draft")]

    if not prs:
        text = "*PR review report* - No PRs waiting for review"
    else:
        grouped = {g: [] for g in GROUPS}
        for pr in prs:
            n = pr["number"]
            group, kind, people = classify(pr, gh_get(f"/repos/{repo}/pulls/{n}/reviews"))
            author = pr["user"]["login"]
            title = pr["title"]
            if len(title) > TITLE_MAX:
                title = title[: TITLE_MAX - 1].rstrip() + "…"
            head = (
                f"<{pr['html_url']}|#{n} {esc(title)}> - by "
                f"{mention(author) if group == 'CHANGES REQUESTED' else author} - "
                f"{days_since(pr['created_at'], now)}d open, updated {days_since(pr['updated_at'], now)}d ago"
            )
            if group == "CHANGES REQUESTED":
                who = ", ".join(people)
                head += f" - {'RE-REVIEW REQUESTED from' if kind else 'changes requested by'} {who}"
            elif group == "WAITING FOR DEV REVIEW":
                head += f" - {', '.join(mention(u) for u in people)}"
            elif people:
                label = "approved by" if group == "APPROVED" else "waiting on"
                head += f" - {label} {', '.join(people)}"
            grouped[group].append((pr["updated_at"], head))

        lines_by_group = {}
        for g in GROUPS:
            lines_by_group[g] = [line for _, line in sorted(grouped[g])]

        header = (
            f"*PR review report* - {now.strftime('%A %d %b')} - {len(prs)} open PRs"
        )

        trimmed = {g: 0 for g in GROUPS}

        def render():
            out = [header]
            for g in GROUPS:
                if lines_by_group[g]:
                    out += ["", f"*{g} ({len(grouped[g])})*"] + lines_by_group[g]
                    if trimmed[g]:
                        out.append(f"+{trimmed[g]} more")
            return "\n".join(out)

        text = render()
        # Over budget: trim the least actionable groups first.
        for g in TRIM_ORDER:
            while len(text) > MAX_CHARS and lines_by_group[g]:
                lines_by_group[g].pop()
                trimmed[g] += 1
                text = render()
        if len(text) > MAX_CHARS:
            print(f"warning: message is {len(text)} chars after trimming", file=sys.stderr)

    if dry_run:
        print(text)
        return

    req = urllib.request.Request(
        webhook,
        data=json.dumps({"text": text}).encode(),
        headers={"Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            status = resp.status
    except urllib.error.HTTPError as e:
        sys.exit(f"Slack webhook error: HTTP {e.code}")
    except urllib.error.URLError as e:
        sys.exit(f"Slack webhook error: {type(e.reason).__name__}")
    if status != 200:
        sys.exit(f"Slack webhook error: HTTP {status}")
    print(f"Posted report for {len(prs)} PRs to Slack.")


if __name__ == "__main__":
    main()
