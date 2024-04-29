# Changelog

All notable changes to this project will be documented in this file.

## [unreleased]

### 🚀 Features

- Add svelte-based frontend web client - ([00160e9](https://github.com/alcroito/digitalocean-dyndns/commit/00160e9b1fca4c5a818a70d69607599c65c162fd)) by ([@alcroito](https://github.com//alcroito))
- Add a REST API rust backend based on Axum and OpenAPI - ([7f8384e](https://github.com/alcroito/digitalocean-dyndns/commit/7f8384eb421778b9a7db9d8072ee3ee0312a379c)) by ([@alcroito](https://github.com//alcroito))
- Add statistics collection - ([822a0fa](https://github.com/alcroito/digitalocean-dyndns/commit/822a0fa6617e0edecfa80045f05578b0ac4ae6cd)) by ([@alcroito](https://github.com//alcroito))

### 🐛 Fixes

- *(deps)* Axum 0.6.13 does not allow nesting fallback routers under / - ([2e3af64](https://github.com/alcroito/digitalocean-dyndns/commit/2e3af64233b03cf1a9365bc5ad2935dff3760922)) by ([@alcroito](https://github.com//alcroito))
- *(web)* Fix web ui not fetching data on macOS sometimes - ([94d4175](https://github.com/alcroito/digitalocean-dyndns/commit/94d4175464f9da56c9ec0f03df296c9e456a061e)) by ([@alcroito](https://github.com//alcroito))
- Enable webapp pre-rendering - ([1d6cc5e](https://github.com/alcroito/digitalocean-dyndns/commit/1d6cc5e4aed371e0d12b0ddcde5543f1c9766319)) by ([@alcroito](https://github.com//alcroito)) in [#38](https://github.com/alcroito/digitalocean-dyndns/pull/38)
- Improve web app design - ([5eab22d](https://github.com/alcroito/digitalocean-dyndns/commit/5eab22d9e1b5411070a22dac355e0e17bd653506)) by ([@alcroito](https://github.com//alcroito))

### 🚜 Refactor

- *(config)* [**breaking**] Use clap derive API and figment for config parsing - ([f449fdd](https://github.com/alcroito/digitalocean-dyndns/commit/f449fdd05604b7ea03b4e29492ae9ac46c7a332a)) by ([@alcroito](https://github.com//alcroito))
  - **BREAKING CHANGE:** toml booleans and numbers in the config files
can't be quoted anymore. Previously they had to be quoted strings due
to implementation issues.

- *(config)* Use std::time::Duration for update intervals - ([6146957](https://github.com/alcroito/digitalocean-dyndns/commit/6146957c1f10917577d9e03b0198485f595482b7)) by ([@alcroito](https://github.com//alcroito))
- *(tests)* Install color_eyre when running tests - ([92aa353](https://github.com/alcroito/digitalocean-dyndns/commit/92aa353b4f1158924e184ecad924afcc835a7ce7)) by ([@alcroito](https://github.com//alcroito)) in [#41](https://github.com/alcroito/digitalocean-dyndns/pull/41)

### ⚙️ Miscellaneous Tasks

- *(web)* Regenerate open api zodius client - ([3c257f3](https://github.com/alcroito/digitalocean-dyndns/commit/3c257f30954777e8eb51827f4559a048fef66501)) by ([@alcroito](https://github.com//alcroito)) in [#43](https://github.com/alcroito/digitalocean-dyndns/pull/43)
- Don't run CI twice when opening pull request - ([36b71f9](https://github.com/alcroito/digitalocean-dyndns/commit/36b71f9d7a370022183892193e97be02008cd8e1)) by ([@alcroito](https://github.com//alcroito)) in [#60](https://github.com/alcroito/digitalocean-dyndns/pull/60)
- Bump versions of github actions - ([1deaec8](https://github.com/alcroito/digitalocean-dyndns/commit/1deaec8724c926246fbeb22e90b7f3c863173b07)) by ([@alcroito](https://github.com//alcroito)) in [#48](https://github.com/alcroito/digitalocean-dyndns/pull/48)
- Fix uploading different feature artifacts to same path - ([f8b824b](https://github.com/alcroito/digitalocean-dyndns/commit/f8b824b665062fb73a84320319cc95dfcbda35dc)) by ([@alcroito](https://github.com//alcroito)) in [#47](https://github.com/alcroito/digitalocean-dyndns/pull/47)
- Silence clippy about loop that never loops - ([f512f92](https://github.com/alcroito/digitalocean-dyndns/commit/f512f922c4acb04e6926ac25367114f21e56c38a)) by ([@alcroito](https://github.com//alcroito))
- Fix typo everywhere from zodius to zodios - ([6615940](https://github.com/alcroito/digitalocean-dyndns/commit/66159409553a9c12be44f3f93565d3ddcb90a7d4)) by ([@alcroito](https://github.com//alcroito)) in [#44](https://github.com/alcroito/digitalocean-dyndns/pull/44)
- Bump crate and web package version numbers - ([2c36584](https://github.com/alcroito/digitalocean-dyndns/commit/2c3658459b1d7c53671346e05ecc92ca64a7f5fe)) by ([@alcroito](https://github.com//alcroito))
- Bump edition to 2021 and package resolver to version 2 - ([a805e67](https://github.com/alcroito/digitalocean-dyndns/commit/a805e671c1c9c552381753afb3e0f1cd3cd2da54)) by ([@alcroito](https://github.com//alcroito))
- Format Cargo.toml using a VSCode extension - ([a24dd25](https://github.com/alcroito/digitalocean-dyndns/commit/a24dd25e1574af5783663869986430ef2c642d81)) by ([@alcroito](https://github.com//alcroito))
- Allow debugging test failures using tmate - ([3f57a4e](https://github.com/alcroito/digitalocean-dyndns/commit/3f57a4e1424d150edcb9321256f75ff6dae4d0f8)) by ([@alcroito](https://github.com//alcroito))
- Add full backtrace when tests fail - ([b75eb02](https://github.com/alcroito/digitalocean-dyndns/commit/b75eb02f657c75990e07466c8b93fad9bfdb6042)) by ([@alcroito](https://github.com//alcroito))
- Update npm dependencies - ([415009c](https://github.com/alcroito/digitalocean-dyndns/commit/415009cfa28f9cdac324c1b2f7552808d19aadf5)) by ([@alcroito](https://github.com//alcroito)) in [#39](https://github.com/alcroito/digitalocean-dyndns/pull/39)
- Fix clippy warning - ([7a33644](https://github.com/alcroito/digitalocean-dyndns/commit/7a33644f1fb9a58b91580aa66755e3d4bae5bc7b)) by ([@alcroito](https://github.com//alcroito))
- Update rust dependencies - ([0237422](https://github.com/alcroito/digitalocean-dyndns/commit/0237422edbcb6bd537d08eb36cdea7bed6fd4022)) by ([@alcroito](https://github.com//alcroito))
- Bump version - ([db3f08b](https://github.com/alcroito/digitalocean-dyndns/commit/db3f08b6d63714e7d7d4069c0917fd678e225af9)) by ([@alcroito](https://github.com//alcroito))
- Remove TODO.md - ([4665bd3](https://github.com/alcroito/digitalocean-dyndns/commit/4665bd346092623f00f812761a22903e7c119ec7)) by ([@alcroito](https://github.com//alcroito))
- Add web app building in docker and CI - ([ab898dc](https://github.com/alcroito/digitalocean-dyndns/commit/ab898dcfcc54aef892193bcf9b488b12f2591635)) by ([@alcroito](https://github.com//alcroito)) in [#37](https://github.com/alcroito/digitalocean-dyndns/pull/37)
- Add web packages that will be used for the front end - ([6c38c79](https://github.com/alcroito/digitalocean-dyndns/commit/6c38c79a1b6adae8bf514b81ceb631a2c5f25f21)) by ([@alcroito](https://github.com//alcroito))
- Add svelte frontend client auto-generated code - ([915ee11](https://github.com/alcroito/digitalocean-dyndns/commit/915ee11cdedd8fcc9d71bb59ab36eb8068db9678)) by ([@alcroito](https://github.com//alcroito))
- Enable stats feature - ([71dac58](https://github.com/alcroito/digitalocean-dyndns/commit/71dac58905b3d32b2486277cefffdffd57d70d46)) by ([@alcroito](https://github.com//alcroito))
- Bump dependencies - ([9eb483b](https://github.com/alcroito/digitalocean-dyndns/commit/9eb483b9f565e1a47e0ac978594080f4939370e7)) by ([@alcroito](https://github.com//alcroito)) in [#36](https://github.com/alcroito/digitalocean-dyndns/pull/36)

### Build

- Add simple devcontainer - ([11c8a6c](https://github.com/alcroito/digitalocean-dyndns/commit/11c8a6c1bc8e10f4120aa40095de59632a53d34f)) by ([@alcroito](https://github.com//alcroito)) in [#46](https://github.com/alcroito/digitalocean-dyndns/pull/46)
- Use nextest cargo test runner in CI - ([f7fb656](https://github.com/alcroito/digitalocean-dyndns/commit/f7fb6560bd55d9f583230f5d08c851e0c4175ddc)) by ([@alcroito](https://github.com//alcroito))



<!-- generated by git-cliff -->
