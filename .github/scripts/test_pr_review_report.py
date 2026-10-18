#!/usr/bin/env python3
"""Tests and offline simulation for pr_review_report.py.

    python3 .github/scripts/test_pr_review_report.py           # run the tests
    python3 .github/scripts/test_pr_review_report.py --print   # print every combination + a sample message
"""
import itertools
import os
import sys
import unittest
from datetime import datetime, timezone

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pr_review_report as r  # noqa: E402

NOW = datetime(2026, 10, 12, 12, 0, tzinfo=timezone.utc)
FAKE_MAP = {"kuruk-mm": "U1", "eibrielinv": "U2", "leanmendoza": "U3", "joanadn": "U4", "rodrigo685": "U5",
            "elzathrusso": "U6", "sdilauro": "U7"}
QA, PM, DESIGN = "JoanaDN", "rodrigo685", "ElzathRusso"
ROLE_LOGIN = {"QA": QA, "PM": PM, "DESIGN": DESIGN}


def mention(login):
    return f"<@{FAKE_MAP[login.lower()]}>" if login.lower() in FAKE_MAP else login


def review(user, state, minute=0):
    return {"user": {"login": user}, "state": state, "submitted_at": f"2026-10-10T10:{minute:02d}:00Z"}


def pr(number=1, author="kuruk-mm", requested=(), teams=()):
    return {
        "number": number, "title": f"PR {number}", "html_url": f"https://example.test/{number}",
        "user": {"login": author}, "created_at": "2026-10-05T10:00:00Z", "updated_at": "2026-10-09T10:00:00Z",
        "requested_reviewers": [{"login": u} for u in requested],
        "requested_teams": [{"slug": t} for t in teams],
    }


# Every shape the code-review side can take: name -> (requested, teams, reviews, expected code state).
CODE_CASES = {
    "nobody assigned": ([], [], [], r.NONE),
    "dev requested": (["EibrielInv"], [], [], r.PENDING),
    "only a team requested": ([], ["rgl-mobile-team"], [], r.PENDING),
    "dev approved": ([], [], [review("EibrielInv", "APPROVED")], r.APPROVED),
    "dev approved, another dev pending": (["leanmendoza"], [], [review("EibrielInv", "APPROVED")], r.APPROVED),
    "only the bot approved": ([], [], [review("regenesis-claw", "APPROVED")], r.NONE),
    "bot approved, dev pending": (["EibrielInv"], [], [review("regenesis-claw", "APPROVED")], r.PENDING),
    "dev requested changes": ([], [], [review("EibrielInv", "CHANGES_REQUESTED")], r.CHANGES),
    "changes + re-review requested": (
        ["EibrielInv"], [], [review("EibrielInv", "CHANGES_REQUESTED")], r.CHANGES),
    "bot requested changes": ([], [], [review("regenesis-claw", "CHANGES_REQUESTED")], r.CHANGES),
    "changes, then same dev approved": (
        [], [], [review("EibrielInv", "CHANGES_REQUESTED", 1), review("EibrielInv", "APPROVED", 2)], r.APPROVED),
    "changes by one dev, approved by another": (
        [], [], [review("EibrielInv", "CHANGES_REQUESTED"), review("leanmendoza", "APPROVED")], r.CHANGES),
    "approval dismissed": (
        [], [], [review("EibrielInv", "APPROVED", 1), review("EibrielInv", "DISMISSED", 2)], r.NONE),
    "only comments": ([], [], [review("EibrielInv", "COMMENTED")], r.NONE),
    "author approving own PR": ([], [], [review("kuruk-mm", "APPROVED")], r.NONE),
}
ROLE_STATES = [None, r.R_APPROVED, r.R_PENDING, r.R_CHANGES]  # None = role not involved


