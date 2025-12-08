# Changelog

All notable changes to this project will be documented in this file.

## [0.2.3]

### Bug Fixes

- Resolve changelog generation issues with empty sections and subshell ✔️
- Skip empty version sections in per-crate changelogs ✔️

## [0.2.0]

### Bug Fixes

- Run svelte-kit sync before build ✔️
- Point $generated alias to index.ts file explicitly ✔️
- Add $generated path alias for TypeScript generated files ✔️
- Update Dockerfile to use bun.lock instead of bun.lockb ✔️
- Remove tsconfig exclude to fix CI warning ✔️
- Cleanup frontend task output ✔️
- Resolve custom actions dropdown reactivity issues ✔️
- Resolve WebSocket integration and task output issues ✔️
- Improve bearer token authentication and error logging ✔️
- Resolve frontend linting errors ✔️
- Resolve permission-based action button visibility race condition ✔️
- Fix task activity indicator animation ✔️
- Resolve TypeScript lint errors and improve type safety ✔️

### Dependencies

- Update dependency svelte to v5.43.15 ✔️
- Update npm dependencies auto-merge (patch) (#438) ✔️
- Update dependency typescript-eslint to v8.41.0 ✔️

### Documentation

- Update frontend README with Scotty-specific documentation ✔️

### Features

- Implement container log viewer with navigation improvements ✔️
- Implement real-time task output and WebSocket integration ✔️
- Add permission-based visibility for custom actions ✔️
- Implement comprehensive permission-based UI access control ✔️
- Implement OIDC profile picture support in user avatars ✔️
- Refactor OAuth to OIDC-compliant provider-agnostic system with Gravatar support ✔️
- Implement OAuth session exchange for secure frontend authentication ✔️
- Improve OAuth login flow and authentication validation ✔️
- Implement comprehensive OAuth authentication system ✔️
- Implement OAuth authentication system with hybrid auth modes ✔️

### Refactor

- Replace barrel file with inline type guards ✔️
- Restructure task detail page for consistency ✔️
- Improve log output styling, performance, and controls ✔️
- Fix ESLint errors and improve code quality ✔️
- Embed TaskOutput directly in TaskDetails for tight coupling ✔️
- Optimize build system and eliminate type duplication ✔️
- Centralize session management and eliminate token storage duplication ✔️

## [0.1.0]

### Bug Fixes

- Restore custom actions dropdown functionality and divider visibility ✔️

### Dependencies

- Update dependency typescript-eslint to v8.48.0 ✔️
- Update dependency svelte to v5.43.14 (#578) ✔️
- Update dependency typescript-eslint to v8.47.0 ✔️
- Update dependency @sveltejs/kit to v2.49.0 ✔️
- Update dependency svelte to v5.43.8 (#569) ✔️
- Update dependency daisyui to v5.5.5 (#567) ✔️
- Update dependency svelte to v5.43.7 (#566) ✔️
- Update dependency daisyui to v5.5.4 (#564) ✔️
- Update dependency @sveltejs/kit to v2.48.5 ✔️
- Update npm dependencies auto-merge (patch) ✔️
- Update dependency daisyui to v5.5.0 ✔️
- Update dependency typescript-eslint to v8.46.4 (#558) ✔️
- Update dependency svelte to v5.43.5 (#555) ✔️
- Update dependency eslint to v9.39.1 ✔️
- Update dependency daisyui to v5.4.7 (#554) ✔️
- Update npm dependencies auto-merge (patch) (#553) ✔️
- Update dependency daisyui to v5.4.4 ✔️
- Update npm dependencies auto-merge (patch) (#548) ✔️
- Update dependency @iconify/svelte to v5.1.0 ✔️
- Update dependency daisyui to v5.3.11 (#539) ✔️
- Update dependency globals to v16.5.0 ✔️
- Update dependency eslint-plugin-svelte to v3.13.0 ✔️
- Update dependency @sveltejs/kit to v2.48.4 (#536) ✔️
- Update dependency svelte to v5.43.2 ✔️
- Update dependency @sveltejs/kit to v2.48.3 (#532) ✔️
- Update dependency eslint to v9.38.0 ✔️
- Update dependency svelte to v5.43.0 ✔️
- Update dependency daisyui to v5.3.10 (#530) ✔️
- Update dependency @sveltejs/vite-plugin-svelte to v6.2.1 ✔️
- Update dependency @sveltejs/kit to v2.48.2 ✔️
- Update npm dependencies auto-merge (patch) (#511) ✔️
- Update dependency @sveltejs/kit to v2.47.3 ✔️

## [0.1.0-alpha.38]

### Bug Fixes

- Update dependency @tailwindcss/typography to v0.5.18 ✔️
- Update npm dependencies auto-merge (patch) to v5.0.2 ✔️
- Update dependency @iconify/svelte to v5 ✔️
- Fix UI issues and provide sort handler default ✔️

### Dependencies

- Update dependency svelte to v5.41.1 ✔️
- Update dependency vite to v6.4.1 [security] ✔️
- Update npm dependencies auto-merge (patch) (#504) ✔️
- Update npm dependencies auto-merge (patch) (#496) ✔️
- Update dependency globals to v16.4.0 ✔️
- Update dependency typescript-eslint to v8.46.1 ✔️
- Update dependency daisyui to v5.3.2 (#494) ✔️
- Update dependency svelte to v5.40.0 ✔️
- Update dependency daisyui to v5.3.1 ✔️
- Update npm dependencies auto-merge (patch) (#486) ✔️
- Update dependency @sveltejs/kit to v2.46.5 ✔️
- Update dependency svelte to v5.39.3 ✔️
- Update dependency svelte to v5.39.2 ✔️
- Update dependency daisyui to v5.1.13 (#479) ✔️
- Update dependency typescript-eslint to v8.44.0 ✔️
- Update dependency @sveltejs/kit to v2.39.1 (#472) ✔️
- Update dependency svelte to v5.38.10 (#471) ✔️
- Update dependency @sveltejs/kit to v2.39.0 ✔️
- Update dependency svelte to v5.38.9 (#468) ✔️
- Update dependency @sveltejs/kit to v2.38.1 ✔️
- Update dependency eslint-plugin-svelte to v3.12.3 ✔️
- Update dependency daisyui to v5.1.10 ✔️
- Update dependency svelte to v5.38.8 (#461) ✔️
- Update dependency vite to v6.3.6 (#458) ✔️
- Update dependency eslint-plugin-svelte to v3.12.2 (#457) ✔️
- Update dependency @sveltejs/kit to v2.37.1 (#456) ✔️
- Update dependency svelte to v5.38.7 (#454) ✔️
- Update dependency typescript-eslint to v8.42.0 ✔️
- Update dependency @sveltejs/kit to v2.37.0 ✔️
- Update dependency eslint-plugin-svelte to v3.12.1 (#453) ✔️
- Update npm dependencies auto-merge (patch) to v4.1.13 ✔️
- Update dependency eslint-plugin-svelte to v3.12.0 ✔️
- Update dependency svelte to v5.38.6 ✔️
- Update dependency @sveltejs/vite-plugin-svelte to v6.1.4 (#445) ✔️
- Update npm dependencies auto-merge (patch) (#438) ✔️
- Update dependency typescript-eslint to v8.41.0 ✔️
- Update dependency @sveltejs/kit to v2.36.2 (#434) ✔️
- Update dependency svelte to v5.38.3 (#433) ✔️
- Update dependency @sveltejs/kit to v2.36.1 ✔️
- Update dependency eslint to v9.34.0 ✔️
- Update dependency @sveltejs/kit to v2.36.0 ✔️
- Update dependency @sveltejs/vite-plugin-svelte to v6.1.3 (#425) ✔️
- Update dependency @sveltejs/kit to v2.33.0 ✔️
- Update dependency typescript-eslint to v8.40.0 ✔️
- Update dependency svelte to v5.38.2 ✔️
- Update npm dependencies auto-merge (patch) to v4.1.12 ✔️
- Update dependency @sveltejs/kit to v2.30.1 ✔️
- Update dependency @sveltejs/kit to v2.30.0 ✔️
- Update dependency @sveltejs/kit to v2.29.1 ✔️
- Update frontend dependencies to latest versions ✔️
- Update dependency @sveltejs/kit to v2.28.0 ✔️
- Update dependency typescript-eslint to v8.39.1 (#402) ✔️
- Update dependency typescript to v5.9.2 ✔️
- Update dependency eslint to v9.33.0 ✔️
- Update npm dependencies auto-merge (patch) ✔️
- Update dependency @sveltejs/kit to v2.27.2 ✔️
- Update dependency @sveltejs/kit to v2.27.1 ✔️
- Update dependency typescript-eslint to v8.39.0 ✔️
- Update dependency eslint-plugin-svelte to v3.11.0 ✔️
- Update dependency globals to v16.3.0 ✔️
- Update dependency eslint to v9.32.0 ✔️
- Update dependency daisyui to v5.0.50 (#388) ✔️
- Update dependency svelte-check to v4.3.1 ✔️
- Update dependency daisyui to v5.0.47 (#386) ✔️
- Update dependency @sveltejs/kit to v2.26.1 ✔️
- Update dependency typescript-eslint to v8.38.0 ✔️
- Update dependency eslint-config-prettier to v10.1.8 (#382) ✔️
- Update dependency @sveltejs/kit to v2.24.0 ✔️
- Update dependency typescript-eslint to v8.37.0 ✔️
- Update dependency @sveltejs/kit to v2.23.0 ✔️
- Update dependency eslint to v9.31.0 ✔️
- Update dependency @sveltejs/kit to v2.22.5 (#371) ✔️
- Update dependency eslint to v9.30.1 ✔️
- Update dependency daisyui to v5.0.46 ✔️
- Update dependency @sveltejs/kit to v2.22.4 ✔️
- Update dependency prettier to v3.6.2 ✔️
- Update dependency typescript-eslint to v8.36.0 ✔️
- Update dependency eslint-plugin-svelte to v3.9.3 (#361) ✔️
- Update dependency svelte-check to v4.2.2 (#360) ✔️
- Update npm dependencies auto-merge (patch) (#358) ✔️
- Update dependency eslint to v9.29.0 ✔️
- Update dependency @sveltejs/kit to v2.21.5 (#355) ✔️
- Update dependency postcss to v8.5.5 (#354) ✔️
- Update npm dependencies auto-merge (patch) (#352) ✔️
- Update dependency typescript-eslint to v8.34.0 ✔️
- Update dependency @sveltejs/kit to v2.21.3 (#347) ✔️

### Features

- Upgrade frontend to latest major versions ✔️
- Add Traefik middleware support and examples ✔️
- Add custom actions dropdown component for app blueprints ✔️

### Styling

- Normalize indentation in app.css ✔️
- Reformat confirmation prompt for clarity ✔️

## [0.1.0-alpha.34]

### Dependencies

- Update dependency @sveltejs/kit to v2.21.2 (#345) ✔️

## [0.1.0-alpha.33]

### Dependencies

- Update npm dependencies auto-merge (patch) (#342) ✔️
- Update dependency eslint to v9.28.0 ✔️

## [0.1.0-alpha.32]

### Dependencies

- Update dependency typescript-eslint to v8.33.0 ✔️
- Update dependency daisyui to v5.0.43 ✔️

## [0.1.0-alpha.30]

### Dependencies

- Update npm dependencies auto-merge (patch) (#333) ✔️
- Update dependency daisyui to v5.0.40 (#331) ✔️
- Update dependency daisyui to v5.0.38 (#327) ✔️
- Update dependency eslint-plugin-svelte to v3.9.0 ✔️
- Update dependency globals to v16.2.0 ✔️
- Update dependency daisyui to v5.0.37 (#322) ✔️
- Update dependency svelte to v4.2.20 (#321) ✔️
- Update dependency @sveltejs/kit to v2.21.1 (#320) ✔️
- Update dependency eslint to v9.27.0 ✔️
- Update dependency eslint-plugin-svelte to v3.7.0 ✔️
- Update dependency @sveltejs/kit to v2.21.0 ✔️
- Update dependency svelte-check to v4.2.1 ✔️
- Update dependency prettier-plugin-svelte to v3.4.0 ✔️
- Update dependency eslint-plugin-svelte to v3.6.0 ✔️
- Update npm dependencies auto-merge (patch) (#307) ✔️

## [0.1.0-alpha.29]

### Dependencies

- Update dependency typescript-eslint to v8.32.0 ✔️
- Update dependency globals to v16.1.0 ✔️
- Update dependency eslint-config-prettier to v10.1.5 (#303) ✔️
- Update dependency eslint-config-prettier to v10.1.3 (#300) ✔️
- Update dependency eslint to v9.26.0 ✔️
- Update npm dependencies auto-merge (patch) (#291) ✔️
- Update dependency daisyui to v5 ✔️

## [0.1.0-alpha.28]

### Features

- Add title management for dynamic page titles ✔️
- Add dynamic page titles across key sections ✔️
- Enhance environment-variable display and add defaults ✔️

### Refactor

- Utilize reusable Pill component for status display ✔️

## [0.1.0-alpha.25]

### Dependencies

- Update dependency vite to v5.4.19 [security] (#288) ✔️

## [0.1.0-alpha.24]

### Dependencies

- Update dependency typescript-eslint to v8.31.1 ✔️
- Update dependency eslint to v9.25.1 (#281) ✔️
- Update dependency eslint to v9.25.0 ✔️

### Features

- Display application last checked timestamp ✔️

### Styling

- Improve readability of conditional statement ✔️

## [0.1.0-alpha.22]

### Bug Fixes

- Small syntax fix in svelte-code ✔️
- Fix some linting errors, setup editorconfig ✔️

### Dependencies

- Update npm dependencies auto-merge (patch) (#274) ✔️
- Update dependency typescript-eslint to v8.30.0 ✔️
- Update dependency @sveltejs/kit to v2.20.6 [security] ✔️
- Update dependency svelte-check to v4.1.6 ✔️
- Update dependency eslint-config-prettier to v10.1.2 (#268) ✔️
- Update dependency vite to v5.4.18 ✔️
- Update dependency @sveltejs/kit to v2.20.5 ✔️
- Update dependency @sveltejs/adapter-auto to v6 ✔️
- Update dependency eslint to v9.24.0 ✔️
- Update npm dependencies auto-merge (patch) (#262) ✔️
- Update dependency eslint to v9.23.0 ✔️
- Update dependency typescript-eslint to v8.29.0 ✔️
- Update dependency vite to v5.4.17 ✔️
- Update dependency eslint-plugin-svelte to v3.5.1 ✔️
- Update dependency vite to v5.4.16 [security] ✔️
- Update dependency @sveltejs/kit to v2.20.3 ✔️
- Update dependency vite to v5.4.15 [security] (#245) ✔️
- Update dependency eslint-plugin-svelte to v3.4.0 ✔️
- Update dependency @sveltejs/kit to v2.19.2 ✔️
- Update dependency @sveltejs/kit to v2.19.1 (#238) ✔️
- Update dependency eslint-plugin-svelte to v3.1.0 ✔️
- Update dependency typescript-eslint to v8.26.1 ✔️
- Update dependency autoprefixer to v10.4.21 (#229) ✔️
- Update dependency @sveltejs/kit to v2.19.0 ✔️
- Update dependency eslint to v9.22.0 ✔️
- Update dependency @sveltejs/kit to v2.18.0 ✔️
- Update dependency eslint-config-prettier to v10.1.1 ✔️
- Update dependency svelte-check to v4.1.5 ✔️
- Update dependency eslint-plugin-svelte to v3.0.3 ✔️
- Update dependency typescript-eslint to v8.26.0 ✔️
- Update dependency prettier to v3.5.3 (#218) ✔️
- Update dependency typescript-eslint to v8.25.0 ✔️
- Update dependency eslint to v9.21.0 ✔️
- Update dependency typescript to v5.8.2 ✔️
- Update dependency prettier to v3.5.2 ✔️
- Update npm dependencies auto-merge (patch) (#210) ✔️
- Update dependency globals to v16 ✔️
- Update dependency @sveltejs/vite-plugin-svelte to v5 ✔️
- Update dependency vite to v6 ✔️
- Update dependency eslint to v9.21.0 ✔️
- Update dependency globals to v15.15.0 ✔️
- Update dependency typescript-eslint to v8.25.0 ✔️
- Update dependency typescript to v5.8.2 ✔️
- Update dependency prettier to v3.5.2 ✔️
- Update npm dependencies auto-merge (patch) (#195) ✔️
- Update dependency eslint-config-prettier to v10 ✔️

## [0.1.0-alpha.21]

### Dependencies

- Update dependency tailwindcss to v4 ✔️
- Bump nanoid from 3.3.7 to 3.3.8 in /frontend ✔️
- Update dependency @sveltejs/adapter-auto to v4 ✔️
- Update dependency @sveltejs/kit to v2.17.1 ✔️

## [0.1.0-alpha.20]

### Bug Fixes

- Update dependency @iconify/svelte to v4.2.0 ✔️
- Update npm dependencies auto-merge (patch) ✔️

### Dependencies

- Update dependency typescript-eslint to v8.21.0 ✔️
- Update dependency postcss to v8.5.1 ✔️
- Update dependency vite to v5.4.12 [security] ✔️
- Update npm dependencies auto-merge (patch) ✔️
- Update dependency eslint to v9.18.0 ✔️
- Update dependency typescript to v5.7.3 (#165) ✔️
- Update dependency typescript-eslint to v8.19.1 ✔️

## [0.1.0-alpha.16]

### Dependencies

- Update dependency typescript-eslint to v8.19.0 (#150) ✔️

## [0.1.0-alpha.15]

### Dependencies

- Update dependency @sveltejs/kit to v2.15.1 ✔️

## [0.1.0-alpha.14]

### Dependencies

- Update dependency daisyui to v4.12.23 (#149) ✔️
- Update dependency typescript-eslint to v8.18.2 (#148) ✔️
- Update dependency @sveltejs/kit to v2.14.1 (#145) ✔️
- Update dependency @sveltejs/adapter-static to v3.0.8 (#143) ✔️
- Update dependency @sveltejs/kit to v2.13.0 ✔️
- Update dependency @sveltejs/kit to v2.12.2 ✔️
- Update dependency globals to v15.14.0 ✔️
- Update dependency tailwindcss to v3.4.17 (#137) ✔️
- Update dependency typescript-eslint to v8.18.1 (#136) ✔️
- Update dependency eslint to v9.17.0 ✔️

### Documentation

- First version of the documentation ✔️

## [0.1.0-alpha.13]

### Bug Fixes

- Update app detail when needed ✔️
- Use proper type for AppTtl ✔️
- Update dependency @iconify/svelte to v4.1.0 ✔️

### Dependencies

- Update dependency @sveltejs/kit to v2.11.1 ✔️
- Update dependency daisyui to v4.12.22 (#132) ✔️
- Update dependency daisyui to v4.12.21 (#131) ✔️
- Update dependency @sveltejs/kit to v2.10.1 ✔️

### Features

- Show version string in footer ✔️
- Add support for multiple domains and settings in UI ✔️
- Reenable dark theme ✔️

## [0.1.0-alpha.12]

### Bug Fixes

- Fix frontend build ✔️

### Dependencies

- Update dependency globals to v15.13.0 ✔️
- Update dependency typescript-eslint to v8.18.0 ✔️
- Update dependency @sveltejs/kit to v2.9.1 ✔️
- Update dependency daisyui to v4.12.20 (#117) ✔️
- Update dependency prettier to v3.4.2 (#113) ✔️
- Update npm dependencies auto-merge (patch) (#110) ✔️

## [0.1.0-alpha.11]

### Dependencies

- Update dependency @sveltejs/kit to v2.9.0 ✔️
- Update dependency eslint-plugin-svelte to v2.46.1 ✔️
- Update dependency eslint to v9.16.0 ✔️
- Update dependency prettier to v3.4.1 ✔️
- Update dependency @sveltejs/kit to v2.8.5 (#99) ✔️
- Update dependency @sveltejs/kit to v2.8.4 (#95) ✔️
- Update dependency typescript-eslint to v8.16.0 ✔️
- Update dependency @sveltejs/kit to v2.8.3 ✔️
- Update dependency prettier-plugin-svelte to v3.3.2 ✔️
- Update dependency svelte-check to v4.1.0 ✔️
- Update dependency @sveltejs/kit to v2.8.2 ✔️
- Update dependency typescript to v5.7.2 ✔️
- Update dependency typescript-eslint to v8.15.0 ✔️
- Update dependency eslint to v9.15.0 ✔️
- Update dependency svelte-check to v4.0.9 ✔️
- Bump cross-spawn from 7.0.3 to 7.0.5 in /frontend ✔️
- Update dependency svelte-check to v4.0.8 (#74) ✔️
- Update dependency tailwindcss to v3.4.15 (#73) ✔️
- Update npm dependencies auto-merge (patch) (#69) ✔️
- Update dependency @sveltejs/kit to v2.8.0 ✔️
- Update dependency prettier-plugin-svelte to v3.2.8 ✔️
- Update dependency svelte-check to v4.0.7 ✔️
- Update dependency typescript-eslint to v8.14.0 ✔️
- Update dependency vite to v5.4.11 ✔️
- Update dependency postcss to v8.4.48 ✔️
- Update dependency svelte-check to v4.0.6 ✔️
- Update dependency @sveltejs/kit to v2.7.7 ✔️
- Update dependency @sveltejs/kit to v2.7.6 ✔️
- Update dependency globals to v15.12.0 ✔️
- Update dependency typescript-eslint to v8.13.0 ✔️
- Update dependency @sveltejs/kit to v2.7.5 ✔️
- Update dependency typescript-eslint to v8.12.2 ✔️

### Features

- Onepassword integration (#91) ✔️
- 1password-connect integration ✔️

## [0.1.0-alpha.10]

### Dependencies

- Update dependency @sveltejs/kit to v2.7.4 ✔️
- Update dependency eslint to v9.14.0 ✔️
- Update dependency typescript-eslint to v8.12.1 ✔️
- Update dependency typescript-eslint to v8.12.0 ✔️
- Update dependency daisyui to v4.12.14 (#39) ✔️

## [0.1.0-alpha.9]

### Bug Fixes

- Frontend app list did not update on changes, made reactive ✔️

### Dependencies

- Update dependency @sveltejs/adapter-auto to v3.3.1 ✔️
- Update dependency typescript-eslint to v8.11.0 ✔️
- Update dependency @sveltejs/adapter-static to v3.0.6 ✔️
- Update dependency @sveltejs/kit to v2.7.3 ✔️
- Update dependency vite to v5.4.10 ✔️

### Features

- Add unsupported status to Apps, prevent running commands against unsupported apps ✔️

## [0.1.0-alpha.6]

### Dependencies

- Update dependency @sveltejs/kit to v2.7.2 ✔️
- Update dependency eslint to v9.13.0 ✔️
- Update dependency typescript-eslint to v8.10.0 ✔️

### Features

- Smaller improvements to the frontend ui ✔️

