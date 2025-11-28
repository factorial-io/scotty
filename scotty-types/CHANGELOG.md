# Changelog

All notable changes to this project will be documented in this file.

## [0.2.2] - 2025-11-28

### Bug Fixes

- Skip empty version sections in per-crate changelogs ✔️

## [0.2.0] - 2025-11-24

### Bug Fixes

- Prevent panic on UTF-8 character truncation ✔️
- Resolve merge conflicts from main branch ✔️
- Prevent unwanted bindings directory creation ✔️

### Documentation

- Add readme to scotty-types ✔️

### Features

- Add dedicated OutputStreamType variants for status messages ✔️

### Refactor

- Trim ShellSessionData payload in logs ✔️
- Migrate from REST to WebSocket-only implementation ✔️
- Embed TaskOutput directly in TaskDetails for tight coupling ✔️
- Optimize build system and eliminate type duplication ✔️

### Testing

- Add comprehensive unit tests for shell feature ✔️

