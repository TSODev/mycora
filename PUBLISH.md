# Publishing a release

Checklist for cutting a new crates.io release of `mycora`. Written from
the actual steps used for 0.9.0 through 0.11.0 — not aspirational, this
is what's really been run each time.

## 1. Decide the version number

Follow semver against `CHANGELOG.md`'s `[Unreleased]` section, not gut
feel:

- `[Unreleased]` has an `### Added` entry (a real new feature) →
  **minor** bump (`0.10.x` → `0.11.0`).
- `[Unreleased]` is `### Fixed` only, nothing `### Added`/`### Changed`
  in a user-visible way → **patch** bump (`0.10.0` → `0.10.1`).
- Breaking change to the on-disk vault format, config format, or CLI →
  hasn't happened yet; would need its own discussion when it does.

## 2. Bump the version

- `Cargo.toml`: `version = "x.y.z"`.
- `cargo build` once so `Cargo.lock` picks up the new version too.

## 3. Promote the changelog

In `CHANGELOG.md`:
- Turn `## [Unreleased]` into `## [Unreleased]` (empty, left above) +
  `## [x.y.z] — YYYY-MM-DD` with today's date.
- Every entry that was under `[Unreleased]` moves down under the new
  dated heading, unchanged.

## 4. Verify clean

```sh
cargo build --release
cargo test
cargo clippy
```

All three must be clean — no warnings from clippy, no failing tests.

## 5. Dry-run the package

```sh
cargo package --allow-dirty
cargo publish --dry-run --allow-dirty
```

Check the file count/size looks sane (no accidental `.claude/`,
scratch files, or other unwanted content in the tarball) and that the
dry-run upload reports no registry-side warnings.

## 6. Commit the release

```sh
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "Release x.y.z"
```

Nothing else goes in this commit — feature/fix commits should already
be in place from earlier in the session.

## 7. Publish for real

**Point of no return: crates.io publishes can't be unpublished, only
yanked.** Only run this once told explicitly to go ahead, never as a
default follow-on from step 6.

```sh
cargo publish
```

## 8. Tag and push

```sh
git tag -a vx.y.z -m "Release x.y.z"
git push
git push origin vx.y.z
```

## 9. Announce / gather feedback

Still an open item for this project (see `ROADMAP.md`'s v1.0
checklist) — no fixed process yet for where/how releases get
announced beyond the crates.io listing itself.
