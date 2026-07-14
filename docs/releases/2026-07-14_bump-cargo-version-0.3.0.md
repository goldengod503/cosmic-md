# Release: Bump Cargo version to 0.3.0

**Date:** 2026-07-14
**Branch:** `main`
**Follows:** `v0.3.0` GitHub Release (tag `v0.3.0`, pushed same day)

## Summary

Bumps the crate version from `0.1.0` to `0.3.0` so the package version matches
the published release tag. `Cargo.toml` had never been bumped for the `v0.2.0`
or `v0.3.0` tag pushes, so `cargo-deb` built `.deb` artifacts labelled
`0.1.0` while the GitHub Release (which derives its version from the git tag)
was correct. This closes that mismatch for future releases.

## Scope

**Files Modified**
- `Cargo.toml` — `version = "0.1.0"` → `version = "0.3.0"`
- `Cargo.lock` — `galaxy-md` package entry synced to `0.3.0`

## Behavioral Impact

No runtime behavior change. Affects packaging metadata only: the next
`cargo deb` build will name the artifact `galaxy-md_0.3.0-1_amd64.deb`.
The already-published `v0.3.0` `.deb` (`galaxy-md_0.1.0-1_amd64.deb`) is
unchanged — this fix applies to the next release onward.

## Test Plan

- `cargo test` → 8 passed (unchanged suite; version bump touches no code).

## Docs Updated

- This release doc created.

## Rollback Plan

Single scoped commit. To revert:

```bash
git revert <this-commit-sha>
```

## Open Questions / Decisions

- **Resolved:** the `.deb`-version-vs-release-tag mismatch flagged after the
  `v0.3.0` release. `Cargo.toml` is now the source of the package version and
  matches the tag. Going forward, bump `Cargo.toml` before tagging so both
  stay in sync.
