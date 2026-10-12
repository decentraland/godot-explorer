#!/usr/bin/env python3
"""Post a daily PR review report to Slack. Stdlib only.

Each PR is judged on two axes: CODE review (dev reviewers) and QA (the `QAStatus` field of the
MobileTeam project). See `status_text()` for how they combine.

Env: GITHUB_TOKEN, GITHUB_REPOSITORY, SLACK_WEBHOOK_URL, SLACK_USER_MAP, DRY_RUN ("true" = print only),
     PROJECTS_TOKEN (optional, can read org Projects; without it the report says QA is unavailable).
Offline simulation of every combination: `python3 .github/scripts/test_pr_review_report.py --print`.
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
PROJECT_NUMBER = 43  # MobileTeam
QA_FIELD = "QAStatus"
MAX_CHARS = 6000
TITLE_MAX = 70

# Code review states.
APPROVED, CHANGES, PENDING, NONE = "APPROVED", "CHANGES REQUESTED", "PENDING REVIEW", "NO REVIEWER"
# QA states: the QAStatus option names, plus "no status" and "could not be read".
QA_NONE, QA_UNAVAILABLE = None, "unavailable"
QA_PHRASES = {
    "QA Approved": "QA APPROVED",
    "Waiting for QA": "WAITING FOR QA",
    "Changes Needed": "QA REQUESTED CHANGES",
    QA_NONE: "NO QA STATUS",
    QA_UNAVAILABLE: "QA STATUS UNAVAILABLE",
}

G_CHANGES = "CHANGES REQUESTED"
G_DEV = "WAITING FOR DEV REVIEW"
G_OTHER = "WAITING FOR OTHER REVIEW"
G_NONE = "NO REVIEWER"
G_QA = "CODE APPROVED - WAITING FOR QA"
G_READY = "READY TO MERGE - CODE AND QA APPROVED"
GROUPS = [G_CHANGES, G_DEV, G_OTHER, G_NONE, G_QA, G_READY]
TRIM_ORDER = [G_READY, G_QA, G_NONE, G_OTHER]

API = "https://api.github.com"


class QAUnavailable(Exception):
    pass


def request_json(req, what, retries=3):
    """Send a request, retrying rate limits and network blips. Exits non-zero on failure.

    Returns (parsed JSON, Link header). Never prints the URL, only `what`."""
    for attempt in range(retries):
        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                return json.load(resp), resp.headers.get("Link", "")
        except urllib.error.HTTPError as e:
            if e.code in (403, 429, 502, 503) and attempt < retries - 1:
                time.sleep(min(int(e.headers.get("Retry-After") or 30), 60))
                continue
            sys.exit(f"GitHub API error: HTTP {e.code} on {what}")
        except urllib.error.URLError as e:
            if attempt < retries - 1:
                time.sleep(5)
                continue
            sys.exit(f"GitHub API error: {type(e.reason).__name__} on {what}")


def gh_headers(token):
    return {
        "Authorization": f"Bearer {token}",
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
    }


def gh_get(path):
    """GET a REST path, following pagination."""
    url = f"{API}{path}{'&' if '?' in path else '?'}per_page=100"
    items = []
    while url:
        req = urllib.request.Request(url, headers=gh_headers(os.environ["GITHUB_TOKEN"]))
        page, link = request_json(req, f"GET {path}")
        items.extend(page)
        m = re.search(r'<([^>]+)>;\s*rel="next"', link)
        url = m.group(1) if m else None
    return items


QA_QUERY = """
query($owner: String!, $name: String!, $cursor: String) {
  repository(owner: $owner, name: $name) {
    pullRequests(states: OPEN, first: 100, after: $cursor) {
      pageInfo { hasNextPage endCursor }
      nodes {
        number
        projectItems(first: 20) {
          nodes {
            project { number }
            qa: fieldValueByName(name: "%s") { ... on ProjectV2ItemFieldSingleSelectValue { name } }
          }
        }
      }
    }
  }
}
""" % QA_FIELD


def fetch_qa(repo, token):
    """PR number -> QAStatus option name (None when unset). Raises QAUnavailable if unreadable."""
    if not token:
        raise QAUnavailable("PROJECTS_TOKEN is not set")
    owner, name = repo.split("/")
    result, cursor = {}, None
    for _ in range(10):
        body = json.dumps({"query": QA_QUERY, "variables": {"owner": owner, "name": name, "cursor": cursor}})
        req = urllib.request.Request(
            f"{API}/graphql", data=body.encode(), headers=gh_headers(token), method="POST"
        )
        try:
            data, _ = request_json(req, "POST /graphql")
        except SystemExit as e:
            raise QAUnavailable(str(e))
        if data.get("errors"):
            raise QAUnavailable(f"GraphQL error: {data['errors'][0].get('type', 'unknown')}")
        prs = data["data"]["repository"]["pullRequests"]
        for pr in prs["nodes"]:
            qa = None
            for item in pr["projectItems"]["nodes"]:
                if item["project"]["number"] == PROJECT_NUMBER and item.get("qa"):
                    qa = item["qa"]["name"]
            result[pr["number"]] = qa
        if not prs["pageInfo"]["hasNextPage"]:
            return result
        cursor = prs["pageInfo"]["endCursor"]
    raise QAUnavailable("too many pages")


def esc(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def days_since(iso, now):
    return (now - datetime.fromisoformat(iso.replace("Z", "+00:00"))).days


def is_bot(login):
    return login.endswith("[bot]") or login in BOT_REVIEWERS


def latest_reviews(reviews, author):
    """login -> latest APPROVED/CHANGES_REQUESTED state. DISMISSED clears it; COMMENTED/PENDING and
    the author's own reviews are ignored."""
    latest = {}
    for r in sorted(reviews, key=lambda r: r.get("submitted_at") or ""):
        login = (r.get("user") or {}).get("login")
        if not login or login == author:
            continue
        if r["state"] in ("APPROVED", "CHANGES_REQUESTED"):
            latest[login] = r["state"]
        elif r["state"] == "DISMISSED":
            latest.pop(login, None)
    return latest


