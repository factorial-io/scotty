# Changelog

All notable changes to this project will be documented in this file.

## [0.2.3] - 2025-11-28

### Bug Fixes

- Resolve changelog generation issues with empty sections and subshell ✔️
- Skip empty version sections in per-crate changelogs ✔️

## [0.2.0] - 2025-11-24

### Bug Fixes

- Add missing ShellSessionRequest type and generate index.ts ✔️
- Generate index.ts with type guards and re-exports ✔️
- Use absolute path from CARGO_MANIFEST_DIR for Docker compatibility ✔️
- Prevent unwanted bindings directory creation ✔️

### Refactor

- Replace barrel file with inline type guards ✔️
- Optimize build system and eliminate type duplication ✔️
