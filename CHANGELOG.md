# Changelog

All notable changes to this project will be documented in this file.

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

### Other (unconventional)

- Merge pull request #1 from factorial-io/dependabot/npm_and_yarn/frontend/rollup-4.24.0

chore(deps-dev): bump rollup from 4.22.0 to 4.24.0 in /frontend ❌
- Merge branch 'renovate/opentelemetry-packages' ❌
- Merge branch 'renovate/traefik-3.x' into 'main'

chore(deps): update traefik docker tag to v3.2

See merge request administration/scotty!11 ❌
- Merge branch 'renovate/dependencies-(patch)' into 'main'

fix(deps): update dependencies (patch)

See merge request administration/scotty!7 ❌
- Merge branch 'renovate/dependencies-(non-major)' into 'main'

fix(deps): update dependencies (non-major)

See merge request administration/scotty!2 ❌
- Merge branch 'first-draft-of-frontend'

# Conflicts:
#	frontend/src/components/start-stop-app-action.svelte
#	frontend/src/lib/index.ts
#	frontend/src/stores/tasksStore.ts
#	frontend/vite.config.ts ❌
- Merge branch 'first-draft-of-frontend' ❌
- Merge branch 'first-draft-of-frontend' ❌
- Rename yafbds to scotty ❌
- Merge branch 'renovate/rust-1.x' into 'main'

chore(deps): update rust docker tag to v1.81

See merge request administration/yafbds!5 ❌
- Savw WIP ❌
- WIP ❌
- Merge branch 'renovate/traefik-3.x' into 'main'

chore(deps): update traefik docker tag to v3.1

See merge request administration/yafbds!3 ❌
- Merge branch 'renovate/configure' into 'main'

Configure Renovate

See merge request administration/yafbds!1 ❌
- Add renovate.json ❌
- Initial commit ❌

<!-- generated by git-cliff -->