def review_summary(pr, reviews, qa):
    """Everything the message needs to know about one PR (logins only, no Slack formatting)."""
    author = pr["user"]["login"]
    requested = [u["login"] for u in pr.get("requested_reviewers", [])]
    teams = [f"team:{t['slug']}" for t in pr.get("requested_teams", [])]
    latest = latest_reviews(reviews, author)
    approved = sorted(u for u, s in latest.items() if s == "APPROVED")
    changes = sorted(u for u, s in latest.items() if s == "CHANGES_REQUESTED")
    rereview = [u for u in changes if u in requested]
    pending = [u for u in requested if u not in rereview] + teams
    dev_approved = [u for u in approved if u in DEV_REVIEWERS]

    if changes:
        code = CHANGES
    elif dev_approved:
        code = APPROVED
    elif pending:
        code = PENDING
    else:
        code = NONE

    if code == CHANGES or qa == "Changes Needed":
        group = G_CHANGES
    elif code == PENDING:
        group = G_DEV if any(u in DEV_REVIEWERS for u in requested) else G_OTHER
    elif code == NONE:
        group = G_NONE
    else:
        group = G_READY if qa == "QA Approved" else G_QA

    return {
        "number": pr["number"], "title": pr["title"], "url": pr["html_url"], "author": author,
        "created_at": pr["created_at"], "updated_at": pr["updated_at"],
        "approved": approved, "changes": changes, "rereview": rereview, "pending": pending,
        "code": code, "qa": qa, "group": group,
    }


def status_text(code, qa):
    """The one-line verdict combining code review and QA."""
    qa_phrase = QA_PHRASES.get(qa, f"QA {str(qa).upper()}")
    if code == APPROVED:
        return "CODE AND QA APPROVED" if qa == "QA Approved" else f"CODE APPROVED, {qa_phrase}"
    return f"CODE {'NO DEV REVIEWER' if code == NONE else code}, {qa_phrase}"


