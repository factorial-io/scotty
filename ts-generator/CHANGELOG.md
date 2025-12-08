# Changelog

All notable changes to this project will be documented in this file.

## [0.2.4]

### Bug Fixes

- Remove dates from changelog template to avoid timestamp issues ✔️

## [0.2.3]

### Bug Fixes

- Resolve changelog generation issues with empty sections and subshell ✔️
- Skip empty version sections in per-crate changelogs ✔️

## [0.2.0]

### Bug Fixes

- Add missing ShellSessionRequest type and generate index.ts ✔️
- Generate index.ts with type guards and re-exports ✔️
- Use absolute path from CARGO_MANIFEST_DIR for Docker compatibility ✔️
- Prevent unwanted bindings directory creation ✔️

### Refactor

- Replace barrel file with inline type guards ✔️
- Optimize build system and eliminate type duplication ✔️

