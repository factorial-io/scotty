# Changelog

All notable changes to this project will be documented in this file.

## [0.3.1](https://github.com/factorial-io/scotty/compare/v0.3.0...v0.3.1) (2026-06-09)


### Bug Fixes

* show service URL based on per-container status ([5ce6967](https://github.com/factorial-io/scotty/commit/5ce696781430a5f49bdfd9bc5e3aae1813340723))

## [0.3.0](https://github.com/factorial-io/scotty/compare/v0.2.9...v0.3.0) (2026-06-07)


### ⚠ BREAKING CHANGES

* **deps:** requires oauth2 5.x; minimum reqwest is now 0.12.

### Features

* add app:cp file transfer to and from app services ([08c043b](https://github.com/factorial-io/scotty/commit/08c043b7628c8f0163a1c364bc6e5bba4ea6cb60))
* **files:** add server-side file transfer endpoints ([0f7652d](https://github.com/factorial-io/scotty/commit/0f7652dff3bebcf09d541b1c079af79aac86dbc0))
* implement landing page for stopped apps ([f09ed2a](https://github.com/factorial-io/scotty/commit/f09ed2a500a58cfa992f1cf9dc9aa83c44cd48ac))
* **logs:** support logs from stopped containers ([eaf3b1d](https://github.com/factorial-io/scotty/commit/eaf3b1d6c54a8453c1d990aa6050186a18624299))
* per-app custom actions with approval workflow ([a9bb74f](https://github.com/factorial-io/scotty/commit/a9bb74f4b13d0b422d60ce0d3f9e9d2d2cd93851))
* propose app-file-transfer openspec change ([c99ebb5](https://github.com/factorial-io/scotty/commit/c99ebb572c3b7cdcd46ee3800b542c5187169171))
* propose app-file-transfer openspec change ([f9c1e60](https://github.com/factorial-io/scotty/commit/f9c1e60d73a91e9a9bcdb63872118a8974101268))
* **scottyctl:** add app:cp command for file transfer ([1e9747c](https://github.com/factorial-io/scotty/commit/1e9747cb0811edea4e09dbb05b95831352e33191))


### Bug Fixes

* add eslint-disable for external return URL href ([2955e63](https://github.com/factorial-io/scotty/commit/2955e631db75084df28031d03859fd1f34017f66))
* add missing `connect` feature to tokio-tungstenite dependency ([5b3be39](https://github.com/factorial-io/scotty/commit/5b3be39ce48831d2450b9b53135ba2e0c9059e80))
* address fourth-round review on app:cp file transfer ([a051d14](https://github.com/factorial-io/scotty/commit/a051d149d193016adcbd02dec7f510579f9806d7))
* address latest MR review findings for landing page ([06acf53](https://github.com/factorial-io/scotty/commit/06acf5358df5dac0a75e2f30389ffdaf192756c1))
* address PR review findings for landing page ([fad151f](https://github.com/factorial-io/scotty/commit/fad151f0e799b14afecffdfc37baf061d5a2709e))
* address second-round review on app:cp file transfer ([aa64d62](https://github.com/factorial-io/scotty/commit/aa64d624d89b3063ce4864ed917f97af034e6ed1))
* address third-round review on app:cp file transfer ([6be632a](https://github.com/factorial-io/scotty/commit/6be632a28699c5d23d6a272ec6edf7498f5e3450))
* **api:** return proper 404 for missing container paths on file download ([cc94ad7](https://github.com/factorial-io/scotty/commit/cc94ad7a49ffeddb985f0f3bb8f368c3167d3f77))
* **app:cp:** address review feedback on file transfer ([c80c25d](https://github.com/factorial-io/scotty/commit/c80c25df6937854ccae1c25f9ffabdb6a5072780))
* **authz:** serialize policy.yaml with sorted keys to avoid diff churn ([f4bbbe7](https://github.com/factorial-io/scotty/commit/f4bbbe7440f3bc03810978bef90694cbe71b52f6))
* **authz:** stop tracking runtime-mutated policy.yaml, seed from example ([62a499d](https://github.com/factorial-io/scotty/commit/62a499d6f39a14e1a62cb5caca3b60c64fa16fef))
* **ci:** prevent duplicate workflow runs on PR branches ([5eb1dca](https://github.com/factorial-io/scotty/commit/5eb1dca94f4be30cdd8f1d4abe6835e500b806a7))
* **ci:** prevent duplicate workflow runs on PR branches ([c89f3b2](https://github.com/factorial-io/scotty/commit/c89f3b2a0300d5013cb214e774f4713fb068e7c4))
* correct custom action approval authorization and CLI parsing ([f3b9ea8](https://github.com/factorial-io/scotty/commit/f3b9ea880da531b78442ac92f9226df7b51f3071))
* correct documentation errors in CLI docs and AGENTS.md ([70913e1](https://github.com/factorial-io/scotty/commit/70913e131174246ef51debf478d07c01116a3a5e))
* correct documentation errors in CLI docs and AGENTS.md ([e0bc21a](https://github.com/factorial-io/scotty/commit/e0bc21ad0af71564c12e6eb84edacc3a1429f141))
* **deps:** fix sha2 0.11 compatibility ([c228038](https://github.com/factorial-io/scotty/commit/c22803892c4e5e8b684eff307dc9ba757c57ceea))
* **deps:** patch known security vulnerabilities in dependencies ([d7a9496](https://github.com/factorial-io/scotty/commit/d7a949681015da7409bafd30d30686ce0f29f058))
* **deps:** patch known security vulnerabilities in dependencies ([5ddfbf1](https://github.com/factorial-io/scotty/commit/5ddfbf111532e19fc0a22a8ca89fe95e780b7544))
* **deps:** sync Cargo.lock with workspace version and keep it synced on release ([358badf](https://github.com/factorial-io/scotty/commit/358badffefedeacf0ada11faf841b9505e313f0c))
* **deps:** update rust crate sha2 to 0.11 ([8bf7491](https://github.com/factorial-io/scotty/commit/8bf74918d562a57bf3576c69345c4acf8a7ac8bc))
* **deps:** update rust crate sha2 to 0.11 ([1ae1d78](https://github.com/factorial-io/scotty/commit/1ae1d789a514eb7773781fc74c60651f9cdba54c))
* **deps:** update rust crate ts-rs to v12 ([5396ced](https://github.com/factorial-io/scotty/commit/5396ced6a6bd5c7d4656bf4a05cb16d657568c70))
* **deps:** update rust crate ts-rs to v12 ([f6c2e57](https://github.com/factorial-io/scotty/commit/f6c2e576f17b50e45fd266fae921469232ea9a7f))
* **deps:** upgrade oauth2 4.x to 5.x to drop vulnerable reqwest 0.11 chain ([08d3195](https://github.com/factorial-io/scotty/commit/08d31953300554395cfd506ab7ddc50fcd7bbf2a))
* **docs:** consolidate lockfiles on bun and patch dependency advisories ([282abd1](https://github.com/factorial-io/scotty/commit/282abd1b48b396b6c7b0f93d324cda8abb221e09))
* **frontend:** fix ESLint v10 compatibility ([6be5c09](https://github.com/factorial-io/scotty/commit/6be5c0984e1ca600ada2adf8fa53906135c58ee8))
* harden custom action route guards and creation validation ([fc3fdb5](https://github.com/factorial-io/scotty/commit/fc3fdb599359d258fedfb68dbf62bf798d0e5f62))
* move .scottyignore section after app:create argument descriptions ([e8c66e2](https://github.com/factorial-io/scotty/commit/e8c66e23ed7aa1377e158a7f13903845af9aa8d1))
* pass Config to ts-rs export() calls for v12 compatibility ([1850f7f](https://github.com/factorial-io/scotty/commit/1850f7f231e45c34f29d5e73f6bb2337cb68aefb))
* prevent caching of landing page redirect and 404 responses ([1e0e518](https://github.com/factorial-io/scotty/commit/1e0e518340720889e832c4c1f0ae3a3548d48227))
* prevent caching of landing page redirect and 404 responses ([5ff9eab](https://github.com/factorial-io/scotty/commit/5ff9eab6127ee2d123c51382740c878029d2af0d))
* **scottyctl:** app:cp upload translates destination into Docker dir + entry name ([897b442](https://github.com/factorial-io/scotty/commit/897b44276d08d3617969d903df28e381b0d70bd0))
* **tls:** install rustls crypto provider at startup ([75dd751](https://github.com/factorial-io/scotty/commit/75dd7515d405896b7c84c20a1216beab42b4401a))
* update tests for axum-test v19 API and Docker availability ([31f80aa](https://github.com/factorial-io/scotty/commit/31f80aaacdef5df7f7c94346699604613825e1a9))
* upgrade vite to v8, svelte to v5.46.4+, and sveltekit to v2.57.1+ for vite-plugin-svelte v7 compatibility ([26946a5](https://github.com/factorial-io/scotty/commit/26946a52de6f18d0186fa3c7641a8346d5435d6c))
* use bollard_stubs::models for private types in bollard 0.20 ([c38a564](https://github.com/factorial-io/scotty/commit/c38a564f0b2ba0950a760b0d3051858fdc10442f))
* validate action names and tidy custom action endpoints ([a1b5d3b](https://github.com/factorial-io/scotty/commit/a1b5d3b7352f879422dac2a2c8b4b78a5f826d86))


### Refactor

* address second-round review on custom actions ([43560b7](https://github.com/factorial-io/scotty/commit/43560b7d6c124c5789999706943bdc8e8cd3b877))
* configure ts-rs export dir via Config instead of env var ([ffc7a92](https://github.com/factorial-io/scotty/commit/ffc7a92c2b993253eafd5816261ec767a28510d2))
* extract get_domains() on ServicePortMapping to eliminate auto-domain coupling ([40e2551](https://github.com/factorial-io/scotty/commit/40e255124133ff238cc86d8b2c9f41dfbddd2283))
* extract test Docker client creation into shared helper ([8796062](https://github.com/factorial-io/scotty/commit/87960628cf39f52b13fe8d9a06b3d61fc1ffee42))


### Documentation

* add implementation plan for scotty landing page ([fbb89e4](https://github.com/factorial-io/scotty/commit/fbb89e44df1e6e85023e19848302e691473fdaa5))
* expand release-please release guide in README ([dba0a61](https://github.com/factorial-io/scotty/commit/dba0a61c8fdd826e478a60c32f097dac8d7aec07))
* **files:** document app:cp and tidy file-transfer beans ([1e92cd1](https://github.com/factorial-io/scotty/commit/1e92cd1bf95fe58bf04bd6866b4e04d1b5f361c9))
* **openspec:** add stopped-container-logs change proposal ([7c9d7cc](https://github.com/factorial-io/scotty/commit/7c9d7cc8083557feea92ffe1e6f04342b6563a71))
* redesign landing page with Start-button-first UX ([0357685](https://github.com/factorial-io/scotty/commit/035768595ae7ef169c308205dfbc08e83ed90b67))
* simplify landing page plan with redirect-based approach ([f196ea7](https://github.com/factorial-io/scotty/commit/f196ea7e26b110a1979026e1259122d14371e436))
* update AGENTS.md for accuracy and condense ([7c27b95](https://github.com/factorial-io/scotty/commit/7c27b95b309be234352a3a44a8e11da7fd1a29e6))
* update AGENTS.md for accuracy and condense to ~150 lines ([64e8cef](https://github.com/factorial-io/scotty/commit/64e8cef081207353830ee7ee8643d55c238e148a))


### Styling

* apply cargo fmt and normalize YAML key ordering ([ecca698](https://github.com/factorial-io/scotty/commit/ecca69809b96bba04683206449c05fe086ced53c))
* fix formatting (cargo fmt) ([db4bdac](https://github.com/factorial-io/scotty/commit/db4bdaca086c8b02b611ac3d01518355f30b391d))
* fix Prettier formatting in landing page ([6131bb8](https://github.com/factorial-io/scotty/commit/6131bb88b5be8a72cfdaa4f5570262da4afe8538))
* fix rustfmt formatting ([62b6cf6](https://github.com/factorial-io/scotty/commit/62b6cf6ea803ab33c15ec62b94c407e596fdc5c0))


### Testing

* **authorization:** stop live-policy test from mutating committed policy.yaml ([49e2017](https://github.com/factorial-io/scotty/commit/49e20173ec44586b270632926ee377ffb42b3b23))
* **authz:** use tracked casbin fixture instead of runtime config ([72f0d33](https://github.com/factorial-io/scotty/commit/72f0d33e5c6dbdae9b3453c8bfb3daa77a0e13f8))


### CI

* build docker image on pull requests to catch breakage before merge ([6c6ad19](https://github.com/factorial-io/scotty/commit/6c6ad19c40a1d2dcc1f76f9160022a8b0543227e))
* fix release-please for virtual workspace manifest ([27dac09](https://github.com/factorial-io/scotty/commit/27dac09b94a347fa53c5f69bb4bf58b6ef298d69))
* fix release-please for virtual workspace manifest ([70e83ad](https://github.com/factorial-io/scotty/commit/70e83ad081828b063564e2f55899b93a80f7ca22))
* grant packages:write and pin binary upload to release tag ([28617d9](https://github.com/factorial-io/scotty/commit/28617d9780c7e82a2bbb93e83c9be0297371c53d))
* replace cargo-release with release-please ([cc44103](https://github.com/factorial-io/scotty/commit/cc441030975f88fe2735003b75067b447e4b9fb6))
* replace cargo-release with release-please ([615fbbc](https://github.com/factorial-io/scotty/commit/615fbbc882604dc4413a847cfc1d97b5a3a635c6))

## [0.3.0](https://github.com/factorial-io/scotty/compare/v0.2.9...v0.3.0) (2026-06-05)


### ⚠ BREAKING CHANGES

* **deps:** requires oauth2 5.x; minimum reqwest is now 0.12.

### Features

* add app:cp file transfer to and from app services ([08c043b](https://github.com/factorial-io/scotty/commit/08c043b7628c8f0163a1c364bc6e5bba4ea6cb60))
* **files:** add server-side file transfer endpoints ([0f7652d](https://github.com/factorial-io/scotty/commit/0f7652dff3bebcf09d541b1c079af79aac86dbc0))
* implement landing page for stopped apps ([f09ed2a](https://github.com/factorial-io/scotty/commit/f09ed2a500a58cfa992f1cf9dc9aa83c44cd48ac))
* **logs:** support logs from stopped containers ([eaf3b1d](https://github.com/factorial-io/scotty/commit/eaf3b1d6c54a8453c1d990aa6050186a18624299))
* per-app custom actions with approval workflow ([a9bb74f](https://github.com/factorial-io/scotty/commit/a9bb74f4b13d0b422d60ce0d3f9e9d2d2cd93851))
* propose app-file-transfer openspec change ([c99ebb5](https://github.com/factorial-io/scotty/commit/c99ebb572c3b7cdcd46ee3800b542c5187169171))
* propose app-file-transfer openspec change ([f9c1e60](https://github.com/factorial-io/scotty/commit/f9c1e60d73a91e9a9bcdb63872118a8974101268))
* **scottyctl:** add app:cp command for file transfer ([1e9747c](https://github.com/factorial-io/scotty/commit/1e9747cb0811edea4e09dbb05b95831352e33191))


### Bug Fixes

* add eslint-disable for external return URL href ([2955e63](https://github.com/factorial-io/scotty/commit/2955e631db75084df28031d03859fd1f34017f66))
* add missing `connect` feature to tokio-tungstenite dependency ([5b3be39](https://github.com/factorial-io/scotty/commit/5b3be39ce48831d2450b9b53135ba2e0c9059e80))
* address fourth-round review on app:cp file transfer ([a051d14](https://github.com/factorial-io/scotty/commit/a051d149d193016adcbd02dec7f510579f9806d7))
* address latest MR review findings for landing page ([06acf53](https://github.com/factorial-io/scotty/commit/06acf5358df5dac0a75e2f30389ffdaf192756c1))
* address PR review findings for landing page ([fad151f](https://github.com/factorial-io/scotty/commit/fad151f0e799b14afecffdfc37baf061d5a2709e))
* address second-round review on app:cp file transfer ([aa64d62](https://github.com/factorial-io/scotty/commit/aa64d624d89b3063ce4864ed917f97af034e6ed1))
* address third-round review on app:cp file transfer ([6be632a](https://github.com/factorial-io/scotty/commit/6be632a28699c5d23d6a272ec6edf7498f5e3450))
* **api:** return proper 404 for missing container paths on file download ([cc94ad7](https://github.com/factorial-io/scotty/commit/cc94ad7a49ffeddb985f0f3bb8f368c3167d3f77))
* **app:cp:** address review feedback on file transfer ([c80c25d](https://github.com/factorial-io/scotty/commit/c80c25df6937854ccae1c25f9ffabdb6a5072780))
* **authz:** serialize policy.yaml with sorted keys to avoid diff churn ([f4bbbe7](https://github.com/factorial-io/scotty/commit/f4bbbe7440f3bc03810978bef90694cbe71b52f6))
* **authz:** stop tracking runtime-mutated policy.yaml, seed from example ([62a499d](https://github.com/factorial-io/scotty/commit/62a499d6f39a14e1a62cb5caca3b60c64fa16fef))
* **ci:** prevent duplicate workflow runs on PR branches ([5eb1dca](https://github.com/factorial-io/scotty/commit/5eb1dca94f4be30cdd8f1d4abe6835e500b806a7))
* **ci:** prevent duplicate workflow runs on PR branches ([c89f3b2](https://github.com/factorial-io/scotty/commit/c89f3b2a0300d5013cb214e774f4713fb068e7c4))
* correct custom action approval authorization and CLI parsing ([f3b9ea8](https://github.com/factorial-io/scotty/commit/f3b9ea880da531b78442ac92f9226df7b51f3071))
* correct documentation errors in CLI docs and AGENTS.md ([70913e1](https://github.com/factorial-io/scotty/commit/70913e131174246ef51debf478d07c01116a3a5e))
* correct documentation errors in CLI docs and AGENTS.md ([e0bc21a](https://github.com/factorial-io/scotty/commit/e0bc21ad0af71564c12e6eb84edacc3a1429f141))
* **deps:** fix sha2 0.11 compatibility ([c228038](https://github.com/factorial-io/scotty/commit/c22803892c4e5e8b684eff307dc9ba757c57ceea))
* **deps:** patch known security vulnerabilities in dependencies ([d7a9496](https://github.com/factorial-io/scotty/commit/d7a949681015da7409bafd30d30686ce0f29f058))
* **deps:** patch known security vulnerabilities in dependencies ([5ddfbf1](https://github.com/factorial-io/scotty/commit/5ddfbf111532e19fc0a22a8ca89fe95e780b7544))
* **deps:** update rust crate sha2 to 0.11 ([8bf7491](https://github.com/factorial-io/scotty/commit/8bf74918d562a57bf3576c69345c4acf8a7ac8bc))
* **deps:** update rust crate sha2 to 0.11 ([1ae1d78](https://github.com/factorial-io/scotty/commit/1ae1d789a514eb7773781fc74c60651f9cdba54c))
* **deps:** update rust crate ts-rs to v12 ([5396ced](https://github.com/factorial-io/scotty/commit/5396ced6a6bd5c7d4656bf4a05cb16d657568c70))
* **deps:** update rust crate ts-rs to v12 ([f6c2e57](https://github.com/factorial-io/scotty/commit/f6c2e576f17b50e45fd266fae921469232ea9a7f))
* **deps:** upgrade oauth2 4.x to 5.x to drop vulnerable reqwest 0.11 chain ([08d3195](https://github.com/factorial-io/scotty/commit/08d31953300554395cfd506ab7ddc50fcd7bbf2a))
* **docs:** consolidate lockfiles on bun and patch dependency advisories ([282abd1](https://github.com/factorial-io/scotty/commit/282abd1b48b396b6c7b0f93d324cda8abb221e09))
* **frontend:** fix ESLint v10 compatibility ([6be5c09](https://github.com/factorial-io/scotty/commit/6be5c0984e1ca600ada2adf8fa53906135c58ee8))
* harden custom action route guards and creation validation ([fc3fdb5](https://github.com/factorial-io/scotty/commit/fc3fdb599359d258fedfb68dbf62bf798d0e5f62))
* move .scottyignore section after app:create argument descriptions ([e8c66e2](https://github.com/factorial-io/scotty/commit/e8c66e23ed7aa1377e158a7f13903845af9aa8d1))
* pass Config to ts-rs export() calls for v12 compatibility ([1850f7f](https://github.com/factorial-io/scotty/commit/1850f7f231e45c34f29d5e73f6bb2337cb68aefb))
* prevent caching of landing page redirect and 404 responses ([1e0e518](https://github.com/factorial-io/scotty/commit/1e0e518340720889e832c4c1f0ae3a3548d48227))
* prevent caching of landing page redirect and 404 responses ([5ff9eab](https://github.com/factorial-io/scotty/commit/5ff9eab6127ee2d123c51382740c878029d2af0d))
* **scottyctl:** app:cp upload translates destination into Docker dir + entry name ([897b442](https://github.com/factorial-io/scotty/commit/897b44276d08d3617969d903df28e381b0d70bd0))
* **tls:** install rustls crypto provider at startup ([75dd751](https://github.com/factorial-io/scotty/commit/75dd7515d405896b7c84c20a1216beab42b4401a))
* update tests for axum-test v19 API and Docker availability ([31f80aa](https://github.com/factorial-io/scotty/commit/31f80aaacdef5df7f7c94346699604613825e1a9))
* upgrade vite to v8, svelte to v5.46.4+, and sveltekit to v2.57.1+ for vite-plugin-svelte v7 compatibility ([26946a5](https://github.com/factorial-io/scotty/commit/26946a52de6f18d0186fa3c7641a8346d5435d6c))
* use bollard_stubs::models for private types in bollard 0.20 ([c38a564](https://github.com/factorial-io/scotty/commit/c38a564f0b2ba0950a760b0d3051858fdc10442f))
* validate action names and tidy custom action endpoints ([a1b5d3b](https://github.com/factorial-io/scotty/commit/a1b5d3b7352f879422dac2a2c8b4b78a5f826d86))


### Refactor

* address second-round review on custom actions ([43560b7](https://github.com/factorial-io/scotty/commit/43560b7d6c124c5789999706943bdc8e8cd3b877))
* configure ts-rs export dir via Config instead of env var ([ffc7a92](https://github.com/factorial-io/scotty/commit/ffc7a92c2b993253eafd5816261ec767a28510d2))
* extract get_domains() on ServicePortMapping to eliminate auto-domain coupling ([40e2551](https://github.com/factorial-io/scotty/commit/40e255124133ff238cc86d8b2c9f41dfbddd2283))
* extract test Docker client creation into shared helper ([8796062](https://github.com/factorial-io/scotty/commit/87960628cf39f52b13fe8d9a06b3d61fc1ffee42))


### Documentation

* add implementation plan for scotty landing page ([fbb89e4](https://github.com/factorial-io/scotty/commit/fbb89e44df1e6e85023e19848302e691473fdaa5))
* expand release-please release guide in README ([dba0a61](https://github.com/factorial-io/scotty/commit/dba0a61c8fdd826e478a60c32f097dac8d7aec07))
* **files:** document app:cp and tidy file-transfer beans ([1e92cd1](https://github.com/factorial-io/scotty/commit/1e92cd1bf95fe58bf04bd6866b4e04d1b5f361c9))
* **openspec:** add stopped-container-logs change proposal ([7c9d7cc](https://github.com/factorial-io/scotty/commit/7c9d7cc8083557feea92ffe1e6f04342b6563a71))
* redesign landing page with Start-button-first UX ([0357685](https://github.com/factorial-io/scotty/commit/035768595ae7ef169c308205dfbc08e83ed90b67))
* simplify landing page plan with redirect-based approach ([f196ea7](https://github.com/factorial-io/scotty/commit/f196ea7e26b110a1979026e1259122d14371e436))
* update AGENTS.md for accuracy and condense ([7c27b95](https://github.com/factorial-io/scotty/commit/7c27b95b309be234352a3a44a8e11da7fd1a29e6))
* update AGENTS.md for accuracy and condense to ~150 lines ([64e8cef](https://github.com/factorial-io/scotty/commit/64e8cef081207353830ee7ee8643d55c238e148a))


### Styling

* apply cargo fmt and normalize YAML key ordering ([ecca698](https://github.com/factorial-io/scotty/commit/ecca69809b96bba04683206449c05fe086ced53c))
* fix formatting (cargo fmt) ([db4bdac](https://github.com/factorial-io/scotty/commit/db4bdaca086c8b02b611ac3d01518355f30b391d))
* fix Prettier formatting in landing page ([6131bb8](https://github.com/factorial-io/scotty/commit/6131bb88b5be8a72cfdaa4f5570262da4afe8538))
* fix rustfmt formatting ([62b6cf6](https://github.com/factorial-io/scotty/commit/62b6cf6ea803ab33c15ec62b94c407e596fdc5c0))


### Testing

* **authorization:** stop live-policy test from mutating committed policy.yaml ([49e2017](https://github.com/factorial-io/scotty/commit/49e20173ec44586b270632926ee377ffb42b3b23))
* **authz:** use tracked casbin fixture instead of runtime config ([72f0d33](https://github.com/factorial-io/scotty/commit/72f0d33e5c6dbdae9b3453c8bfb3daa77a0e13f8))


### CI

* fix release-please for virtual workspace manifest ([27dac09](https://github.com/factorial-io/scotty/commit/27dac09b94a347fa53c5f69bb4bf58b6ef298d69))
* fix release-please for virtual workspace manifest ([70e83ad](https://github.com/factorial-io/scotty/commit/70e83ad081828b063564e2f55899b93a80f7ca22))
* grant packages:write and pin binary upload to release tag ([28617d9](https://github.com/factorial-io/scotty/commit/28617d9780c7e82a2bbb93e83c9be0297371c53d))
* replace cargo-release with release-please ([cc44103](https://github.com/factorial-io/scotty/commit/cc441030975f88fe2735003b75067b447e4b9fb6))
* replace cargo-release with release-please ([615fbbc](https://github.com/factorial-io/scotty/commit/615fbbc882604dc4413a847cfc1d97b5a3a635c6))

## [0.2.9]

### Bug Fixes

- Remove --unreleased flag from changelog generation Phase 3 ✔️

## [0.2.8]

### Bug Fixes

- Enable crossterm use-dev-tty feature for shell sessions spawned by parent processes ✔️
- Derive override filename from compose file for Docker Compose compatibility ✔️

### Dependencies

- Update rust dependencies auto-merge (patch) (#712) ✔️
- Update rust crate tempfile to v3.25.0 ✔️
- Update rust crate sysinfo to v0.38.1 (#708) ✔️
- Update rust crate anyhow to v1.0.101 ✔️
- Bump time from 0.3.37 to 0.3.47 ✔️
- Update otel/opentelemetry-collector docker tag to v0.145.0 ✔️
- Bump bytes from 1.11.0 to 1.11.1 ✔️
- Update rust dependencies auto-merge (patch) ✔️
- Update rust crate flate2 to v1.1.9 ✔️

## [0.2.7]

### Bug Fixes

- Centralize authentication logic for REST and WebSocket ✔️
- Resolve clippy unnecessary_unwrap warning in app info display ✔️
- Disable HTTP redirects and return helpful error message ✔️
- Update dependency @iconify/svelte to v5 ✔️

### Dependencies

- Update rust crate bollard to v0.20.1 (#697) ✔️
- Update rust crate clap to v4.5.56 (#695) ✔️
- Update rust crate clap to v4.5.55 (#694) ✔️
- Update rust crate uuid to v1.20.0 ✔️
- Update rust crate sysinfo to 0.38 ✔️
- Update otel/opentelemetry-collector docker tag to v0.144.0 ✔️
- Update rust crate thiserror to v2.0.18 ✔️
- Update rust docker tag to v1.93 ✔️
- Update rust crate axum-test to v18.7.0 ✔️
- Update rust crate tokio-metrics to v0.4.7 ✔️
- Update rust crate chrono to v0.4.43 ✔️
- Update bun lockfile ✔️
- Update dependency typescript-eslint to v8.53.0 ✔️
- Update dependency svelte to v5.46.3 ✔️
- Update rust crate tower to v0.5.3 ✔️
- Update rust dependencies auto-merge (patch) ✔️
- Update dependency globals to v17 ✔️
- Update dependency typescript-eslint to v8.52.0 ✔️
- Update dependency @iconify/svelte to v5.2.1 ✔️
- Update otel/opentelemetry-collector docker tag to v0.143.1 ✔️
- Update rust crate bcrypt to 0.18.0 ✔️
- Update dependency eslint-plugin-svelte to v3.14.0 ✔️
- Update rust crate axum-test to v18.6.0 ✔️
- Update rust crate bollard to 0.20.0 ✔️
- Update dependency @sveltejs/vite-plugin-svelte to v6.2.4 ✔️
- Update dependency @sveltejs/kit to v2.49.4 ✔️
- Update rust crate clap_complete to v4.5.65 (#671) ✔️
- Update npm dependencies auto-merge (patch) (#670) ✔️
- Update rust crate serde_json to v1.0.149 (#668) ✔️
- Update dependency @sveltejs/vite-plugin-svelte to v6.2.2 (#667) ✔️
- Update rust crate url to v2.5.8 (#666) ✔️
- Update dependency @sveltejs/kit to v2.49.3 (#664) ✔️
- Update rust dependencies auto-merge (patch) (#662) ✔️
- Update rust crate tokio to v1.49.0 ✔️
- Update rust crate clap to v4.5.54 (#659) ✔️
- Update dependency typescript-eslint to v8.51.0 ✔️
- Update rust crate axum-test to v18.5.0 ✔️
- Update rust crate casbin to v2.19.1 (#657) ✔️
- Update rust crate clap_complete to v4.5.64 (#655) ✔️
- Update rust crate clap_complete to v4.5.63 (#652) ✔️
- Update rust crate serde_json to v1.0.148 ✔️
- Update dependency svelte to v5.46.1 ✔️
- Update rust crate tempfile to v3.24.0 ✔️
- Update rust dependencies auto-merge (patch) (#647) ✔️
- Update dependency typescript-eslint to v8.50.1 (#648) ✔️
- Update actions/checkout action to v6 ✔️
- Update dependency typescript-eslint to v8.50.0 ✔️
- Update dependency vite to v7 ✔️
- Update rust docker tag to v1.92 ✔️
- Update rust crate casbin to v2.19.0 ✔️
- Update dependency svelte-check to v4.3.5 ✔️
- Update rust dependencies auto-merge (patch) (#641) ✔️
- Update rust crate governor to v0.10.4 ✔️
- Update otel/opentelemetry-collector docker tag to v0.142.0 ✔️
- Update rust crate reqwest to v0.12.26 (#636) ✔️
- Update dependency prettier-plugin-svelte to v3.4.1 ✔️
- Update rust crate bollard to v0.19.5 ✔️
- Update dependency daisyui to v5.5.14 (#633) ✔️
- Update dependency svelte to v5.46.0 ✔️
- Update dependency eslint to v9.39.2 ✔️

### Documentation

- Add end-user documentation for logs and shell features ✔️

## [0.2.6]

### Bug Fixes

- Enable rustls-tls for oauth2 crate to support HTTPS token endpoints ✔️

### Dependencies

- Update dependency svelte to v5.45.10 ✔️
- Update dependency daisyui to v5.5.13 (#628) ✔️
- Update npm dependencies auto-merge (patch) ✔️
- Update dependency node to 24.12 ✔️
- Update dependency daisyui to v5.5.11 ✔️

## [0.2.5]

### Bug Fixes

- Resolve race conditions in task output streaming ✔️

### Dependencies

- Update dependency svelte to v5.45.8 (#623) ✔️
- Update dependency @sveltejs/kit to v2.49.2 (#622) ✔️
- Update dependency svelte to v5.45.7 ✔️
- Update rust crate reqwest to v0.12.25 ✔️
- Update rust crate tower-http to v0.6.8 ✔️
- Update dependency typescript-eslint to v8.49.0 ✔️

### Documentation

- Add new beads for new features ✔️

## [0.2.4]

### Bug Fixes

- Remove dates from changelog template to avoid timestamp issues ✔️
- Use tag timestamp for changelog dates and clarify release process ✔️
- Clear status line before command exits ✔️
- Add Default implementations for StatusLine and Ui to satisfy clippy ✔️
- Validate token in auth:status and return exit code 1 when invalid (GH#607) ✔️
- Clippy warning in test and strengthen pre-push hook ✔️
- Address code review feedback for HttpError ✔️
- Prioritize explicit access token over cached OAuth tokens in scottyctl ✔️
- Restore frontend/build/.gitkeep to fix CI builds ✔️
- Enable std feature for serde in scotty-types to support String deserialization ✔️
- Add ts-rs tag/content attributes to WebSocketMessage for proper TypeScript generation ✔️
- Address MR feedback - remove duplicate counter, fix naming confusion, optimize hot path ✔️
- Make metrics() return trait object with no-op fallback for tests ✔️
- Restore OAuth session count metrics ✔️
- Refresh app state on task completion for both success and failure ✔️

### Dependencies

- Update rust dependencies auto-merge (patch) ✔️
- Update npm dependencies auto-merge (patch) (#611) ✔️
- Update rust crate uuid to v1.19.0 ✔️
- Update rust crate axum-test to v18.4.1 ✔️
- Update otel/opentelemetry-collector docker tag to v0.141.0 ✔️
- Update dependency typescript-eslint to v8.48.1 (#603) ✔️
- Update dependency svelte to v5.45.3 (#600) ✔️
- Update rust docker tag to v1.91 ✔️
- Update rust crate sysinfo to 0.37 ✔️
- Update dependency prettier to v3.7.3 ✔️
- Update dependency svelte to v5.45.2 ✔️
- Update rust dependencies auto-merge (patch) (#597) ✔️
- Update dependency prettier to v3.7.2 ✔️

### Documentation

- Close issue tracking for scotty-28453 and scotty-a2dce - GH#607 complete ✔️
- Update issue tracking - close scotty-0791a, scotty-46245, create scotty-28453, scotty-6f06c, scotty-a2dce ✔️
- Track scottyctl auth precedence bug (scotty-a84a4, GH #609) ✔️
- Update AGENTS.md to clarify both telemetry transports can be enabled ✔️
- Clarify that telemetry-grpc and telemetry-http can be enabled simultaneously ✔️

### Features

- Preserve HTTP status codes with custom error types ✔️
- Implement metrics tracking for task output streams ✔️
- Implement task output streaming metrics ✔️
- Add no-telemetry feature flag for minimal builds without OpenTelemetry ✔️

### Performance

- Optimize compile times by disabling default features and adding telemetry feature flags ✔️

### Refactor

- Improve status line cleanup documentation and consistency ✔️
- Improve status messages for auth commands ✔️
- Improve error handling with custom ApiError type ✔️
- Remove duplicate retry logic from scottyctl ✔️
- Move active count atomics from static to OtelRecorder instance ✔️
- Simplify telemetry feature logic and test all configurations in CI ✔️
- Remove redundant inherent methods from OtelRecorder ✔️

### Testing

- Add comprehensive test coverage for auth:status token validation ✔️

## [0.2.3]

### Bug Fixes

- Update pre-release-hook to run from workspace root ✔️
- Resolve changelog generation issues with empty sections and subshell ✔️
- Skip empty version sections in per-crate changelogs ✔️
- Update changelog generation to preserve full version history per crate ✔️
- Correct build badge workflow filename ✔️
- Show error when auth token expired in auth:status ✔️
- Scope rust dependency to HEAD builds only ✔️

### Dependencies

- Update rust crate once_cell to v1.21.3 ✔️
- Update rust crate tokio-metrics to 0.4 ✔️

### Documentation

- Replace text header with logo in README ✔️
- Update logo to block-style design and remove padding ✔️
- Close scotty-df8eb as won't fix ✔️
- Add comprehensive documentation for domain-based assignments ✔️
- Track documentation tasks for domain-based authorization ✔️

### Features

- Implement reproducible changelog generation from scratch ✔️
- Add per-crate filtered changelog generation ✔️
- Add domain-based user matching for Casbin RBAC ✔️

### Refactor

- Use Casbin user_match_impl as single source of truth ✔️
- Use Casbin user_match_impl as single source of truth ✔️
- Move Permission enum to scotty-core and iterate over it ✔️

## [0.2.1]

### Bug Fixes

- Scope rust dependency to HEAD builds only ✔️

### Documentation

- Cleanup changelog ✔️

### Refactor

- Standardize to single workspace-level changelog ✔️

## [0.2.0]

### Bug Fixes

- Add Cargo workspace configuration for proper dependency updates ✔️
- Add missing ShellSessionRequest type and generate index.ts ✔️
- Enable TLS support for wss:// connections ✔️
- Implement case-insensitive email matching per RFC 5321 ✔️
- Add allow dead_code attribute to validate_domain_assignment ✔️
- Enable real-time task output streaming in scottyctl ✔️
- Enable real-time task output streaming in scottyctl ✔️
- Improve decompression error handling and size limit enforcement ✔️
- Address PR feedback with proper error handling and security ✔️
- Address PR feedback with tests and security improvements ✔️
- Extract and display error messages from API responses ✔️
- Propagate errors from handlers to tasks ✔️
- Prevent secrets in Docker images and clarify identifier vs token distinction ✔️
- Correct metric names in overview dashboard ✔️
- Reduce VictoriaMetrics retention and disk space threshold ✔️
- Set Y-axis minimum to 0 for memory panels ✔️
- Remove non-functional --workdir option from app:shell command ✔️
- Fix broken doctests after adding lib.rs ✔️
- Suppress dead_code warnings for test utils ✔️
- Resolve clippy warnings for pre-push hook ✔️
- Remove double-wrapping of shell commands ✔️
- Resolve critical TTY mode bugs for interactive shell ✔️
- Add .env file loading support for server configuration ✔️
- Use singleton ShellService across all handlers ✔️
- Add handler for ShellSessionData input ✔️
- Address critical rate limiting issues from PR review ✔️
- Add IP headers to rate limiting integration tests ✔️
- Protect PKCE verifier and CSRF token with MaskedSecret ✔️
- Use placeholder tokens for bearer-tokens ✔️
- Replace hardcoded localhost with configurable frontend base URL ✔️
- Apply constant-time comparison to login handler ✔️
- Replace hardcoded localhost with configurable frontend base URL ✔️
- Apply constant-time comparison to login handler ✔️
- Use constant-time comparison for bearer token validation ✔️
- Apply constant-time comparison to login handler ✔️
- Use constant-time comparison for bearer token validation ✔️
- Replace hardcoded localhost with configurable frontend base URL ✔️
- Generate index.ts with type guards and re-exports ✔️
- Run TypeScript generator from workspace root ✔️
- Run svelte-kit sync before build ✔️
- Point $generated alias to index.ts file explicitly ✔️
- Add $generated path alias for TypeScript generated files ✔️
- Use absolute path from CARGO_MANIFEST_DIR for Docker compatibility ✔️
- Update Dockerfile to use bun.lock instead of bun.lockb ✔️
- Remove tsconfig exclude to fix CI warning ✔️
- Prevent panic on UTF-8 character truncation ✔️
- Normalize URLs to prevent double slashes in API calls (#470) ✔️
- Resolve clippy linting errors in metrics modules ✔️
- Enable HTTP metrics middleware when metrics telemetry is enabled ✔️
- Correct OpenTelemetry metric names in Grafana dashboard ✔️
- Remove resourcedetection processor from otel-collector config ✔️
- Run TypeScript generator from correct working directory ✔️
- Cleanup frontend task output ✔️
- Resolve WebSocket dev mode authentication and security issues ✔️
- Resolve custom actions dropdown reactivity issues ✔️
- Resolve deadlock and lock contention in task management ✔️
- Resolve merge conflicts from main branch ✔️
- Prevent unwanted bindings directory creation ✔️
- Resolve WebSocket integration and task output issues ✔️
- Resolve wildcard scope expansion bug in authorization system ✔️
- Update secure_response_test for removed TaskDetails fields ✔️
- Show container status messages to clients via task output ✔️
- Fix code warning ✔️
- Improve bearer token authentication and error logging ✔️
- Align Casbin model matcher between test and production environments ✔️
- Resolve frontend linting errors ✔️
- Improve authorization security and robustness ✔️
- Resolve clippy warnings and improve code quality ✔️
- Resolve permission-based action button visibility race condition ✔️
- Update OIDC test data and apply code formatting ✔️
- Resolve clap panic in admin permissions test command ✔️
- Centralize user ID logic and fix bearer token authorization ✔️
- Update rust crate tempfile to v3.21.0 ✔️
- Scottyctl bearer token authentication with RBAC ✔️
- Resolve RBAC authorization middleware issues ✔️
- Remove unnecessary assert!(true) statements flagged by clippy ✔️
- Fix task activity indicator animation ✔️
- Resolve TypeScript lint errors and improve type safety ✔️
- Resolve clippy warnings and format code ✔️

### CI

- Generate TypeScript bindings before frontend checks in pre-push hook ✔️
- Add TypeScript generation step to frontend build ✔️
- Trigger ci ✔️

### Dependencies

- Update dependency svelte to v5.43.15 ✔️
- Update rust dependencies auto-merge (patch) ✔️
- Update otel/opentelemetry-collector docker tag to v0.140.1 ✔️
- Update rust crate tower-http to v0.6.7 (#583) ✔️
- Update rust crate tracing-subscriber to v0.3.20 [security] ✔️
- Update rust crate config to v0.15.15 ✔️
- Update npm dependencies auto-merge (patch) (#438) ✔️
- Update rust crate clap to v4.5.46 (#439) ✔️
- Update dependency typescript-eslint to v8.41.0 ✔️

### Documentation

- Add readme to scotty-types ✔️
- Update app:shell documentation to reflect implementation ✔️
- Update frontend README with Scotty-specific documentation ✔️
- Complete CLI documentation with admin and auth commands ✔️
- Complete authorization documentation with admin permissions ✔️
- Clean up intermediate docs and fix OAuth authentication documentation ✔️
- Document task handle behavior and add WebSocket fallback logging ✔️
- Address PR feedback on hybrid auth documentation ✔️
- Add comprehensive hybrid OAuth + bearer token authentication documentation ✔️
- Add documentation for app:logs and app:shell commands ✔️
- Document .env file support for configuration ✔️
- Add comprehensive project documentation ✔️
- Address PR review feedback for rate limiting ✔️
- Add comprehensive rate limiting documentation ✔️
- Document frontend_base_url configuration option ✔️
- Add observability to documentation navigation menu ✔️
- Condense Prometheus compatibility section ✔️
- Add Prometheus compatibility and stack flexibility section ✔️
- Add comprehensive observability documentation (scotty-14) ✔️
- Research OpenTelemetry metrics with OTel Collector + VictoriaMetrics ✔️
- Move progress tracking from docs to beads issues ✔️
- Correct unified output system implementation status ✔️
- Clarify TaskDetails breaking change implementation ✔️
- Update documentation for Phase 3.7 infrastructure optimization ✔️
- Improve OAuth assignments documentation ✔️
- Update CLAUDE.md and PRD with Phase 3.6 completion details ✔️
- Update PRD and CLAUDE.md for Phase 3.5 completion ✔️
- Update PRD and CLAUDE.md to reflect Phase 3 completion ✔️
- Update CLAUDE.md with Phase 2 completion status ✔️
- Document Phase 1 completion and next steps ✔️
- Add code quality reminder to CLAUDE.md ✔️
- Add PRD for unified output system with logs and shell access ✔️
- Enhance bearer token security documentation ✔️
- Update authorization system terminology from groups to scopes ✔️
- Update authorization documentation for RBAC changes ✔️
- Fix CLI command format throughout documentation ✔️
- Add OAuth authentication documentation and update configuration ✔️

### Features

- Add ASCII art logo with version and copyright ✔️
- Add validation, tests, and documentation for domain assignments ✔️
- Add domain-based role assignment support ✔️
- Add gzip compression for file uploads in app:create ✔️
- Add .scottyignore support for app:create ✔️
- Support bearer token fallback when OAuth is enabled ✔️
- Add structured audit logging for compliance ✔️
- Propagate exit codes in command mode ✔️
- Add terminal size support for interactive shell ✔️
- Implement interactive shell with raw terminal mode ✔️
- Add app:shell command and refactor service validation ✔️
- Add comprehensive rate limiting metrics ✔️
- Add rate limiting metrics and Grafana dashboard ✔️
- Implement comprehensive API rate limiting ✔️
- Implement session cleanup and comprehensive monitoring ✔️
- Instrument shell service with metrics (scotty-10) ✔️
- Add task output streaming metrics (scotty-16) ✔️
- Add WebSocket metrics instrumentation (scotty-11) ✔️
- Add AppList metrics for application monitoring (scotty-20) ✔️
- Upgrade to OpenTelemetry 0.31 and implement custom HTTP metrics ✔️
- Enhance metrics collection and add HTTP metrics infrastructure ✔️
- Add Tokio runtime metrics to Grafana dashboard ✔️
- Add stable Tokio task metrics tracking ✔️
- Add task metrics and refactor to use dedicated helper functions ✔️
- Add memory usage metrics (scotty-17) ✔️
- Add Grafana dashboard for scotty metrics ✔️
- Make OTLP endpoint configurable via environment variable ✔️
- Instrument log streaming service with OpenTelemetry metrics ✔️
- Add OpenTelemetry metrics module with ScottyMetrics struct ✔️
- Add OpenTelemetry metrics infrastructure with OTel Collector + VictoriaMetrics ✔️
- Implement container log viewer with navigation improvements ✔️
- Add dedicated OutputStreamType variants for status messages ✔️
- Implement real-time task output and WebSocket integration ✔️
- Enhance message types for frontend integration ✔️
- Add TypeScript bindings generation for WebSocket messages ✔️
- Add Zed debug configuration for scotty server ✔️
- Implement real-time task output streaming for Phase 3.6 ✔️
- Implement unified task output streaming system ✔️
- Improve log command UX and add terminal detection ✔️
- Improve log command options for better UX ✔️
- Simplify --timestamps flag to boolean behavior ✔️
- Implement authenticated WebSocket log streaming with improved UX ✔️
- Integrate logs and shell endpoints into API router ✔️
- Implement logs and shell API endpoints ✔️
- Integrate service errors with AppError ✔️
- Add helper methods for container lookup in AppData ✔️
- Implement bollard log streaming and shell services ✔️
- Add WebSocket message types for logs and shell sessions ✔️
- Refactor TaskDetails and TaskManager to use unified output ✔️
- Add unified output system and configuration ✔️
- Add bollard technical spike and findings documentation ✔️
- Add permission-based visibility for custom actions ✔️
- Implement comprehensive permission-based UI access control ✔️
- Implement OIDC profile picture support in user avatars ✔️
- Enhance OIDC user info capture and logging ✔️
- Use email addresses as user identifiers for OAuth users ✔️
- Implement shared admin types and enhance authentication logging ✔️
- Add comprehensive admin API for authorization management ✔️
- Implement comprehensive RBAC authorization system ✔️
- Unify OAuth error handling system and fix device flow polling ✔️
- Consolidate shared functionality and improve OAuth error handling ✔️
- Implement version compatibility check between scottyctl and server ✔️
- Add comprehensive authentication testing for scotty backend ✔️
- Implement complete OAuth device flow for scottyctl ✔️
- Refactor OAuth to OIDC-compliant provider-agnostic system with Gravatar support ✔️
- Implement OAuth session exchange for secure frontend authentication ✔️
- Optimize healthcheck configuration for faster startup ✔️
- Improve OAuth login flow and authentication validation ✔️
- Implement comprehensive OAuth authentication system ✔️
- Implement OAuth authentication system with hybrid auth modes ✔️

### Performance

- Check bearer tokens before OAuth to avoid network latency ✔️
- Implement token caching to reduce filesystem access ✔️

### Refactor

- Extract task completion logic into shared helper ✔️
- Streamline bearer token check and improve logging context ✔️
- Split monolithic Grafana dashboard into dedicated metric group dashboards ✔️
- Trim ShellSessionData payload in logs ✔️
- Add SessionGuard for panic-safe cleanup ✔️
- Migrate from REST to WebSocket-only implementation ✔️
- Remove unnecessary base64 encoding from PKCE verifier ✔️
- Replace barrel file with inline type guards ✔️
- Update authorization config to use serde_norway ✔️
- Move app metrics into dedicated dashboard row ✔️
- Use spawn_instrumented for consistent Tokio metrics tracking ✔️
- Reorganize observability stack into dedicated directory ✔️
- Consolidate observability stack into main docker-compose.yml ✔️
- Restructure task detail page for consistency ✔️
- Improve log output styling, performance, and controls ✔️
- Unify task completion handlers and fix state management ✔️
- Fix ESLint errors and improve code quality ✔️
- Embed TaskOutput directly in TaskDetails for tight coupling ✔️
- Reduce app state creation duplication in bearer_auth_tests ✔️
- Optimize build system and eliminate type duplication ✔️
- Centralize session management and eliminate token storage duplication ✔️
- Improve messaging consistency and error handling ✔️
- Consolidate message types in scotty-core ✔️
- Restructure handlers into REST and WebSocket modules ✔️
- Reorganize app commands into modular structure and add app:logs command ✔️
- Improve error handling and add helper methods ✔️
- Remove unused get_user_by_token method from AuthorizationService ✔️
- Streamline admin CLI command error handling ✔️
- Remove emojis from admin command success messages ✔️
- Replace authorization groups terminology with scopes ✔️
- Make RBAC configuration mandatory and improve logging ✔️
- Update auth commands to use UI helper and reduce emoji usage ✔️

### Security

- Fix domain extraction to prevent bypass via multiple @ symbols ✔️

### Styling

- Apply cargo fmt and fix clippy warnings ✔️
- Apply cargo fmt formatting ✔️
- Apply cargo fmt formatting fixes ✔️

### Testing

- Add E2E WebSocket integration tests ✔️
- Add comprehensive unit tests for shell feature ✔️
- Add comprehensive tests for logs and shell services ✔️

## [0.1.0]

### Bug Fixes

- Correct cargo-release README.md path in workspace metadata ✔️
- Increase registry cleanup rate to 100 images per run ✔️
- Improve error reporting and fix env vars in custom actions ✔️
- Restore custom actions dropdown functionality and divider visibility ✔️

### CI

- Update docker-cleanup.yml to delete all tags ✔️

### Dependencies

- Update traefik docker tag to v3.6 ✔️
- Update dependency typescript-eslint to v8.48.0 ✔️
- Update dependency svelte to v5.43.14 (#578) ✔️
- Update dependency typescript-eslint to v8.47.0 ✔️
- Update dependency @sveltejs/kit to v2.49.0 ✔️
- Update rust dependencies auto-merge (patch) (#577) ✔️
- Update rust crate clap to v4.5.52 (#571) ✔️
- Update dependency svelte to v5.43.8 (#569) ✔️
- Update dependency daisyui to v5.5.5 (#567) ✔️
- Update dependency svelte to v5.43.7 (#566) ✔️
- Update rust crate axum to v0.8.7 (#565) ✔️
- Update dependency daisyui to v5.5.4 (#564) ✔️
- Update dependency @sveltejs/kit to v2.48.5 ✔️
- Update rust crate config to v0.15.19 ✔️
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
- Update rust crate bollard to v0.19.4 (#540) ✔️
- Update dependency daisyui to v5.3.11 (#539) ✔️
- Update dependency globals to v16.5.0 ✔️
- Update dependency eslint-plugin-svelte to v3.13.0 ✔️
- Update rust docker tag to v1.91 ✔️
- Update dependency @sveltejs/kit to v2.48.4 (#536) ✔️
- Update dependency svelte to v5.43.2 ✔️
- Update rust dependencies auto-merge (patch) (#533) ✔️
- Update dependency @sveltejs/kit to v2.48.3 (#532) ✔️
- Update dependency eslint to v9.38.0 ✔️
- Update dependency node to v24 ✔️
- Update dependency svelte to v5.43.0 ✔️
- Update dependency daisyui to v5.3.10 (#530) ✔️
- Update dependency @sveltejs/vite-plugin-svelte to v6.2.1 ✔️
- Update dependency @sveltejs/kit to v2.48.2 ✔️
- Update npm dependencies auto-merge (patch) (#511) ✔️
- Update dependency @sveltejs/kit to v2.47.3 ✔️
- Update dependency node to 22.21 ✔️
- Update rust crate tokio to v1.48.0 ✔️
- Update rust crate tempfile to v3.23.0 ✔️

### Documentation

- Add version to readme ✔️

## [0.1.0-alpha.38]

### Bug Fixes

- Sanitize service names in autogenerated environment variables ✔️
- Address code review feedback ✔️
- Apply environment variables to all containers, not only public services ✔️
- Normalize URLs to prevent double slashes in API calls (#470) ✔️
- Update dependency @tailwindcss/typography to v0.5.18 ✔️
- Update npm dependencies auto-merge (patch) to v5.0.2 ✔️
- Update rust crate tempfile to v3.22.0 ✔️
- Update dependency @iconify/svelte to v5 ✔️
- Update rust crate tempfile to v3.21.0 ✔️
- Fix UI issues and provide sort handler default ✔️
- Remove unused CustomActionResponse struct ✔️
- Correct method calls for table column modification ✔️
- Correct function call for formatting services commands ✔️
- Rename format_services_command to format_services_commands for clarity ✔️
- Fix iteration and formatting issues in blueprint lifecycle actions ✔️
- Update usage of InspectContainerOptions for compatibility ✔️

### CI

- Trigger ci ✔️
- Increase retained container versions for cleanup ✔️
- Update cleanup workflow to new action version ✔️

### Dependencies

- Update dependency svelte to v5.41.1 ✔️
- Update dependency vite to v6.4.1 [security] ✔️
- Update rust crate clap to v4.5.50 (#505) ✔️
- Update npm dependencies auto-merge (patch) (#504) ✔️
- Update npm dependencies auto-merge (patch) (#496) ✔️
- Update rust crate zeroize to v1.8.2 ✔️
- Update dependency globals to v16.4.0 ✔️
- Update dependency typescript-eslint to v8.46.1 ✔️
- Update rust crate regex to v1.12.2 ✔️
- Update dependency daisyui to v5.3.2 (#494) ✔️
- Update dependency svelte to v5.40.0 ✔️
- Update dependency daisyui to v5.3.1 ✔️
- Update dependency node to 22.20 ✔️
- Update rust dependencies auto-merge (patch) (#484) ✔️
- Update npm dependencies auto-merge (patch) (#486) ✔️
- Update dependency @sveltejs/kit to v2.46.5 ✔️
- Update dependency svelte to v5.39.3 ✔️
- Update dependency svelte to v5.39.2 ✔️
- Update dependency daisyui to v5.1.13 (#479) ✔️
- Update rust dependencies auto-merge (patch) (#477) ✔️
- Update dependency typescript-eslint to v8.44.0 ✔️
- Update dependency @factorial/docs to v0.5.6 (#476) ✔️
- Update rust dependencies auto-merge (patch) (#474) ✔️
- Update rust dependencies auto-merge (patch) (#473) ✔️
- Update dependency @sveltejs/kit to v2.39.1 (#472) ✔️
- Update dependency svelte to v5.38.10 (#471) ✔️
- Update dependency @sveltejs/kit to v2.39.0 ✔️
- Update dependency svelte to v5.38.9 (#468) ✔️
- Update dependency @sveltejs/kit to v2.38.1 ✔️
- Update dependency eslint-plugin-svelte to v3.12.3 ✔️
- Update dependency daisyui to v5.1.10 ✔️
- Update dependency svelte to v5.38.8 (#461) ✔️
- Update rust crate chrono to v0.4.42 (#459) ✔️
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
- Update rust dependencies auto-merge (patch) (#446) ✔️
- Update dependency @sveltejs/vite-plugin-svelte to v6.1.4 (#445) ✔️
- Update dependency node to 22.19 ✔️
- Update rust crate tracing-subscriber to v0.3.20 [security] ✔️
- Update rust crate config to v0.15.15 ✔️
- Update npm dependencies auto-merge (patch) (#438) ✔️
- Update rust crate clap to v4.5.46 (#439) ✔️
- Update dependency typescript-eslint to v8.41.0 ✔️
- Update rust crate regex to v1.11.2 (#435) ✔️
- Update dependency @sveltejs/kit to v2.36.2 (#434) ✔️
- Update dependency svelte to v5.38.3 (#433) ✔️
- Update rust crate url to v2.5.7 (#432) ✔️
- Update dependency @sveltejs/kit to v2.36.1 ✔️
- Update dependency eslint to v9.34.0 ✔️
- Update rust crate url to v2.5.6 (#429) ✔️
- Update dependency @sveltejs/kit to v2.36.0 ✔️
- Update rust crate thiserror to v2.0.16 (#428) ✔️
- Update dependency @sveltejs/vite-plugin-svelte to v6.1.3 (#425) ✔️
- Update rust crate serde_json to v1.0.143 (#424) ✔️
- Update dependency @sveltejs/kit to v2.33.0 ✔️
- Update dependency typescript-eslint to v8.40.0 ✔️
- Update dependency node to 22.18 ✔️
- Update dependency svelte to v5.38.2 ✔️
- Update rust crate bcrypt to v0.17.1 ✔️
- Update rust crate uuid to v1.18.0 ✔️
- Update npm dependencies auto-merge (patch) to v4.1.12 ✔️
- Update rust crate thiserror to v2.0.15 ✔️
- Update rust crate async-trait to v0.1.89 (#413) ✔️
- Update dependency @sveltejs/kit to v2.30.1 ✔️
- Update dependency @sveltejs/kit to v2.30.0 ✔️
- Update dependency @sveltejs/kit to v2.29.1 ✔️
- Update frontend dependencies to latest versions ✔️
- Update rust dependencies auto-merge (patch) (#407) ✔️
- Update rust crate reqwest to v0.12.23 (#406) ✔️
- Update dependency @sveltejs/kit to v2.28.0 ✔️
- Update rust dependencies auto-merge (patch) (#403) ✔️
- Update dependency typescript-eslint to v8.39.1 (#402) ✔️
- Update dependency typescript to v5.9.2 ✔️
- Update rust docker tag to v1.89 ✔️
- Update dependency eslint to v9.33.0 ✔️
- Update rust crate clap_complete to v4.5.56 (#397) ✔️
- Update npm dependencies auto-merge (patch) ✔️
- Update dependency @sveltejs/kit to v2.27.2 ✔️
- Update rust crate clap to v4.5.43 ✔️
- Update dependency @sveltejs/kit to v2.27.1 ✔️
- Update dependency typescript-eslint to v8.39.0 ✔️
- Update rust dependencies auto-merge (patch) (#390) ✔️
- Update dependency eslint-plugin-svelte to v3.11.0 ✔️
- Update dependency globals to v16.3.0 ✔️
- Bump form-data from 4.0.1 to 4.0.4 in /docs ✔️
- Update dependency eslint to v9.32.0 ✔️
- Update traefik docker tag to v3.5 ✔️
- Update dependency daisyui to v5.0.50 (#388) ✔️
- Update dependency svelte-check to v4.3.1 ✔️
- Update dependency daisyui to v5.0.47 (#386) ✔️
- Update dependency @sveltejs/kit to v2.26.1 ✔️
- Update dependency typescript-eslint to v8.38.0 ✔️
- Update rust crate serde_json to v1.0.141 (#383) ✔️
- Update dependency eslint-config-prettier to v10.1.8 (#382) ✔️
- Update dependency @sveltejs/kit to v2.24.0 ✔️
- Update dependency typescript-eslint to v8.37.0 ✔️
- Update dependency @sveltejs/kit to v2.23.0 ✔️
- Update dependency eslint to v9.31.0 ✔️
- Update rust docker tag to v1.88 ✔️
- Update dependency @sveltejs/kit to v2.22.5 (#371) ✔️
- Update rust crate thiserror to v2.0.12 (#369) ✔️
- Update rust crate thiserror to v2 ✔️
- Update dependency eslint to v9.30.1 ✔️
- Update rust crate tabled to 0.20.0 ✔️
- Update dependency daisyui to v5.0.46 ✔️
- Update rust dependencies auto-merge (patch) ✔️
- Update rust crate utoipa to v5.4.0 ✔️
- Update dependency @sveltejs/kit to v2.22.4 ✔️
- Update dependency prettier to v3.6.2 ✔️
- Update dependency typescript-eslint to v8.36.0 ✔️
- Update rust crate owo-colors to v4.2.2 (#364) ✔️
- Update dependency eslint-plugin-svelte to v3.9.3 (#361) ✔️
- Update dependency svelte-check to v4.2.2 (#360) ✔️
- Update npm dependencies auto-merge (patch) (#358) ✔️
- Update rust crate bollard to v0.19.1 (#357) ✔️
- Update dependency eslint to v9.29.0 ✔️
- Update dependency @sveltejs/kit to v2.21.5 (#355) ✔️
- Update dependency postcss to v8.5.5 (#354) ✔️
- Update rust crate reqwest to v0.12.20 (#353) ✔️
- Update npm dependencies auto-merge (patch) (#352) ✔️
- Update rust crate clap_complete to v4.5.54 ✔️
- Update rust dependencies auto-merge (patch) ✔️
- Update dependency typescript-eslint to v8.34.0 ✔️
- Update rust crate bollard to 0.19.0 ✔️
- Update dependency @sveltejs/kit to v2.21.3 (#347) ✔️

### Documentation

- Document augmented environment variables for blueprint actions ✔️
- Correct typo in middleware section ✔️
- Update CLI documentation with new installation instructions and options ✔️
- Update preferred CLI installation method ✔️

### Features

- Migrate core secrets to MaskedSecret (Phase 1) ✔️
- Implement MaskedSecret and SecretHashMap for memory-safe secret handling ✔️
- Replace serde_yml with serde_norway dependency ✔️
- Upgrade frontend to latest major versions ✔️
- Add Traefik middleware support and examples ✔️
- Add custom actions dropdown component for app blueprints ✔️
- Add notification type for completed custom app actions ✔️
- Restructure blueprint actions with unified description format ✔️
- Add blueprint:info command and action descriptions ✔️
- Add support for custom actions on apps ✔️

### Refactor

- Migrate environment variables to SecretHashMap ✔️
- Simplify lifecycle action handling ✔️
- Update import path for InspectContainerOptions ✔️

### Styling

- Normalize indentation in app.css ✔️
- Apply new Rust format string syntax ✔️
- Reformat confirmation prompt for clarity ✔️

### Testing

- Add serialization and deserialization tests for AppTtl ✔️

## [0.1.0-alpha.37]

### Bug Fixes

- Try to fix homebrew formula ✔️

## [0.1.0-alpha.36]

### Bug Fixes

- Try to fix homebrew formula ✔️

## [0.1.0-alpha.35]

### Bug Fixes

- Try to fix homebrew formula ✔️

## [0.1.0-alpha.34]

### CI

- Rewrite Homebrew formula publishing workflow ✔️
- Use new token for brew ✔️

### Dependencies

- Update dependency @sveltejs/kit to v2.21.2 (#345) ✔️

## [0.1.0-alpha.33]

### Bug Fixes

- Add SecureJson wrapper to mask sensitive env vars in API responses ✔️
- Update rust dependencies auto-merge (patch) (#343) ✔️

### Dependencies

- Add tempfile as a dev dependency ✔️
- Update npm dependencies auto-merge (patch) (#342) ✔️
- Update dependency eslint to v9.28.0 ✔️
- Update dawidd6/action-homebrew-bump-formula action to v4 ✔️

## [0.1.0-alpha.32]

### Bug Fixes

- Reduce lock scope in wait_for_all_containers_handler ✔️
- Remove duplicate WaitForAllContainers handler ✔️
- Add container readiness check and improve Drush commands ✔️
- Update rust dependencies auto-merge (patch) (#337) ✔️

### CI

- Combine release and Homebrew publishing workflows ✔️

### Dependencies

- Update dependency typescript-eslint to v8.33.0 ✔️
- Update dependency daisyui to v5.0.43 ✔️

### Documentation

- Clarify install instructions ✔️

### Features

- Wait for containers to be ready before running post-actions ✔️

### Refactor

- Improve error handling and simplify collection logic ✔️
- Refactor app_data.rs into modular components ✔️

## [0.1.0-alpha.31]

### Features

- Update homebrew tap on new releases ✔️

## [0.1.0-alpha.30]

### Bug Fixes

- Standardize domain hash to 6 fixed-width hex characters ✔️
- Use domain-safe app names when creating domains (Fixes #328) ✔️
- Use domain-safe app names when creating domains (Fixes #328) ✔️
- Make AppContext fields private with getter methods ✔️
- Update rust dependencies auto-merge (patch) (#332) ✔️
- Update rust dependencies auto-merge (patch) (#329) ✔️
- Update rust dependencies auto-merge (patch) ✔️
- Update rust dependencies auto-merge (patch) (#324) ✔️
- Replace atty dependency with std::io::IsTerminal ✔️

### Dependencies

- Update npm dependencies auto-merge (patch) (#333) ✔️
- Update dependency daisyui to v5.0.40 (#331) ✔️
- Update dependency daisyui to v5.0.38 (#327) ✔️
- Update dependency eslint-plugin-svelte to v3.9.0 ✔️
- Update dependency globals to v16.2.0 ✔️
- Update rust crate uuid to v1.17.0 ✔️
- Update dependency daisyui to v5.0.37 (#322) ✔️
- Update dependency svelte to v4.2.20 (#321) ✔️
- Update dependency @sveltejs/kit to v2.21.1 (#320) ✔️
- Update dependency eslint to v9.27.0 ✔️
- Update dependency eslint-plugin-svelte to v3.7.0 ✔️
- Update dependency @sveltejs/kit to v2.21.0 ✔️
- Update dependency svelte-check to v4.2.1 ✔️
- Update rust crate owo-colors to v4.2.1 ✔️
- Update rust docker tag to v1.87 ✔️
- Update dependency prettier-plugin-svelte to v3.4.0 ✔️
- Update dependency eslint-plugin-svelte to v3.6.0 ✔️
- Update npm dependencies auto-merge (patch) (#307) ✔️

### Features

- Refactor to use shared AppContext with unified UI ✔️
- Add retry mechanism with backoff for API calls ✔️
- Expose public URLs as environment variables to actions ✔️

### Refactor

- Refactor app_data.rs into modular components ✔️

## [0.1.0-alpha.29]

### Bug Fixes

- Remove trailing newlines from UI messages ✔️
- Fix: Change task output from stderr to stdout if it was targeted to
stdout ❌
- Include file path in env file parse error message ✔️
- Fix environment variable precedence in app creation ✔️
- Update rust crate tempfile to v3.20.0 ✔️
- Support binary file handling in file reading ✔️
- Enhance error messages for root folder path resolution ✔️
- Update rust crate tempfile to v3.19.1 ✔️

### Dependencies

- Update dependency typescript-eslint to v8.32.0 ✔️
- Update dependency globals to v16.1.0 ✔️
- Update rust dependencies auto-merge (patch) (#304) ✔️
- Update dependency eslint-config-prettier to v10.1.5 (#303) ✔️
- Update rust crate tower-http to v0.6.3 (#302) ✔️
- Update dependency eslint-config-prettier to v10.1.3 (#300) ✔️
- Update rust crate tokio to v1.45.0 ✔️
- Update rust crate axum to v0.8.4 (#297) ✔️
- Update rust crate axum to 0.8.0 ✔️
- Update traefik docker tag to v3.4 ✔️
- Update rust dependencies auto-merge (patch) ✔️
- Update dependency eslint to v9.26.0 ✔️
- Update npm dependencies auto-merge (patch) (#291) ✔️
- Update rust crate config to 0.15.0 ✔️
- Update dependency daisyui to v5 ✔️

### Documentation

- Improve examples in AppData documentation ✔️

### Features

- Enhance user interface with status line functionality ✔️
- Enhance status line with emoji indicators ✔️
- Embed frontend files into the executable ✔️

### Refactor

- Modularize and reorganize file and parser utilities ✔️
- Implement custom debug for file structure ✔️
- Introduce StatusLine for better status tracking and UI feedback ✔️
- Streamline router setup for improved clarity ✔️
- Upgrade axum to 0.8.1 ✔️
- Improve builder pattern for configuration loading ✔️

## [0.1.0-alpha.28]

### Features

- Add title management for dynamic page titles ✔️
- Add dynamic page titles across key sections ✔️
- Enhance environment-variable display and add defaults ✔️

### Refactor

- Utilize reusable Pill component for status display ✔️

## [0.1.0-alpha.27]

### Bug Fixes

- Allow test commits to be included in change logs ✔️

## [0.1.0-alpha.26]

## [0.1.0-alpha.25]

### Dependencies

- Update dependency vite to v5.4.19 [security] (#288) ✔️

### Refactor

- Change AppSettings from_file to return Option ✔️

### Testing

- Add tests for handling edge cases in environment variable parsing ✔️

## [0.1.0-alpha.24]

### Bug Fixes

- Improve password masking in URI handling ✔️
- Update security scheme to bearerAuth ✔️
- Improve error handling for missing environment variables ✔️

### Dependencies

- Update rust crate tabled to 0.19.0 ✔️
- Update rust dependencies auto-merge (patch) ✔️
- Update dependency typescript-eslint to v8.31.1 ✔️
- Update dependency eslint to v9.25.1 (#281) ✔️
- Update rust crate clap to v4.5.37 (#280) ✔️
- Update dependency eslint to v9.25.0 ✔️

### Documentation

- Document bearer token authentication with utoipa ✔️

### Features

- Display application last checked timestamp ✔️
- Enhance sensitive data handling with URI credential masking ✔️
- Add redaction for sensitive environment variables ✔️
- Add environment variable substitution functionality ✔️
- Enhance Docker Compose validation for environment variables ✔️
- Add support for environment variables in docker-compose validation ✔️

### Refactor

- Improve environment variable checking in Docker Compose validation ✔️
- Simplify environment variable checks and service validation ✔️
- Simplify environment variable processing ✔️
- Streamline regex initialization in environment variable processing ✔️
- Enhance docker-compose command execution with better error handling and documentation ✔️

### Styling

- Improve readability of conditional statement ✔️
- Format code for consistency and readability ✔️

## [0.1.0-alpha.23]

### Refactor

- Improve env var parsing logic ✔️

## [0.1.0-alpha.22]

### Bug Fixes

- Fix crash wehn blueprint cant be found, return proper error now ✔️
- Small syntax fix in svelte-code ✔️
- CI use latest actions for rust ✔️
- CI use latest actions for rust ✔️
- CI use latest actions for rust ✔️
- CI setup ✔️
- Fix some linting errors, setup editorconfig ✔️

### Dependencies

- Update npm dependencies auto-merge (patch) (#274) ✔️
- Update dependency typescript-eslint to v8.30.0 ✔️
- Update dependency @sveltejs/kit to v2.20.6 [security] ✔️
- Update rust crate anyhow to v1.0.98 (#271) ✔️
- Update dependency svelte-check to v4.1.6 ✔️
- Update rust crate clap to v4.5.36 (#269) ✔️
- Update dependency eslint-config-prettier to v10.1.2 (#268) ✔️
- Update dependency vite to v5.4.18 ✔️
- Update dependency @sveltejs/kit to v2.20.5 ✔️
- Update dependency @sveltejs/adapter-auto to v6 ✔️
- Update dependency eslint to v9.24.0 ✔️
- Update npm dependencies auto-merge (patch) (#262) ✔️
- Update rust docker tag to v1.86 ✔️
- Bump tokio from 1.42.0 to 1.44.2 ✔️
- Bump openssl from 0.10.70 to 0.10.72 ✔️
- Update dependency eslint to v9.23.0 ✔️
- Update dependency typescript-eslint to v8.29.0 ✔️
- Update dependency vite to v5.4.17 ✔️
- Update dependency eslint-plugin-svelte to v3.5.1 ✔️
- Update rust crate clap to v4.5.35 (#251) ✔️
- Update dependency vite to v5.4.16 [security] ✔️
- Update dependency @sveltejs/kit to v2.20.3 ✔️
- Update rust dependencies auto-merge (patch) (#247) ✔️
- Update dependency vite to v5.4.15 [security] (#245) ✔️
- Update dependency eslint-plugin-svelte to v3.4.0 ✔️
- Bump zip from 2.2.2 to 2.4.1 ✔️
- Update rust crate deunicode to v1.6.1 (#241) ✔️
- Update dependency @sveltejs/kit to v2.19.2 ✔️
- Update rust crate async-trait to v0.1.88 (#239) ✔️
- Update dependency @sveltejs/kit to v2.19.1 (#238) ✔️
- Update rust crate uuid to v1.16.0 ✔️
- Update rust crate reqwest to v0.12.14 (#236) ✔️
- Update dependency eslint-plugin-svelte to v3.1.0 ✔️
- Bump ring from 0.17.8 to 0.17.13 ✔️
- Update rust crate reqwest to v0.12.13 ✔️
- Bump prismjs from 1.29.0 to 1.30.0 in /docs ✔️
- Update rust crate clap to v4.5.32 (#233) ✔️
- Update dependency typescript-eslint to v8.26.1 ✔️
- Update rust crate serde to v1.0.219 (#230) ✔️
- Update dependency autoprefixer to v10.4.21 (#229) ✔️
- Update dependency @sveltejs/kit to v2.19.0 ✔️
- Update dependency eslint to v9.22.0 ✔️
- Update dependency @sveltejs/kit to v2.18.0 ✔️
- Update dependency eslint-config-prettier to v10.1.1 ✔️
- Update dependency svelte-check to v4.1.5 ✔️
- Update dependency eslint-plugin-svelte to v3.0.3 ✔️
- Update rust crate readonly to v0.2.13 (#221) ✔️
- Update dependency typescript-eslint to v8.26.0 ✔️
- Update rust dependencies auto-merge (patch) (#219) ✔️
- Update dependency prettier to v3.5.3 (#218) ✔️
- Update dependency typescript-eslint to v8.25.0 ✔️
- Update dependency eslint to v9.21.0 ✔️
- Update dependency typescript to v5.8.2 ✔️
- Update dependency prettier to v3.5.2 ✔️
- Update npm dependencies auto-merge (patch) (#210) ✔️
- Update dependency globals to v16 ✔️
- Update dependency @sveltejs/vite-plugin-svelte to v5 ✔️
- Update dependency vite to v6 ✔️
- Update rust crate owo-colors to v4.2.0 ✔️
- Update rust crate uuid to v1.15.1 ✔️
- Update rust docker tag to v1.85 ✔️
- Update dependency eslint to v9.21.0 ✔️
- Update dependency globals to v15.15.0 ✔️
- Update opentelemetry packages ✔️
- Update dependency typescript-eslint to v8.25.0 ✔️
- Update dependency typescript to v5.8.2 ✔️
- Update dependency prettier to v3.5.2 ✔️
- Update rust dependencies auto-merge (patch) ✔️
- Update npm dependencies auto-merge (patch) (#195) ✔️
- Update dependency eslint-config-prettier to v10 ✔️
- Update rust crate bcrypt to 0.17.0 ✔️
- Update rust crate tabled to 0.18.0 ✔️

### Documentation

- Clarify usage, and prevent some frustration when trying to get things running ✔️

### Features

- Add new flag destroy_on_ttl which lets you destroy an app instead of stopping it after the TTL expired. ✔️
- Add dotenv integration and restructure scottyctl command handling ✔️
- Lint + check frontend in ci ✔️

## [0.1.0-alpha.21]

### Bug Fixes

- Fix middleware setup for traefik config and multiple domains #194 ✔️
- Enable docker registry cleanup ✔️
- Try to fix docker cleanup ✔️

### Dependencies

- Update rust crate uuid to v1.13.1 ✔️
- Update dependency tailwindcss to v4 ✔️
- Update rust docker tag to v1.84 ✔️
- Bump nanoid from 3.3.7 to 3.3.8 in /frontend ✔️
- Update dependency @sveltejs/adapter-auto to v4 ✔️
- Update rust dependencies auto-merge (patch) (#182) ✔️
- Update dependency @sveltejs/kit to v2.17.1 ✔️
- Bump openssl from 0.10.68 to 0.10.70 ✔️

### Documentation

- Clarify documentation (#181) ✔️
- Clarify autocompletion ✔️

## [0.1.0-alpha.20]

### Bug Fixes

- Log into registry before trying to run docker run ✔️
- Update dependency @iconify/svelte to v4.2.0 ✔️
- Apply clippy fixes ✔️
- Update npm dependencies auto-merge (patch) ✔️

### Dependencies

- Update rust crate uuid to v1.12.1 ✔️
- Update dependency typescript-eslint to v8.21.0 ✔️
- Update rust dependencies auto-merge (patch) ✔️
- Update dependency postcss to v8.5.1 ✔️
- Update dependency vite to v5.4.12 [security] ✔️
- Update dependency @factorial/docs to v0.5.5 ✔️
- Update rust dependencies auto-merge (patch) (#160) ✔️
- Update npm dependencies auto-merge (patch) ✔️
- Update dependency eslint to v9.18.0 ✔️
- Update rust crate axum-tracing-opentelemetry to 0.25.0 ✔️
- Update dependency typescript to v5.7.3 (#165) ✔️
- Update traefik docker tag to v3.3 ✔️
- Update dependency typescript-eslint to v8.19.1 ✔️

### Documentation

- Clarify installation from source ✔️

### Testing

- Add a test for haproxy and custom domains ✔️

## [0.1.0-alpha.19]

### Bug Fixes

- Fork slugify to support up to two dashes as separator ✔️

## [0.1.0-alpha.18]

### Bug Fixes

- Slugify app-names passed to the API -- (Fixes #158) ✔️

## [0.1.0-alpha.17]

### Bug Fixes

- Fix for wrong traefik config regarding TLS (Fixes #157) ✔️

### Dependencies

- Update rust crate async-trait to v0.1.84 (#156) ✔️
- Update rust dependencies auto-merge (patch) (#154) ✔️

### Documentation

- Update docs how to create a release ✔️

## [0.1.0-alpha.16]

### Bug Fixes

- Update rust crate init-tracing-opentelemetry to 0.25.0 (#128) ✔️

### Dependencies

- Update dependency typescript-eslint to v8.19.0 (#150) ✔️
- Update rust crate serde to v1.0.217 (#153) ✔️

### Documentation

- Fix cli docs ✔️
- Add readmes for all three apps/libs ✔️
- Update readme ✔️
- Update badges ✔️
- Update the readme and remove redundancy ✔️
- Add documentation for shell autocompletion ✔️

### Features

- Restructure into workspaces (#152) ✔️
- Add new subcommand to generate completion scripts for shell autocompletion ✔️

## [0.1.0-alpha.15]

### Dependencies

- Update dependency @sveltejs/kit to v2.15.1 ✔️

## [0.1.0-alpha.14]

### Bug Fixes

- Update rust crate anyhow to v1.0.95 (#146) ✔️
- Update rust crate serde_json to v1.0.134 (#144) ✔️
- Fix for crash when docker container with specific id is not available (Fixes #139) ✔️
- Recreate loadbalancer config for app:rebuild ✔️
- Rename migrate to adopt also for the cli ✔️

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

### Features

- Add `blueprint:list` command to scotty cli ✔️

## [0.1.0-alpha.13]

### Bug Fixes

- Update app detail when needed ✔️
- Check for app-changes every 15 secs ✔️
- Use proper type for AppTtl ✔️
- Handle missing domains in yaml files correctly, print an error message if the settings file couldnt be read ✔️
- Update rust crate serde to v1.0.216 (#129) ✔️
- Update dependency @iconify/svelte to v4.1.0 ✔️
- Update url dependency to prevent dependabot alert #8 ✔️
- Increase default ttl to 7 days ✔️
- Update rust crate init-tracing-opentelemetry to v0.24.2 ✔️

### Dependencies

- Update dependency @sveltejs/kit to v2.11.1 ✔️
- Update dependency daisyui to v4.12.22 (#132) ✔️
- Update dependency daisyui to v4.12.21 (#131) ✔️
- Update dependency @sveltejs/kit to v2.10.1 ✔️

### Features

- Try to adopt basic_auth data when available ✔️
- Rename app:migrate to app:adopt ✔️
- Show version string in footer ✔️
- Add support for multiple domains and settings in UI ✔️
- Reenable dark theme ✔️
- Support multiple domains for a service (fixes #126) ✔️
- Export env-vars to settings when migrating an app ✔️

## [0.1.0-alpha.12]

### Bug Fixes

- Update rust crate tokio to v1.42.0 ✔️
- Fix frontend build ✔️
- Update rust crate chrono to v0.4.39 (#118) ✔️
- Update rust crate tokio-stream to v0.1.17 (#115) ✔️
- Update rust crate clap to v4.5.23 (#114) ✔️
- Update rust crate clap to v4.5.22 (#112) ✔️

### Dependencies

- Update dependency globals to v15.13.0 ✔️
- Update dependency typescript-eslint to v8.18.0 ✔️
- Update dependency @sveltejs/kit to v2.9.1 ✔️
- Update dependency daisyui to v4.12.20 (#117) ✔️
- Update dependency prettier to v3.4.2 (#113) ✔️
- Update npm dependencies auto-merge (patch) (#110) ✔️

### Features

- Apply environment also when running the docker-compose commands, Add a preliminary migrate command to create a .scotty-file ✔️

## [0.1.0-alpha.11]

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

## [0.1.0-alpha.10]

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

## [0.1.0-alpha.9]

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

## [0.1.0-alpha.8]

### CI

- Fix cross compiling for linux, disable linux arm for now ✔️

## [0.1.0-alpha.7]

### CI

- Fix cross compiling for linux ✔️

## [0.1.0-alpha.6]

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

## [0.1.0-alpha.5]

### CI

- Add openssl to dependencies to fix problem with cross-compilation in ci ✔️

### Documentation

- Document how to create a new release ✔️

## [0.1.0-alpha.4]

### CI

- Enable changelog for ci changes ✔️

## [0.1.0-alpha.3]
