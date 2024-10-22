# Changelog

All notable changes to this project will be documented in this file.

## [unreleased]

### Features

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

## [0.1.0-alpha.3] - 2024-10-17

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
