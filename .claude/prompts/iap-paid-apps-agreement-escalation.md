# Prompt: Generate escalation documents for Paid Apps Agreement (Decentraland Foundation)

## How to use this prompt

Paste the entire content below into a fresh Claude session (or any capable AI assistant). The output should be a set of ready-to-send documents — emails, Slack/Notion messages, or a one-page brief — addressed to the right stakeholders inside Decentraland Foundation. The user can then forward each document to the appropriate person.

If the user wants only one specific document (e.g. "just the email to the Account Holder"), they can ask the AI to generate that one. The default is to produce all of them.

---

## Prompt content (copy from here down)

You are helping Leandro Mendoza (engineer at Decentraland Foundation, email `leandro@dclregenesislabs.xyz`) escalate a blocker that's preventing in-app purchases from working on the iOS Decentraland Explorer app. Generate the documents listed in the **Deliverables** section at the end. Each document must be self-contained, professional, and tailored to its audience. Do NOT skip context — these readers may not know what an IAP is or why we care.

### Background context (everything below is fact, do not make things up beyond this)

**Project**

- Repository: `godot-explorer` (Decentraland's iOS/Android/desktop client built with Godot 4.5.1 + Rust)
- iOS app bundle ID: `org.decentraland.godotexplorer`
- App Store Connect listing: "Decentraland", under team **DECENTRALAND FOUNDATION** (Apple Team ID `8T73XM973P`)
- Decentraland Foundation is a Panama-registered legal entity. Address on file in App Store Connect: `C/O Quijano & Asociados, Bloc Office Hub, Fifth Floor, Panama City, Panama` (DUNS / equivalent ID `92397745`).

**What we're building**

The iOS app is going to offer **in-app purchases (IAPs) of credits** — consumable virtual currency that users buy with real money via Apple's StoreKit, and then spend inside Decentraland scenes. The first product registered in App Store Connect is `credits_10` (10 credits), in status "Ready to Submit". More credit packs (e.g. `credits_50`, `credits_100`) will follow.

The technical implementation is complete and working end-to-end:
- A Swift GDExtension (`DclSwiftLib`) bridges Apple's StoreKit 2 framework into Godot.
- A GDScript autoload (`IapManager`) wraps it with a clean API for the rest of the codebase.
- On a real iPhone with a sandbox tester account signed in, the app correctly initializes StoreKit and asks Apple's servers for the product catalog.

**The blocker**

When the app calls `Product.products(for: ["credits_10"])`, Apple's servers return **0 products** — meaning the product is invisible in sandbox testing. After diagnosing systematically (verifying bundle ID match, verifying sandbox is active at the OS level, verifying the App ID has In-App Purchase capability enabled in the Apple Developer portal, verifying product metadata is complete and "Ready to Submit"), the root cause is confirmed at App Store Connect → Business → Agreements:

> **"To offer apps or other in-app purchases, you must sign the Paid Apps Agreement."**

The Agreements table at App Store Connect shows:

| Agreement Type | Status |
|---|---|
| Free Apps Agreement | Active |
| **Paid Apps Agreement** | **New** (i.e. NOT signed) |

Without a signed and active Paid Apps Agreement, **no IAP product can load on any device — sandbox or production**. The catalog is gated at the team level. This is not a bug, not a Decentraland code issue, not an Apple delay — it's an organizational/legal step that nobody at the Foundation has completed yet.

**What signing the Paid Apps Agreement requires (per Apple's standard onboarding)**

1. **Authority to sign**: Only an "Account Holder" or "Admin" with legal signing authority on behalf of Decentraland Foundation can complete this. We need to identify who that is and confirm they're the right person legally.

2. **Banking information**: A bank account in the name of Decentraland Foundation, into which Apple will deposit revenue (after Apple's commission). Apple supports SWIFT/IBAN. Required fields typically include:
   - Bank name and address
   - Account number
   - SWIFT / BIC code (or routing number for US banks)
   - Account holder name (must match the legal entity exactly)
   - Currency (USD strongly recommended for global revenue)

3. **Tax forms**: Because Decentraland Foundation is a non-US entity (Panama), Apple requires a **W-8BEN-E** form (Certificate of Foreign Status of Beneficial Owner). Apple typically generates the form digitally inside App Store Connect — the signer fills it out online. This involves declaring:
   - Legal entity classification (corporation, partnership, foundation, etc.)
   - Country of organization (Panama)
   - Tax residency
   - Treaty benefit claims (Panama-US tax treaty status — there is no comprehensive tax treaty, so default 30% withholding may apply unless other relief is documented)
   - GIIN (Global Intermediary Identification Number) if Decentraland Foundation is registered under FATCA — usually applies to financial institutions, may not apply

   Recommend involving the Foundation's tax advisor (Quijano & Asociados in Panama appears to be on the address record) to confirm the right declarations.

4. **Contacts**: Apple wants four named contacts inside ASC, each with email + phone:
   - **Senior Management**
   - **Financial**
   - **Technical** (Leandro Mendoza is the natural fit)
   - **Legal**

5. **Revenue split awareness**: Standard Apple commission is **30% on the first $1M of revenue per developer per year**, **15% above** under the Small Business Program (which the Foundation likely qualifies for if total Apple revenue is under $1M/year). For consumable IAPs like credits, this commission applies to every transaction. Pricing strategy for `credits_10` and future packs should account for this — Apple takes its cut before depositing.

6. **Compliance / regional tax**: Apple's Paid Apps Agreement also makes the Foundation responsible for collecting/remitting consumption taxes (VAT/GST) in many jurisdictions, OR opting in to Apple's "merchant of record" service where Apple handles it. For mobile IAPs Apple is increasingly the merchant of record, but it depends on the country.

**Timeline impact**

- **Today (May 5, 2026)**: agreement is "New", IAPs blocked.
- **Optimistic**: 24-72 hours after submission to "Active" status — assuming all info is complete and Apple doesn't request revisions. Often Apple replies the same business day.
- **Realistic with iteration**: 1-2 weeks if banking/tax info needs to be gathered, signed, or amended.
- **After "Active"**: an additional 2-24 hours for the sandbox catalog to refresh and start serving products to test devices.

Until the agreement is signed, **all IAP development on iOS happens against a local mock (StoreKit Configuration File)**, not against real ASC products. The team can keep building UI, persistence, and validation paths, but real end-to-end testing on device is gated.

**Project impact**

- Credits monetization is a new revenue line for the Foundation — this is the gating step.
- The engineering investment (Swift GDExtension, native StoreKit 2 integration, GDScript autoload) is already done and proven working at the bridge layer.
- Server-side receipt validation backend (post-purchase verification of Apple's JWS signed transactions) is **not yet built** — that's a separate workstream once the agreement is unblocked. The current code has a `_validate_with_backend` stub that returns true, marked with a TODO. **Real money should not flow until that backend exists**, but signing the Paid Apps Agreement is independent and should not wait for backend work.

### Constraints when generating the documents

- Be specific. Don't write "please complete the necessary forms" — write "please complete the W-8BEN-E form by clicking Edit on the Paid Apps Agreement row at App Store Connect → Business → Agreements".
- Acknowledge that Leandro is engineering, not finance/legal. He can hand off but cannot complete this himself.
- Do NOT recommend specific tax positions (e.g. "claim treaty benefits") — defer to Quijano & Asociados.
- Tone: clear, factual, urgent-but-not-alarmist. The Foundation doesn't lose anything by waiting, but it loses time-to-revenue.
- Length: each document should be readable in under 3 minutes by a busy executive.

### Deliverables

Generate the following documents, each clearly labeled with a heading:

1. **Email to the Account Holder / Admin of the Apple Developer team** — the person who has legal signing authority. Goal: get them to start the Paid Apps Agreement workflow today. Include a numbered checklist of what they'll need to gather before sitting down (banking, tax form, contact assignments). End with an offer to do a 30-min call to walk through the ASC UI.

2. **Email/message to Finance/Treasury** — request the banking details needed (account info, SWIFT, account holder name on file). Explain why it's needed and what happens to the revenue.

3. **Email to Legal/Tax (or to Quijano & Asociados)** — flag the W-8BEN-E form, ask for guidance on tax residency and treaty position for Panama-US. Mention FATCA/GIIN status. Ask whether the Foundation needs to update any tax filings consequently.

4. **One-page Notion/Slack-ready brief** for the Engineering Leadership / Product Manager. Goal: status update with no jargon. Sections: What we're building, What's blocked, What we need from whom, Timeline, Risk if we wait. Markdown formatted.

5. **A short Slack message** (under 100 words) that Leandro can paste in a leadership channel asking who the Account Holder is and tagging the right person.

### Output format

For each deliverable: clearly labeled heading (`## Deliverable 1: Email to Account Holder` etc.), followed by the document with its own subject line / recipient line / body. Markdown allowed inside email bodies for clarity.

Begin generating now.
