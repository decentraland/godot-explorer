# Prompt for LLM to Fix Rust Errors

Copy and paste this prompt:

---

Fix all Rust compilation errors in this project using the error parser CLI.

**Workflow:**

1. Run `./parse_errors.py build` to build and parse errors
2. Run `./parse_errors.py fix-plan` to see priorities
3. Pick the file with most errors from the plan
4. Run `./parse_errors.py file <filename> -e -d` to see all errors in that file
5. Read the file, fix all errors in it
6. Repeat from step 1 until no errors remain

**Rules:**
- Fix one file at a time, starting with the file that has the most errors
- After fixing a file, always rebuild to verify fixes and catch new errors
- For repeated error patterns (like E0283, E0277), identify the root cause and apply the same fix pattern
- Don't fix warnings until all errors are resolved
- If stuck on an error, run `./parse_errors.py detail <index>` for more context

**Start now by running `./parse_errors.py build`**

---
