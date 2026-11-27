---
name: git-pr-647
description: Commit and push rev-647 work on its own branch with its own PR into main - branch naming, sync, one item per branch, plain messages with sources, gates, and what to do when someone else's push breaks a test. Use before any commit, push or PR for 647 work.
---

# Git and PRs for 647

Every piece of 647 work goes on **its own branch** with **its own PR into `main`**. There is no
shared 647 branch: `feature/647-support` (PR #16) is merged and gone. Never push to `main`, and
never mix two items on one branch.

1. **Start a branch from `origin/main`:**
   ```sh
   git fetch origin
   git switch -c <type>/647-<short-name> origin/main
   ```
   - `<type>` is `feat` (new content), `fix` (a bug) or `test` (tests only).
   - `<short-name>` is a few words in kebab case: `feat/647-waterfall`,
     `fix/647-task-varps-saved`.
   - If your checkout is busy (another branch, local changes), use a worktree instead:
     `git worktree add -b <branch> <dir> origin/main`.
   - If you already committed the work somewhere else (a local test branch), cherry-pick that
     commit onto the new branch.
2. **One item per branch and PR.** One quest, one shop, one fix. Unrelated changes in your
   working tree (other people's, or your own for another item) stay out: `git add` the files by
   name, never `git add -A`.
3. **Message:**
   - Subject: `feat(647): ...`, `fix(647): ...` or `test(647): ...`, saying what the game does
     now.
   - Body: what changed, why 2011 needs it, and the sources (cache ids, wiki page and revision,
     recording id @ second).
   - No co-author or tool attribution lines.
4. **Leave other revisions alone.** Changes for 647 go behind `#[cfg(since_647)]`. When in doubt,
   diff against `origin/main`.
5. **Gates:** run the full list in skill `testing-647`. Config or pack changes need the CRC gate
   too.
6. **Sync again just before pushing:** `git fetch origin && git rebase origin/main`. Your branch
   is yours alone until someone else commits to it, so a rebase here is fine.
7. **Push and open the PR:**
   ```sh
   git push -u origin <branch>
   gh pr create --base main --head <branch> --title "<the commit subject>" --body "<body>"
   ```
   - The PR body says what the player sees now, how it was tested (the rs-test cases, red then
     green, and any in-client check), and what is left out on purpose.
   - Without `gh`, open the PR from the link `git push` prints.
   - Give the user the PR link.

## When someone else's push breaks a test

Comment on that PR, tagging the author, with:
- **the cause**: the commit, and why it breaks the test;
- **the fix**: a commit, or what is needed;
- **who owns it**.

Keep it short and factual.

## Never

- Push to `main`, or to someone else's branch without asking.
- Push without the gates.
- Rewrite history on a branch someone else has committed to.
- Commit any of these:
  - scratch files, downloaded videos or derived cache output;
  - absolute paths from your machine;
  - tokens, passwords or other credentials.
