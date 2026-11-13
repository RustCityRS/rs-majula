---
name: testing-647
description: Test rev-647 content and engine changes - red-then-green rs-test cases on the shared headless engine, the full gate list before a push, and checks in the real client. Use when adding or fixing a test, before committing, or when a suite fails only in a full run.
---

# Testing at 647

## A headless case (rs-engine/rs-test/tests/<area>.rs)

1. **Take turns.** Each case takes a static `Mutex` (`TURN`) first, so cases in one suite take
   turns on the shared engine.
2. **Get the engine.** Use `runner::on_loaded_engine(|engine| { ... })`, then
   `engine.harness_reset()`.
3. **Set the scene:**
   - log in with `engine.harness_login(...)`;
   - set vars and stats on `engine.harness_player_mut(pid)`;
   - give items with the inventory's `set(slot, obj, count)`;
   - move with `engine.harness_telejump(pid, x, level, z)`, onto a walkable tile.
4. **Act as the client:** send the real packet (`client.send(ClientProt::OpLoc1, ...)` etc.) and
   step with `engine.cycle()`.
5. **Assert what 2011 shows**: the exact line (`HarnessClient::game_messages(&client.packets())`),
   the item, the var, the loc. Assert `engine.script_errors.take().is_empty()` too.
6. **Run it red, then green:** `REV=647 cargo test -q -p rs-test --test <area>`.

## Deterministic, always

- **Test the trigger, not the dice.** Set life points and defence so the first hit lands; protect
  the player if dying isn't the point.
- **Suspect leaked state.** A case that passes alone but fails in the full run is picking up state
  from an earlier case: vars, timers, who is fighting whom. Fix the leak in the harness or the
  test, never with a retry.
- **Prove it holds.** Run the suite three times in a row before calling a fix done.

## Gates before every push

```sh
cargo fmt --all --check
REV=647 cargo clippy --workspace --all-targets -- -D warnings -A clippy::nonminimal_bool -A clippy::vec_init_then_push
REV=289 cargo check --workspace && REV=225 cargo check --workspace
REV=289 cargo test -q -p rs-protocol
runec lint --source content/647/scripts --pack content/647/pack
REV=647 cargo test -p rs-pack --release --no-fail-fast -- --include-ignored    # with the CRC gate
REV=647 cargo test -q -p rs-protocol --test wire_647
REV=647 cargo test -q -p rs-test --no-fail-fast
```

- **During a step**, run only the touched suites.
- **Failures on the base.** A failure that also fails on the base commit isn't yours: show it by
  running the same test there, and say so in the PR.
- **CI stops at the first failing binary.** Reproduce with `--no-fail-fast`.
- **The CRC gate's known list is exact.** Remove a group from it once it matches cache 786.
- **Heavy jobs one at a time.** Run builds and servers one at a time on a shared machine.

## In the client

1. Drive it by action (an option on a named npc, loc or item, or a button), not by hovering the
   mouse over the screen.
2. Check the state itself (inventory, open interface, menu, chat), not only a screenshot.
3. Verify it yourself before asking a person to test.