def build_message(summaries, now, mention, qa_note=""):
    def names(logins, suffix=lambda u: ""):
        return ", ".join(mention(u) + suffix(u) for u in logins)

    def tag(u):
        if is_bot(u):
            return " (bot)"
        return "" if u in DEV_REVIEWERS else " (non-dev)"

    def line(s):
        title = s["title"] if len(s["title"]) <= TITLE_MAX else s["title"][: TITLE_MAX - 1].rstrip() + "…"
        head = (
            f"<{s['url']}|#{s['number']} {esc(title)}> - by {mention(s['author'])} - "
            f"{days_since(s['created_at'], now)}d open, updated {days_since(s['updated_at'], now)}d ago"
        )
        roster = []
        if s["approved"]:
            roster.append(f"APPROVED: {names(s['approved'], tag)}")
        if s["changes"]:
            again = lambda u: " (re-review requested)" if u in s["rereview"] else ""
            roster.append(f"REQUESTED CHANGES: {names(s['changes'], again)}")
        if s["pending"]:
            roster.append(f"PENDING REVIEW: {names(s['pending'])}")
        who = " | ".join(roster) or "no reviewers"
        return f"{head}\n    ↳ *{status_text(s['code'], s['qa'])}* - {who}"

    grouped = {g: [] for g in GROUPS}
    for s in summaries:
        grouped[s["group"]].append(s)
    lines = {g: [line(s) for s in sorted(grouped[g], key=lambda s: s["updated_at"])] for g in GROUPS}
    trimmed = {g: 0 for g in GROUPS}

    def render():
        out = [f"*PR review report* - {now.strftime('%A %d %b')} - {len(summaries)} open PRs"]
        for g in GROUPS:
            if grouped[g]:
                out += ["", f"*{g} ({len(grouped[g])})*"] + lines[g]
                if trimmed[g]:
                    out.append(f"+{trimmed[g]} more")
        if qa_note:
            out += ["", f"_{qa_note}_"]
        return "\n".join(out)

    text = render()
    for g in TRIM_ORDER:  # least actionable first
        while len(text) > MAX_CHARS and lines[g]:
            lines[g].pop()
            trimmed[g] += 1
            text = render()
    return text


def main():
    repo = os.environ["GITHUB_REPOSITORY"]
    dry_run = os.environ.get("DRY_RUN", "false").lower() == "true"
    webhook = os.environ.get("SLACK_WEBHOOK_URL", "")
    if not webhook and not dry_run:
        sys.exit("SLACK_WEBHOOK_URL secret is missing; cannot post to Slack.")
    try:
        user_map = {k.lower(): v for k, v in json.loads(os.environ.get("SLACK_USER_MAP") or "{}").items()}
    except json.JSONDecodeError:
        sys.exit("SLACK_USER_MAP is not valid JSON.")

    unmapped = set()

    def mention(login):
        if login.lower() in user_map:
            # Dry-run logs are public: show @login instead of the Slack ID.
            return f"@{login}" if dry_run else f"<@{user_map[login.lower()]}>"
        unmapped.add(login)
        return login

    now = datetime.now(timezone.utc)
    prs = [p for p in gh_get(f"/repos/{repo}/pulls?state=open") if not p.get("draft")]
    if not prs:
        text = "*PR review report* - No PRs waiting for review"
    else:
        qa_note = ""
        try:
            qa_by_pr = fetch_qa(repo, os.environ.get("PROJECTS_TOKEN", ""))
        except QAUnavailable as e:
            print(f"::warning::QAStatus unavailable: {e}", file=sys.stderr)
            qa_by_pr = {}
            qa_note = "QAStatus could not be read (check the PROJECTS_READ_TOKEN secret), so QA is not reflected."
        summaries = [
            review_summary(
                p,
                gh_get(f"/repos/{repo}/pulls/{p['number']}/reviews"),
                qa_by_pr.get(p["number"], QA_UNAVAILABLE if qa_note else QA_NONE),
            )
            for p in prs
        ]
        text = build_message(summaries, now, mention, qa_note)

    if unmapped:
        print(f"::warning::No Slack mapping in SLACK_USER_MAP for: {', '.join(sorted(unmapped))}", file=sys.stderr)
    if len(text) > MAX_CHARS:
        print(f"::warning::message is {len(text)} chars after trimming", file=sys.stderr)
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
