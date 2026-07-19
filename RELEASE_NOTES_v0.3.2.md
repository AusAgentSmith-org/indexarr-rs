# Indexarr v0.3.2

## Summary

Indexarr v0.3.2 improves *arr integration and makes the Windows installer
safe for machines where the default web port is already occupied.

## Prowlarr / Torznab

- Added compatibility for Prowlarr's conventional `/api` suffix.
- Updated the web UI, FAQ, and API documentation to configure Prowlarr with
  the base URL and `/api/torznab` as separate fields.
- Added regression coverage for both the documented and compatibility paths.

## Windows installer

- Added an installation step to choose the Indexarr HTTP port, defaulting to
  `8080` and validating ports from `1` through `65535`.
- Persists the selected port as `INDEXARR_PORT`.
- Registers or updates the Windows service with the selected `--port` value so
  upgrades do not retain an old port.
- Opens the selected local URL from the installer Finish page.
- Writes the installer environment without a UTF-8 BOM for reliable service
  startup.

## Validation

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- Vue type-check and production build
- Windows x86_64 cross-build and NSIS packaging

