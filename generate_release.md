# Indexarr release procedure

Indexarr follows a candidate-first release flow. Forgejo is the authoritative
source and build registry; GitHub is the complete public source, artifact, and
container mirror.

## Development track

Every verified `main` push publishes the exact commit to:

- `repo.indexarr.net/indexarr/indexarr-rs:<full-sha>`
- `repo.indexarr.net/indexarr/indexarr-rs:dev`
- `ghcr.io/ausagentsmith-org/indexarr-rs:<full-sha>`
- `ghcr.io/ausagentsmith-org/indexarr-rs:dev`

Ordinary pushes do not move `latest`.

## Release track

1. Update `Cargo.toml`, `Cargo.lock`, and `RELEASE_NOTES_vX.Y.Z.md`.
2. Run formatting, all-target checks, strict Clippy, workspace and documentation
   tests, the UI build, and the vendored-SSL release build.
3. Commit as `release: vX.Y.Z` and push Forgejo `main`.
4. Require the `main` pipeline to pass and confirm GitHub `main` resolves to
   the same commit.
5. Create and push annotated tag `vX.Y.Z` to Forgejo.
6. Require the tag pipeline to build Linux x86_64, Linux aarch64, Windows
   x86_64, and the Windows installer; generate one SHA-256 manifest; build and
   smoke-test both container architectures; and mirror the source tag.
7. Verify the public GitHub release, all six downloadable assets, and both
   architectures of the version and `latest` Docker tags.

Release image tags:

- Forgejo: `vX.Y.Z-amd64`, `vX.Y.Z-arm64`, `vX.Y.Z`, and `latest`
- GHCR: `vX.Y.Z` and `latest`

Mutable tags move only after the immutable candidate passes its runtime smoke
test. Roll back by copying the prior verified digest back to the mutable tag;
never rewrite a published release tag.
