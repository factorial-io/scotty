# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0-alpha.11] - 2024-12-03

### Bug Fixes

- Make 1password config optional in settings-file ✔️
- Update rust crate anyhow to v1.0.94 (#111) ✔️
- Adapt code so it works with new major version of utoipa ✔️
- Update utoipa packages ✔️
- Update rust crate tracing-subscriber to v0.3.19 ✔️
- Update rust dependencies auto-merge (patch) (#100) ✔️
- Update rust dependencies auto-merge (patch) to v0.24.1 (#92) ✔️
- Update opentelemetry packages ✔️
- Update rust crate tabled to 0.17.0 ✔️
- Update rust crate bollard to v0.18.1 (#85) ✔️
- Update rust crate tower-http to v0.6.2 (#83) ✔️
- Update rust crate bcrypt to 0.16.0 ✔️
- Update rust crate serde_json to v1.0.133 (#81) ✔️
- Update rust crate bollard to 0.18.0 ✔️
- Update rust crate axum to v0.7.9 (#78) ✔️
- Update rust crate axum to v0.7.8 (#75) ✔️
- Update rust crate clap to v4.5.21 (#71) ✔️
- Update rust crate serde to v1.0.215 (#68) ✔️
- Update rust crate tokio to v1.41.1 ✔️
- Update opentelemetry packages ✔️
- Update rust crate thiserror to v1.0.69 (#60) ✔️
- Update rust crate anyhow to v1.0.93 ✔️
- Update rust crate thiserror to v1.0.68 ✔️
- Update rust crate thiserror to v1.0.67 ✔️

### Dependencies

- Update rust docker tag to v1.83 ✔️
- Update dependency @sveltejs/kit to v2.9.0 ✔️
- Update dependency eslint-plugin-svelte to v2.46.1 ✔️
- Update dependency eslint to v9.16.0 ✔️
- Update dependency prettier to v3.4.1 ✔️
- Update mariadb docker tag to v10.11 ✔️
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

### Documentation

- Update readme and section about notifications ✔️

### Features

- Implement gitlab MR notifications, smaller code restructuring ✔️
- Implement initial notification service ✔️
- Finish add/remove notification logic in scottyctl and api ✔️
- Implement initial notification service ✔️
- Implement initial notification service ✔️
- Implement initial notification service ✔️
- Implement initial notification service ✔️
- Onepassword integration (#91) ✔️
- 1password-connect integration ✔️
- Create apic-call supports payload up to 50M, configurable via settings. ✔️
- Add option to allow robots for scottyctl create ✔️

## [0.1.0-alpha.10] - 2024-11-02

### Bug Fixes

- Cleanup will also work with unsupported apps ✔️
- Increase default cleanup ttl to 7 days ✔️
- Update rust crate anyhow to v1.0.92 ✔️
- Update rust crate thiserror to v1.0.66 ✔️
- Update rust dependencies auto-merge (patch) ✔️

### Dependencies

- Update dependency @sveltejs/kit to v2.7.4 ✔️
- Update dependency eslint to v9.14.0 ✔️
- Update dependency typescript-eslint to v8.12.1 ✔️
- Update dependency typescript-eslint to v8.12.0 ✔️
- Update dependency daisyui to v4.12.14 (#39) ✔️

### Documentation

- Better help texts ✔️
- Add clarifying comment on how to map the apps folder into the docker-container ✔️

### Features

- Try to get registry from docker metadata for legacy apps and use that when needed ✔️
- Add support for custom domain per service ✔️
- Allow separate blueprint config files in config/blueprints ✔️
- Add ttl-option for scottyctl create ✔️

## [0.1.0-alpha.9] - 2024-10-26

### Bug Fixes

- Update rust crate regex to v1.11.1 ✔️
- Update rust crate config to v0.14.1 ✔️
- Frontend app list did not update on changes, made reactive ✔️
- Update rust dependencies auto-merge (patch) ✔️

### Dependencies

- Update dependency @sveltejs/adapter-auto to v3.3.1 ✔️
- Update dependency typescript-eslint to v8.11.0 ✔️
- Update dependency @sveltejs/adapter-static to v3.0.6 ✔️
- Update dependency @sveltejs/kit to v2.7.3 ✔️
- Update dependency vite to v5.4.10 ✔️

### Features

- Add unsupported status to Apps, prevent running commands against unsupported apps ✔️
- Validate docker-compose for the create task better ✔️
- Expose version via API and CLI for both ctl and server ✔️

## [0.1.0-alpha.8] - 2024-10-22

### CI

- Fix cross compiling for linux, disable linux arm for now ✔️

## [0.1.0-alpha.7] - 2024-10-22

### CI

- Fix cross compiling for linux ✔️

## [0.1.0-alpha.6] - 2024-10-22

### Bug Fixes

- Update rust crate serde to v1.0.211 ✔️
- Update rust crate serde_json to v1.0.132 ✔️
- Update rust crate serde_json to v1.0.131 ✔️
- Update rust dependencies auto-merge (patch) ✔️
- Update rust crate uuid to v1.11.0 ✔️
- Update rust dependencies auto-merge (patch) (#3) ✔️

### CI

- Fix cross compiling for linux ✔️
- Do not run ci actions in parallel ✔️
- Fine-tune docker cleanup ✔️
- Add docker cleanup action, dry-run for now ✔️
- Remove arm64 docker builds again, as they are slow as hell ✔️
- Remove openssl again, as it breaks docker-builds ✔️

### Dependencies

- Update dependency @sveltejs/kit to v2.7.2 ✔️
- Update mariadb docker tag to v11 ✔️
- Update dependency eslint to v9.13.0 ✔️
- Update docker/build-push-action action to v6 ✔️
- Update rust docker tag to v1.82 ✔️
- Update docker/setup-buildx-action action to v3 ✔️
- Update docker/login-action action to v3 ✔️
- Update actions/checkout action to v4 ✔️
- Update dependency typescript-eslint to v8.10.0 ✔️

### Features

- Smaller improvements to the frontend ui ✔️

## [0.1.0-alpha.5] - 2024-10-17

### CI

- Add openssl to dependencies to fix problem with cross-compilation in ci ✔️

### Documentation

- Document how to create a new release ✔️

## [0.1.0-alpha.4] - 2024-10-17

### CI

- Enable changelog for ci changes ✔️

## [0.1.0-alpha.2] - 2024-10-17

### Bug Fixes

- Do not show obsolete (and failing) app-info for destroy command ✔️
- Update opentelemetry packages ✔️
- Update dependencies (patch) ✔️
- Update dependencies (non-major) ✔️
- Fix syntax error ✔️
- Cleanup finished tasks if finished_time > ttl ✔️
- Fix vite config broken because of wrong merge ✔️
- Satisfy typescript, continuing with websocket ✔️
- Work on task detail ✔️
- Show better task results ✔️
- Show better task results ✔️
- Show error messages in the frontend if a task failed ✔️
- Fix a linting error in Dockerfile ✔️
- Make docker availabile inside the image ✔️
- Validate registry for create_app ✔️
- Use '__' as separator in env vars ✔️
- Make sure, that app is using slugified appname ✔️
- Better debug logging ✔️
- Better nginx example ✔️
- No tls related config if disabled ✔️
- Cleanup debugging logs ✔️
- Fix healthcheck ✔️
- Get docker version running ✔️
- Smaller enhancements and fixes ✔️
- Update list of apps after tasks are finished ✔️
- Some fixes in task execution ✔️
- Disable appuser in dockerfile ✔️
- Fix docker permission issue in docker container ✔️
- Fix docker permission issue in docker container ✔️
- Better error reporting for failing docker-compose commands ✔️
- Better error reporting ✔️
- Optimize instrumentation ✔️
- Do not use a semaphore when inspecting apps ✔️
- Better error reporting ✔️
- Add docker-compose to the docker image ✔️
- Add docker-compose to the docker image ✔️

### CI

- Cleanup double action runs ✔️
- Add workflow to build executables for all major platforms ✔️
- Fix tests ✔️
- Deploy latest image to testbed ✔️
- Deploy latest image to testbed ✔️
- Deploy latest image to testbed ✔️
- Deploy latest image to testbed ✔️
- Deploy latest image to testbed ✔️
- Deploy latest image to testbed ✔️
- Deploy latest image to testbed ✔️
- Deploy latest image to testbed ✔️
- Deploy latest image to testbed ✔️
- Fix ci ✔️
- Fix CI ✔️
- Add to ci ✔️
- Add to ci ✔️

### Dependencies

- Update traefik docker tag to v3.2 ✔️
- Update rust docker tag to v1.81 ✔️
- Update rust docker tag to v1.80 ✔️
- Update traefik docker tag to v3.1 ✔️

### Documentation

- Fixing typos ✔️
- Add some more context what the server is actually doing ✔️
- Some typo fixes ✔️
- Add documentation and local development sections to the readme ✔️
- Update readme ✔️
- Update readme ✔️
- Better help texts ✔️

### Features

- Apply public_services onto app-settings when they are empty ✔️
- Ass required_services to App blueprints and validate the struct better ✔️
- Add api route for getting all blueprints ✔️
- App detail page, link tasks to their apps and vice-versa ✔️
- Auto-update frontend from backend via websockets ✔️
- Preliminary task list ✔️
- Dedicated header component ✔️
- Add apps-filter, sort apps alphabetically, fixed some bugs ✔️
- Work on backend to serve frontend statically ✔️
- Implement run/stop functionality ✔️
- Implement login, token-validation and preliminary list of apps ✔️
- Add new routes for login and token-validation, and a general info-route ✔️
- Expose url of running services ✔️
- Start wiht frontend ✔️
- New circle-dot based example ✔️
- Better cleanup of destroyed apps, pull always latest available images ✔️
- Use docker-compose down when destroying an app ✔️
- Implement authentication using bearer_token ✔️
- Implement app blueprints and post actions ✔️
- Add support for private docker registries, validate payload for create_app ✔️
- Add initial support for private docker registries ✔️
- Add destroy command ✔️
- Add support for passing environment variables ✔️
- Implemennt support for haproxy-config, add test-coverage to loadbalancer stuff ✔️
- Add support for disallowing robots ✔️
- Start writing load blanacer config ✔️
- Continuing with rebuild command ✔️
- Refactor all commands to use new state machine, add rebuild command ✔️
- Introduce state machine, rework run command to use sm ✔️
- Add support for long running tasks, still WIP ✔️
- Implement ttl, stop apps automatically aafter a certain time to live ✔️
- Add run, stop and rm commands to yafbdsctl ✔️
- Implement list, run, stop and rm docker-compose based app via API ✔️
- Add first cut of a websocket implementation ✔️

<!-- generated by git-cliff -->