def build(code_case, roles=None, number=1):
    """Build (pr, reviews) for a code situation plus {role: state}."""
    requested, teams, reviews = CODE_CASES[code_case][:3]
    requested, reviews = list(requested), list(reviews)
    for role, state in (roles or {}).items():
        login = ROLE_LOGIN[role]
        if state == r.R_APPROVED:
            reviews.append(review(login, "APPROVED"))
        elif state == r.R_CHANGES:
            reviews.append(review(login, "CHANGES_REQUESTED"))
        elif state == r.R_PENDING:
            requested.append(login)
    return pr(number, requested=requested, teams=teams), reviews


def summarize(code_case, roles=None):
    return r.review_summary(*build(code_case, roles))


def all_role_combos():
    for states in itertools.product(ROLE_STATES, repeat=3):
        yield {role: st for role, st in zip(r.ROLE_ORDER, states) if st}


class Tests(unittest.TestCase):
    def test_code_states(self):
        for name, case in CODE_CASES.items():
            self.assertEqual(summarize(name)["code"], case[3], name)

    def test_role_states(self):
        s = summarize("nobody assigned", {"QA": r.R_APPROVED, "PM": r.R_PENDING, "DESIGN": r.R_CHANGES})
        self.assertEqual(s["roles"], {"QA": r.R_APPROVED, "PM": r.R_PENDING, "DESIGN": r.R_CHANGES})

    def test_role_not_shown_unless_involved(self):
        self.assertEqual(summarize("dev approved")["roles"], {})
        self.assertEqual(r.status_text(r.APPROVED, {}), "CODE APPROVED")

    def test_role_people_never_count_as_code_review(self):
        for role in r.ROLE_ORDER:
            self.assertEqual(summarize("nobody assigned", {role: r.R_APPROVED})["code"], r.NONE)
            self.assertEqual(summarize("nobody assigned", {role: r.R_PENDING})["code"], r.NONE)
            self.assertEqual(summarize("dev approved", {role: r.R_CHANGES})["code"], r.APPROVED)

    def test_status_text(self):
        t = r.status_text
        self.assertEqual(t(r.APPROVED, {"QA": r.R_APPROVED}), "CODE APPROVED, QA APPROVED")
        self.assertEqual(t(r.APPROVED, {"QA": r.R_PENDING}), "CODE APPROVED, PENDING FOR QA REVIEW")
        self.assertEqual(t(r.APPROVED, {"QA": r.R_CHANGES}), "CODE APPROVED, QA REQUESTED CHANGES")
        self.assertEqual(
            t(r.PENDING, {"QA": r.R_APPROVED, "PM": r.R_APPROVED, "DESIGN": r.R_PENDING}),
            "PENDING FOR CODE REVIEW, QA APPROVED, PM APPROVED, PENDING FOR DESIGN REVIEW")

    def test_groups(self):
        g = lambda case, roles=None: summarize(case, roles)["group"]
        self.assertEqual(g("dev approved"), r.G_READY)
        self.assertEqual(g("dev approved", {"QA": r.R_APPROVED, "PM": r.R_APPROVED}), r.G_READY)
        self.assertEqual(g("dev approved", {"QA": r.R_PENDING}), r.G_ROLES)
        self.assertEqual(g("dev approved", {"DESIGN": r.R_CHANGES}), r.G_CHANGES)
        self.assertEqual(g("dev requested", {"QA": r.R_APPROVED}), r.G_DEV)
        self.assertEqual(g("only a team requested"), r.G_OTHER)
        self.assertEqual(g("nobody assigned"), r.G_NONE)
        self.assertEqual(g("nobody assigned", {"QA": r.R_PENDING}), r.G_NONE)
        self.assertEqual(g("dev requested changes", {"QA": r.R_APPROVED}), r.G_CHANGES)

    def test_every_combination_resolves(self):
        for case in CODE_CASES:
            for roles in all_role_combos():
                s = summarize(case, roles)
                self.assertIn(s["group"], r.GROUPS)
                self.assertTrue(r.status_text(s["code"], s["roles"]).startswith(("CODE", "PENDING", "NO DEV")))
                # a role is shown only when that person approved, requested changes or was requested
                self.assertEqual(set(s["roles"]), set(roles))

    def test_rerequested_role_is_pending_and_not_listed_as_approved(self):
        pull = pr(requested=[QA])
        s = r.review_summary(pull, [review("EibrielInv", "APPROVED"), review(QA, "APPROVED")])
        self.assertEqual(s["roles"], {"QA": r.R_PENDING})
        self.assertNotIn(QA, s["approved"])

    def test_roster(self):
        s = summarize("changes + re-review requested")
        self.assertEqual((s["changes"], s["rereview"], s["pending"]), (["EibrielInv"], ["EibrielInv"], []))
        s = summarize("dev approved, another dev pending")
        self.assertEqual((s["approved"], s["pending"]), (["EibrielInv"], ["leanmendoza"]))

    def test_message_tags_everyone_and_names_each_role(self):
        pull = pr(7, author="EibrielInv", requested=["leanmendoza", DESIGN])
        sums = [r.review_summary(pull, [review(QA, "APPROVED"), review(PM, "CHANGES_REQUESTED")])]
        text = r.build_message(sums, NOW, mention)
        for tagged in ("<@U2>", "<@U3>", "<@U4>", "<@U5>", "<@U6>"):
            self.assertIn(tagged, text)
        self.assertIn("APPROVED: <@U4> (QA)", text)
        self.assertIn("REQUESTED CHANGES: <@U5> (PM)", text)
        self.assertIn("PENDING REVIEW: <@U3>, <@U6> (DESIGN)", text)
        self.assertIn("*PENDING FOR CODE REVIEW, QA APPROVED, PM REQUESTED CHANGES, PENDING FOR DESIGN REVIEW*", text)

    def test_unmapped_login_is_plain_and_case_insensitive(self):
        self.assertEqual(mention("EIBRIELINV"), "<@U2>")
        self.assertEqual(mention("dependabot[bot]"), "dependabot[bot]")

    def test_trim_drops_least_actionable_first(self):
        sums = [r.review_summary(pr(i, requested=["EibrielInv"]), []) for i in range(1, 40)]
        sums += [r.review_summary(pr(100 + i), [review("EibrielInv", "APPROVED")]) for i in range(30)]
        text = r.build_message(sums, NOW, mention)
        self.assertLessEqual(len(text), r.MAX_CHARS)
        self.assertIn("more", text)
        self.assertIn("WAITING FOR DEV REVIEW (39)", text)


def print_simulation():
    shown = [{}, {"QA": r.R_APPROVED}, {"QA": r.R_PENDING}, {"QA": r.R_CHANGES},
             {"QA": r.R_APPROVED, "PM": r.R_APPROVED, "DESIGN": r.R_APPROVED},
             {"QA": r.R_APPROVED, "PM": r.R_PENDING}, {"PM": r.R_CHANGES, "DESIGN": r.R_PENDING}]
    print(f"{'CODE REVIEW SITUATION':42} {'GROUP':46} VERDICT")
    for case in CODE_CASES:
        for roles in shown:
            s = summarize(case, roles)
            print(f"{case:42} {s['group']:46} {r.status_text(s['code'], s['roles'])}")
        print()
    print("=" * 100, "\nSAMPLE MESSAGE (one PR per code situation, roles cycling)\n")
    sums = []
    for i, case in enumerate(CODE_CASES):
        p, reviews = build(case, shown[i % len(shown)], 1000 + i)
        sums.append(r.review_summary(p | {"title": case}, reviews))
    print(r.build_message(sums, NOW, lambda u: "@" + u if u.lower() in FAKE_MAP else u))


if __name__ == "__main__":
    if "--print" in sys.argv:
        print_simulation()
    else:
        unittest.main()
