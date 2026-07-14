# Release: Architectural findings fixes + smooth scrolling

**Date:** 2026-07-14
**Branch merged to main:** `fixes/architectural-findings` (fast-forward)
**Pre-merge base SHA:** `25c2d06` (rollback target)

## Summary

Fixes 13 verified issues from an architectural review of the single-file
markdown viewer — the highest-impact being live-reload dying after atomic
saves and the watcher panicking on inotify exhaustion — and adds app-level
smooth mouse-wheel scrolling. No upstream libcosmic changes were required.
User-facing result: reliable live reload, safe link/scheme handling, a
copy-correct Select Text view, and eased wheel scrolling instead of hard 60px
jumps.

## Scope

**Files Created**
- `docs/releases/2026-07-14_architectural-findings-and-smooth-scroll.md` (this doc)

**Files Modified**
- `src/main.rs` — all 13 fixes + smooth-scroll (overlay + easing loop, first `#[cfg(test)]` module)
- `Cargo.toml` — pinned `libcosmic` to rev `a37be90e811f365273bf632bf7090b63a368f092`
- `Cargo.lock` — `?rev=` pin annotation only (resolved commit unchanged)
- `README.md` — corrected theming claim; synced feature list; dropped stale line count

## Behavioral Impact

Explicit behavior changes (all intended):
- **Live reload** survives atomic saves (write-temp+rename) by watching the parent dir and filtering by path (`src/main.rs`, `FileChanged`/subscription).
- **Watcher failure** degrades to a static viewer + stderr log instead of panicking.
- **Oversized files** (>10 MB) are refused at startup with exit 1.
- **Links** restricted to `http`/`https`/`mailto`; other schemes logged and ignored.
- **Select Text** now interprets the same markdown option set as the formatted view and serializes tables as pipe-separated rows (previously ran cells together / rendered nothing).
- **Reload reads** moved off the update loop (async Task); read failures now logged.
- **Mouse-wheel scrolling** is eased/animated; trackpad pixel scrolling unchanged.
- **CLI** handles `-h`/`--help` and warns on extra args.

Theme behavior unchanged — Tokyo Night remains hard-locked (README corrected to match, not re-enabled to system theming).

## Test Plan

- First tests in the repo: 8 `#[cfg(test)]` unit tests for `markdown_to_plain_text` (table serialization, cell separation, strikethrough, task lists, nested/ordered lists, code blocks, hidden YAML front matter). `cargo test` → 8 passed.
- Live-reload fix verified with a standalone `notify`-8 repro (parent-dir watch catches both mv-over atomic saves; file-inode watch went deaf after the first).
- `cargo build --release` and `cargo clippy --all-targets` clean on the branch.
- Async reload + smooth-scroll verified by launching the real app against `README.md` (survives atomic saves, no panic, clean stderr).

## Docs Updated

- `README.md` — theming line corrected; feature list synced (smooth scroll, live reload, Select Text); stale "~100 lines" claim removed.
- This release doc created.

## Rollback Plan

main fast-forwarded from `25c2d06` to the tip of this branch. To revert the entire batch:

```bash
git revert --no-commit 25c2d06..main   # or:
git reset --hard 25c2d06 && git push --force-with-lease origin main
```

Individual fixes are independently revertible — each is one scoped commit
(see `git log 25c2d06..main`). Smooth scrolling alone: revert `9d64ab0`.

## Open Questions / Decisions

- **Scrolling (Part 2 investigation):** the built-in scrollable's wheel step is a hardcoded 60px instant jump with no interpolation and no builder knob (`scrollable.rs:923-953`). A "native-feel" kinetic fix would need upstream libcosmic/iced changes; instead this ships an app-level overlay+easing approach (transparent `mouse_area` captures the wheel, `window::frames()` drains an accumulator via `scroll_by`). `SCROLL_EASE=0.18` tuned by feel. No open blocker.
- No decision-log entries required — no eligibility/money/rounding/authorization/event-timing logic touched.
