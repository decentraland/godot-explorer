#!/usr/bin/env python3
"""Tests and offline simulation for pr_review_report.py.

    python3 .github/scripts/test_pr_review_report.py           # run the tests
    python3 .github/scripts/test_pr_review_report.py --print   # print every combination + a sample message
"""
import os
import sys
import unittest
from datetime import datetime, timezone

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pr_review_report as r  # noqa: E402

NOW = datetime(2026, 10, 12, 12, 0, tzinfo=timezone.utc)
FAKE_MAP = {"kuruk-mm": "U1", "eibrielinv": "U2", "leanmendoza": "U3", "joanadn": "U4", "sdilauro": "U5"}


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


# Every shape the code-review side can take: name -> (PR kwargs, reviews, expected code state).
CODE_CASES = {
    "nobody assigned": (dict(), [], r.NONE),
    "dev requested": (dict(requested=["EibrielInv"]), [], r.PENDING),
    "only non-dev requested": (dict(requested=["JoanaDN"]), [], r.PENDING),
    "only a team requested": (dict(teams=["rgl-mobile-team"]), [], r.PENDING),
    "dev approved": (dict(), [review("EibrielInv", "APPROVED")], r.APPROVED),
    "dev approved, another dev pending": (
        dict(requested=["leanmendoza"]), [review("EibrielInv", "APPROVED")], r.APPROVED),
    "only a non-dev approved": (dict(), [review("JoanaDN", "APPROVED")], r.NONE),
    "only the bot approved": (dict(), [review("regenesis-claw", "APPROVED")], r.NONE),
    "bot approved, dev pending": (
        dict(requested=["EibrielInv"]), [review("regenesis-claw", "APPROVED")], r.PENDING),
    "dev requested changes": (dict(), [review("EibrielInv", "CHANGES_REQUESTED")], r.CHANGES),
    "changes + re-review requested": (
        dict(requested=["EibrielInv"]), [review("EibrielInv", "CHANGES_REQUESTED")], r.CHANGES),
    "bot requested changes": (dict(), [review("regenesis-claw", "CHANGES_REQUESTED")], r.CHANGES),
    "changes, then same dev approved": (
        dict(), [review("EibrielInv", "CHANGES_REQUESTED", 1), review("EibrielInv", "APPROVED", 2)], r.APPROVED),
    "changes by one dev, approved by another": (
        dict(), [review("EibrielInv", "CHANGES_REQUESTED"), review("leanmendoza", "APPROVED")], r.CHANGES),
    "approval dismissed": (
        dict(), [review("EibrielInv", "APPROVED", 1), review("EibrielInv", "DISMISSED", 2)], r.NONE),
    "only comments": (dict(), [review("EibrielInv", "COMMENTED")], r.NONE),
    "author approving own PR": (dict(), [review("kuruk-mm", "APPROVED")], r.NONE),
}
QA_VALUES = [r.QA_NONE, "Waiting for QA", "Changes Needed", "QA Approved", r.QA_UNAVAILABLE]


def summarize(case, qa):
    kwargs, reviews, _ = CODE_CASES[case]
    return r.review_summary(pr(**kwargs), reviews, qa)


