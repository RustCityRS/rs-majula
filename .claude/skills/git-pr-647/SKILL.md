---
name: git-pr-647
description: Commit and push rev-647 work to feature/647-support (PR #16) - sync, one item per commit, plain messages with sources, gates, and what to do when someone else's push breaks a test. Use before any commit or push on the 647 branch.
---

# Git and PRs for 647

1. **Sync before starting, and again before pushing:**
   ```sh
   git fetch origin
   git rebase origin/feature/647-support
   ```
2. **One item per commit.** One quest, one shop, one fix.
3. **Message:**
   - Subject: `feat: ...`, `fix: ...` or `test: ...`, saying what the game does now.
   - Body: what changed, why 2011 needs it, and the sources (cache ids, wiki page and revision,
     recording id @ second).
   - No co-author or tool attribution lines.
4. **Leave other revisions alone.** Changes for 647 go behind `#[cfg(since_647)]`. When in doubt,
   diff against `origin/main`.
5. **Gates:** run the full list in skill `testing-647`. Config or pack changes need the CRC gate
   too.
6. **Push:** `git push origin <local branch>:feature/647-support`.

## When someone else's push breaks a test

Comment on the PR, tagging the author, with:
- **the cause**: the commit, and why it breaks the test;
- **the fix**: a commit, or what is needed;
- **who owns it**.

Keep it short and factual.

## Never

- Push without the gates.
- Rewrite shared history.
- Commit any of these:
  - scratch files, downloaded videos or derived cache output;
  - absolute paths from your machine;
  - tokens, passwords or other credentials.