class Tests(unittest.TestCase):
    def test_code_states(self):
        for name, (_, _, expected) in CODE_CASES.items():
            self.assertEqual(summarize(name, None)["code"], expected, name)

    def test_requested_examples(self):
        self.assertEqual(r.status_text(r.APPROVED, "QA Approved"), "CODE AND QA APPROVED")
        self.assertEqual(r.status_text(r.APPROVED, "Waiting for QA"), "CODE APPROVED, WAITING FOR QA")
        self.assertEqual(r.status_text(r.APPROVED, "Changes Needed"), "CODE APPROVED, QA REQUESTED CHANGES")
        self.assertEqual(r.status_text(r.APPROVED, None), "CODE APPROVED, NO QA STATUS")

    def test_every_combination_has_a_group_and_status(self):
        for case in CODE_CASES:
            for qa in QA_VALUES:
                s = summarize(case, qa)
                self.assertIn(s["group"], r.GROUPS)
                self.assertTrue(r.status_text(s["code"], s["qa"]))

    def test_groups(self):
        g = lambda case, qa: summarize(case, qa)["group"]
        self.assertEqual(g("dev approved", "QA Approved"), r.G_READY)
        self.assertEqual(g("dev approved", "Waiting for QA"), r.G_QA)
        self.assertEqual(g("dev approved", None), r.G_QA)
        self.assertEqual(g("dev approved", "Changes Needed"), r.G_CHANGES)
        self.assertEqual(g("dev requested", "QA Approved"), r.G_DEV)
        self.assertEqual(g("dev requested", "Changes Needed"), r.G_CHANGES)
        self.assertEqual(g("only a team requested", None), r.G_OTHER)
        self.assertEqual(g("nobody assigned", None), r.G_NONE)
        self.assertEqual(g("only the bot approved", "QA Approved"), r.G_NONE)
        self.assertEqual(g("dev requested changes", "QA Approved"), r.G_CHANGES)

    def test_roster(self):
        s = summarize("changes + re-review requested", None)
        self.assertEqual((s["changes"], s["rereview"], s["pending"]), (["EibrielInv"], ["EibrielInv"], []))
        s = summarize("dev approved, another dev pending", None)
        self.assertEqual((s["approved"], s["pending"]), (["EibrielInv"], ["leanmendoza"]))

    def test_message_tags_everyone_and_names_each_role(self):
        sums = [
            r.review_summary(
                pr(7, author="EibrielInv", requested=["leanmendoza"]),
                [review("JoanaDN", "APPROVED"), review("sdilauro", "CHANGES_REQUESTED")], "Waiting for QA"),
        ]
        text = r.build_message(sums, NOW, mention)
        for tagged in ("<@U2>", "<@U3>", "<@U4>", "<@U5>"):
            self.assertIn(tagged, text)
        self.assertIn("APPROVED: <@U4> (non-dev)", text)
        self.assertIn("REQUESTED CHANGES: <@U5>", text)
        self.assertIn("PENDING REVIEW: <@U3>", text)

    def test_unmapped_login_is_plain_and_case_insensitive(self):
        self.assertEqual(mention("EIBRIELINV"), "<@U2>")
        self.assertEqual(mention("dependabot[bot]"), "dependabot[bot]")

    def test_trim_drops_least_actionable_first(self):
        sums = [r.review_summary(pr(i, requested=["EibrielInv"]), [], None) for i in range(1, 40)]
        sums += [r.review_summary(pr(100 + i), [review("EibrielInv", "APPROVED")], "QA Approved") for i in range(30)]
        text = r.build_message(sums, NOW, mention)
        self.assertLessEqual(len(text), r.MAX_CHARS)
        self.assertIn("more", text)
        self.assertIn("WAITING FOR DEV REVIEW (39)", text)

    def test_qa_note_is_appended(self):
        text = r.build_message([summarize("dev approved", r.QA_UNAVAILABLE)], NOW, mention, "QA missing")
        self.assertIn("QA STATUS UNAVAILABLE", text)
        self.assertTrue(text.rstrip().endswith("_QA missing_"))


def print_simulation():
    print(f"{'CODE REVIEW SITUATION':44} {'QAStatus':15} {'GROUP':40} VERDICT")
    for case in CODE_CASES:
        for qa in QA_VALUES:
            s = summarize(case, qa)
            print(f"{case:44} {str(qa):15} {s['group']:40} {r.status_text(s['code'], s['qa'])}")
        print()
    print("=" * 100, "\nSAMPLE MESSAGE (one PR per code situation, QA cycling)\n")
    sums = []
    for i, case in enumerate(CODE_CASES):
        kwargs, reviews, _ = CODE_CASES[case]
        sums.append(r.review_summary(pr(1000 + i, **kwargs) | {"title": case}, reviews, QA_VALUES[i % 4]))
    print(r.build_message(sums, NOW, lambda u: "@" + u if u.lower() in FAKE_MAP else u))


if __name__ == "__main__":
    if "--print" in sys.argv:
        print_simulation()
    else:
        unittest.main()
