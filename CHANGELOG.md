# Changelog

## [0.10.0](https://github.com/lobinuxsoft/capydeploy/compare/v0.9.0...v0.10.0) (2026-02-19)


### Features

* add protocol version negotiation to handshake ([0eed92c](https://github.com/lobinuxsoft/capydeploy/commit/0eed92c31db91ccc4671b95a9e0a1b811549238b)), closes [#196](https://github.com/lobinuxsoft/capydeploy/issues/196)
* bundle libwebkit2gtk-4.1 and runtime deps in AppImages ([414a0e6](https://github.com/lobinuxsoft/capydeploy/commit/414a0e6877f9963cdbe1035045687ae782791214)), closes [#194](https://github.com/lobinuxsoft/capydeploy/issues/194)
* protocol version negotiation ([#196](https://github.com/lobinuxsoft/capydeploy/issues/196)) ([f7a5944](https://github.com/lobinuxsoft/capydeploy/commit/f7a594461d0c44bd4114d2efc95641cf1eec82af))
* wire game-log crate to Tauri agent with play button and log capture ([61627ec](https://github.com/lobinuxsoft/capydeploy/commit/61627ecb04486177dace7319d6ddbad6160d844c))
* wire game-log crate to Tauri agent with play button and log capture ([7834ec3](https://github.com/lobinuxsoft/capydeploy/commit/7834ec391f1dca2482f94b6bf39d687690318908)), closes [#192](https://github.com/lobinuxsoft/capydeploy/issues/192)


### Bug Fixes

* **agent:** remove system tray to fix SteamOS/AppImage compatibility ([f6f9c79](https://github.com/lobinuxsoft/capydeploy/commit/f6f9c7996c2b389a37defc37a5dfab1261f84a54))
* **agent:** remove system tray to fix SteamOS/AppImage compatibility ([490e39f](https://github.com/lobinuxsoft/capydeploy/commit/490e39f3168ecf3a8afe314bce8d36fa831007bd)), closes [#193](https://github.com/lobinuxsoft/capydeploy/issues/193)
* apply rustfmt formatting for CI compatibility ([d370a77](https://github.com/lobinuxsoft/capydeploy/commit/d370a77056a511aef9deed41f25c68fa38bbb4b2))
* apply rustfmt formatting for CI compatibility ([38daad3](https://github.com/lobinuxsoft/capydeploy/commit/38daad33f3f07470ca84fb4308df0d384affddcf))
* binary-patch webkit2gtk hardcoded helper paths ([04d7fb0](https://github.com/lobinuxsoft/capydeploy/commit/04d7fb09d68c6b0669e6f8983dd309baf9daf275))
* correct desktop Exec field and WebKit helper detection ([3d752ea](https://github.com/lobinuxsoft/capydeploy/commit/3d752ea467648e461d84eca151c639fbe31e5a3a))
* guard gdk-pixbuf-query-loaders and add missing CI dep ([aa36209](https://github.com/lobinuxsoft/capydeploy/commit/aa362094b9e8abb509b26b3c7eb94dd5ebe4126d))
* **hub:** generate UUID for new game setups to prevent wrong deploy ([7ee091d](https://github.com/lobinuxsoft/capydeploy/commit/7ee091d09e08dd5eea508089a351e2b79ef2a761))
* **hub:** generate UUID for new game setups to prevent wrong deploy ([dfab393](https://github.com/lobinuxsoft/capydeploy/commit/dfab3934b8f1ef64a8509f32bdd30337df31d79b)), closes [#197](https://github.com/lobinuxsoft/capydeploy/issues/197)
* remove linuxdeploy AppRun symlink before writing custom AppRun ([b668a88](https://github.com/lobinuxsoft/capydeploy/commit/b668a881e67d6fc7cefe8010c266c0644b605294))
* **telemetry:** implement Windows CPU, memory, and battery metrics ([28590d9](https://github.com/lobinuxsoft/capydeploy/commit/28590d9b8d0ad7a3c5824ab32af59da702a90cc9))
* **telemetry:** implement Windows CPU, memory, and battery metrics ([17d8b05](https://github.com/lobinuxsoft/capydeploy/commit/17d8b059f25f07e3135652ecdf60f6fa99379595)), closes [#191](https://github.com/lobinuxsoft/capydeploy/issues/191)

## [0.9.0](https://github.com/lobinuxsoft/capydeploy/compare/v0.8.3...v0.9.0) (2026-02-19)


### Features

* **docs:** mostrar versión del release en las cards de descarga ([7ef4d8c](https://github.com/lobinuxsoft/capydeploy/commit/7ef4d8c0de1694bdb5a1e38cf83abd0b4067bbee)), closes [#188](https://github.com/lobinuxsoft/capydeploy/issues/188)
* **docs:** show release version in download cards ([8120277](https://github.com/lobinuxsoft/capydeploy/commit/8120277dcd7f24ce899978e266a3e4fc7f1dcf40))
* **docs:** show release version in download cards ([3406cf6](https://github.com/lobinuxsoft/capydeploy/commit/3406cf63b1bed2b372bd9c447b6286f000ec3d35))

## [0.8.3](https://github.com/lobinuxsoft/capydeploy/compare/v0.8.2...v0.8.3) (2026-02-19)


### Bug Fixes

* **ci:** use native OS runners for Windows builds (MSVC) ([48a9c8d](https://github.com/lobinuxsoft/capydeploy/commit/48a9c8d6d938b2d4bd0c8d595e90377fd0795d66))
* **ci:** use native OS runners for Windows builds (MSVC) ([bcda324](https://github.com/lobinuxsoft/capydeploy/commit/bcda3242c7b6b82cdec0c9500dc972777cfb2879)), closes [#173](https://github.com/lobinuxsoft/capydeploy/issues/173)
* **icons:** reemplazar iconos placeholder por mascota real ([96da7da](https://github.com/lobinuxsoft/capydeploy/commit/96da7dab6c56445e8a6d8e0ce0d9196efaa6414e)), closes [#169](https://github.com/lobinuxsoft/capydeploy/issues/169)
* **icons:** replace placeholder icons with actual mascot ([95fc1e3](https://github.com/lobinuxsoft/capydeploy/commit/95fc1e3e9a628bbb790295281bdce804c2755c86))


### Refactoring

* **agent:** split handler.rs god object into domain handlers ([5d3fb11](https://github.com/lobinuxsoft/capydeploy/commit/5d3fb11f8fd93f7fc249ce50b0868ed013e25fe6)), closes [#145](https://github.com/lobinuxsoft/capydeploy/issues/145)
* **decky:** split ws_server.py into focused handler modules ([b67d082](https://github.com/lobinuxsoft/capydeploy/commit/b67d08207f8c25750fc5d667d6b99cb246fdd2c7))
* **hub:** split ArtworkSelector.svelte into focused sub-components ([e354dc0](https://github.com/lobinuxsoft/capydeploy/commit/e354dc0bea1b4aab494a11b36a487279a6741481)), closes [#147](https://github.com/lobinuxsoft/capydeploy/issues/147)
* **hub:** split connection manager into focused modules ([0f2a565](https://github.com/lobinuxsoft/capydeploy/commit/0f2a565ab7fad0ef43f2ebea5de5774567efb78c))
* **hub:** split ws_client.rs into focused modules ([c3afb94](https://github.com/lobinuxsoft/capydeploy/commit/c3afb947c2f70748444b32364b73f1c939d68408))
* split monolithic code across codebase ([#144](https://github.com/lobinuxsoft/capydeploy/issues/144)) ([406133e](https://github.com/lobinuxsoft/capydeploy/commit/406133e5e5d4ec12557d5e7891777d973ec95238))


### Documentation

* fix critical docs issues and improve install guide ([bf7b6c8](https://github.com/lobinuxsoft/capydeploy/commit/bf7b6c893bf0360d4fae1e458f08f378ddcc6872))
* fix Decky URL, add missing API types, and build deps ([15c98e9](https://github.com/lobinuxsoft/capydeploy/commit/15c98e9703fbe5582dd203a303e4218d4f686d09))
* fix license, timing constants, and API response types ([77d1643](https://github.com/lobinuxsoft/capydeploy/commit/77d16439b8dbeeac54b00744906832b63e19fc0f))
* **install:** mejorar guía de instalación para usuarios no técnicos ([2a06e3a](https://github.com/lobinuxsoft/capydeploy/commit/2a06e3aa76b728987e9ef3095277c231c2a4b753))
* **site:** agregar Cloudflare Web Analytics ([f5bbf17](https://github.com/lobinuxsoft/capydeploy/commit/f5bbf1755ffb3b595da3a1cae089d7e80b2f4666))
* **site:** agregar Cloudflare Web Analytics al sitio de documentación ([63df843](https://github.com/lobinuxsoft/capydeploy/commit/63df8430dae466f4a0ec3dd566468705ec16a68f))

## [0.8.2](https://github.com/lobinuxsoft/capydeploy/compare/v0.8.1...v0.8.2) (2026-02-18)


### Bug Fixes

* sync app versions with release-please VERSION file ([cbd3535](https://github.com/lobinuxsoft/capydeploy/commit/cbd3535c818be24329c818f264f3bdc7acde96a1))
* sync app versions with release-please VERSION file ([36cc61e](https://github.com/lobinuxsoft/capydeploy/commit/36cc61e3c50758fd9493db41b5c335ba3dda5f05))
* sync app versions with release-please VERSION file ([f0ab61e](https://github.com/lobinuxsoft/capydeploy/commit/f0ab61e40184b920c8e3477476986d0b0a020501)), closes [#165](https://github.com/lobinuxsoft/capydeploy/issues/165)

## [0.8.1](https://github.com/lobinuxsoft/capydeploy/compare/v0.8.0...v0.8.1) (2026-02-17)


### Bug Fixes

* **hub:** show agent's install path instead of local Hub path ([ef177ab](https://github.com/lobinuxsoft/capydeploy/commit/ef177ab89639d44f53df8cdbf0dc98c565ce3891))
* **hub:** show agent's install path instead of local Hub path ([ed4d210](https://github.com/lobinuxsoft/capydeploy/commit/ed4d2102b2deae55e2562cae2edeb296107f3524))
* **hub:** show agent's install path instead of local Hub path ([9207669](https://github.com/lobinuxsoft/capydeploy/commit/920766946ff48ad19fb579ee084f828a31d744ea))

## [0.8.0](https://github.com/lobinuxsoft/capydeploy/compare/v0.7.0...v0.8.0) (2026-02-17)


### Features

* **decky:** add full lockout UX for pairing rate limiting ([47851f9](https://github.com/lobinuxsoft/capydeploy/commit/47851f98bf219dbf3b673716a96039fa850ce0f4)), closes [#154](https://github.com/lobinuxsoft/capydeploy/issues/154)
* **ui:** auto-close Decky pairing modal on success ([19bdee7](https://github.com/lobinuxsoft/capydeploy/commit/19bdee77ffbb004b36cd078d5dc27581099bcc9b))


### Bug Fixes

* **security:** add rate limiting to Decky pairing brute-force ([a387163](https://github.com/lobinuxsoft/capydeploy/commit/a387163c6168f73c9db646ffc355d6e2d69835d8))
* **security:** add rate limiting to Decky pairing brute-force ([9f155c9](https://github.com/lobinuxsoft/capydeploy/commit/9f155c98cfb7bb86f4590bed1e840b17afec4699)), closes [#154](https://github.com/lobinuxsoft/capydeploy/issues/154)
* **security:** enforce SSL cert verification in Decky artwork downloads ([005fff3](https://github.com/lobinuxsoft/capydeploy/commit/005fff3c5b8ec5b45ffbd5df14cbdc309cc99721)), closes [#152](https://github.com/lobinuxsoft/capydeploy/issues/152)
* **security:** prevenir path traversal en uploads de archivos ([#155](https://github.com/lobinuxsoft/capydeploy/issues/155)) ([d4e8c6f](https://github.com/lobinuxsoft/capydeploy/commit/d4e8c6fdc68606763b588355a4b6bd5f6199a9c3)), closes [#150](https://github.com/lobinuxsoft/capydeploy/issues/150)
* **security:** prevent SSRF in Decky artwork downloads ([cae026d](https://github.com/lobinuxsoft/capydeploy/commit/cae026dce60c7e5cfddd17170a732a2b0c0eee82))
* **security:** prevent SSRF in Decky artwork downloads ([2b32894](https://github.com/lobinuxsoft/capydeploy/commit/2b32894464513f6ee9111e08cae52802808d89b9)), closes [#151](https://github.com/lobinuxsoft/capydeploy/issues/151)
* **security:** use cryptographic PRNG for Decky pairing ([9a758bb](https://github.com/lobinuxsoft/capydeploy/commit/9a758bb40102cfde6b7337e2ae8ccf402fa20c68))
* **security:** use cryptographic PRNG for Decky pairing ([8cee117](https://github.com/lobinuxsoft/capydeploy/commit/8cee11757678e320aae5825609cf15410ec6807b)), closes [#153](https://github.com/lobinuxsoft/capydeploy/issues/153)

## [0.7.0](https://github.com/lobinuxsoft/capydeploy/compare/v0.6.0...v0.7.0) (2026-02-16)


### Features

* add agent WebSocket server crate ([#114](https://github.com/lobinuxsoft/capydeploy/issues/114)) ([60c44c5](https://github.com/lobinuxsoft/capydeploy/commit/60c44c5b6e3806fd117b50047a58c2354e3317fb))
* add build_all.sh and build_all.bat for Tauri + Decky builds ([4952e0f](https://github.com/lobinuxsoft/capydeploy/commit/4952e0f3e36c4037a67b64e0da98118e563edb69))
* add Rust workspace with Phase 1 core crates ([#40](https://github.com/lobinuxsoft/capydeploy/issues/40)) ([b77f8ce](https://github.com/lobinuxsoft/capydeploy/commit/b77f8ceaef0301061b62e353199a6ef6e3577cea))
* **agent:** port desktop Agent to Tauri v2 with flexible telemetry ([a9dfedc](https://github.com/lobinuxsoft/capydeploy/commit/a9dfedcd56e3b4f5d7c53f37b9184df1a21511a2))
* assemble Rust agent desktop binary ([#122](https://github.com/lobinuxsoft/capydeploy/issues/122)) ([3d3db22](https://github.com/lobinuxsoft/capydeploy/commit/3d3db22e36142f59985a12cdb97a185ded3ebf1e))
* **hub-console-log:** implementar state management de console log del Hub ([#128](https://github.com/lobinuxsoft/capydeploy/issues/128)) ([d6a55b8](https://github.com/lobinuxsoft/capydeploy/commit/d6a55b8be307a403847bdb4d8a28e66896f0b82c))
* **hub-deploy:** implementar flujo de deploy de juegos del Hub ([#125](https://github.com/lobinuxsoft/capydeploy/issues/125)) ([9c194a3](https://github.com/lobinuxsoft/capydeploy/commit/9c194a300a7e8557b56b4d25e0c84f9aed0c9f73))
* **hub-games:** implementar gestión de juegos instalados del Hub ([#126](https://github.com/lobinuxsoft/capydeploy/issues/126)) ([1f32afe](https://github.com/lobinuxsoft/capydeploy/commit/1f32afe699d3c24c1b0dfca0fb5e6b6fd787cb6e))
* **hub-settings:** implementar toast queue del Hub ([#129](https://github.com/lobinuxsoft/capydeploy/issues/129)) ([5bf7b13](https://github.com/lobinuxsoft/capydeploy/commit/5bf7b13510a0892632c6c8bb082639d586f71b33))
* **hub-telemetry:** implementar state management de telemetría del Hub ([#127](https://github.com/lobinuxsoft/capydeploy/issues/127)) ([0bcb2ca](https://github.com/lobinuxsoft/capydeploy/commit/0bcb2ca4b8a84ccb7b7605860c05eed0349da58c))
* **hub-widgets:** implementar widgets canvas para dashboard de telemetría ([#130](https://github.com/lobinuxsoft/capydeploy/issues/130)) ([dd789d6](https://github.com/lobinuxsoft/capydeploy/commit/dd789d6d48f66061e9f127a6aec95d8ad8929014))
* **hub:** add connection header bar and Hub unit tests ([#131](https://github.com/lobinuxsoft/capydeploy/issues/131)) ([0665f94](https://github.com/lobinuxsoft/capydeploy/commit/0665f941c91f633c9485a2854fbdddaa7bfae789))
* **hub:** add deploy view with game setup CRUD and upload ([#131](https://github.com/lobinuxsoft/capydeploy/issues/131)) ([0e17e77](https://github.com/lobinuxsoft/capydeploy/commit/0e17e77f8298c4a078bc2997f42160840bec0d25))
* **hub:** add game artwork edit, source filter, about section, and pagination ([#131](https://github.com/lobinuxsoft/capydeploy/issues/131)) ([4188b6e](https://github.com/lobinuxsoft/capydeploy/commit/4188b6ed4bde39b7c94f0e39f37286f9d7a051c3))
* **hub:** add installed games view and settings page ([#131](https://github.com/lobinuxsoft/capydeploy/issues/131)) ([76bf0b3](https://github.com/lobinuxsoft/capydeploy/commit/76bf0b323e261c8380d4d80625b51264c26abe4e))
* **hub:** add native folder picker dialogs for local path and log dir ([#131](https://github.com/lobinuxsoft/capydeploy/issues/131)) ([f95d07c](https://github.com/lobinuxsoft/capydeploy/commit/f95d07c88c40887a5c5c14cf7552ff3a143f86ec))
* **hub:** add secure API key input, discovery refresh, and console source badges ([#131](https://github.com/lobinuxsoft/capydeploy/issues/131)) ([51ac691](https://github.com/lobinuxsoft/capydeploy/commit/51ac6915b20cfdad698f9df85aa7cc07bd33511d))
* **hub:** add toast notifications and artwork selector ([#131](https://github.com/lobinuxsoft/capydeploy/issues/131)) ([9c2a269](https://github.com/lobinuxsoft/capydeploy/commit/9c2a2693d99c44d2991ad24828bbe0226a4818a9))
* **hub:** agregar toggle de telemetría y corregir deserialización de SteamUser ([#131](https://github.com/lobinuxsoft/capydeploy/issues/131)) ([a4063c1](https://github.com/lobinuxsoft/capydeploy/commit/a4063c163e3b9f4f288d4b1e801fb193dec97faf))
* **hub:** integrate console log viewer with level filters ([#131](https://github.com/lobinuxsoft/capydeploy/issues/131)) ([2e09f8a](https://github.com/lobinuxsoft/capydeploy/commit/2e09f8a2219abfe96cd2dd0c526716c9c4d880e1))
* **hub:** integrate telemetry dashboard with canvas widgets ([#131](https://github.com/lobinuxsoft/capydeploy/issues/131)) ([ee5763e](https://github.com/lobinuxsoft/capydeploy/commit/ee5763ea14e4eb7f13668f884662aac98a630452))
* **hub:** port Hub to Tauri v2, replace libcosmic/hub-rs with Svelte WebView ([f4054eb](https://github.com/lobinuxsoft/capydeploy/commit/f4054eb9c137e50d12859e431627052bb86b7356))
* **hub:** scaffold Hub binary with send_binary support ([#131](https://github.com/lobinuxsoft/capydeploy/issues/131)) ([e84dedb](https://github.com/lobinuxsoft/capydeploy/commit/e84dedba0d3d18f8801c741f5406c9384768c772))
* **hub:** virtual scrolling en console log, filtros artwork y thumbnails retry ([#140](https://github.com/lobinuxsoft/capydeploy/issues/140)) ([4916f5a](https://github.com/lobinuxsoft/capydeploy/commit/4916f5aa3f9aa98adf135f29285eca2b3a993aed))
* **hub:** wire connection bridge, devices view, and pairing dialog ([#131](https://github.com/lobinuxsoft/capydeploy/issues/131)) ([7dba700](https://github.com/lobinuxsoft/capydeploy/commit/7dba700491f83e1283b5919e65c7b5c51ad1d49e))
* **hub:** wire deploy progress stream with real-time UI updates ([#131](https://github.com/lobinuxsoft/capydeploy/issues/131)) ([13dbdf8](https://github.com/lobinuxsoft/capydeploy/commit/13dbdf85d29e0b421b209de6b5fb3ba858253c6f))
* implement Rust agent file operations ([#120](https://github.com/lobinuxsoft/capydeploy/issues/120)) ([b000a4a](https://github.com/lobinuxsoft/capydeploy/commit/b000a4a65089fc3c676aabb0d72f633a5423d4a7))
* implement Rust agent system tray ([#121](https://github.com/lobinuxsoft/capydeploy/issues/121)) ([4fbdf89](https://github.com/lobinuxsoft/capydeploy/commit/4fbdf89495f3b2d89138841dc306116e91838442))
* implement Rust CDP console log collector ([#118](https://github.com/lobinuxsoft/capydeploy/issues/118)) ([8b4e716](https://github.com/lobinuxsoft/capydeploy/commit/8b4e7169726131e78e1be2fe534f4f9e21fd52b1))
* implement Rust CEF/CDP client ([#116](https://github.com/lobinuxsoft/capydeploy/issues/116)) ([abac1f6](https://github.com/lobinuxsoft/capydeploy/commit/abac1f6f1c6d1a1dd680831b82bebec8381809c0))
* implement Rust game log wrapper ([#119](https://github.com/lobinuxsoft/capydeploy/issues/119)) ([1f1defb](https://github.com/lobinuxsoft/capydeploy/commit/1f1defbf518675f389837c70ee935e19cb67d3fe))
* implement Rust hardware telemetry collector ([#117](https://github.com/lobinuxsoft/capydeploy/issues/117)) ([7155399](https://github.com/lobinuxsoft/capydeploy/commit/71553994e6655abcd46a236939049289799c0c60))
* implement Rust Steam controller ([#115](https://github.com/lobinuxsoft/capydeploy/issues/115)) ([af73ef4](https://github.com/lobinuxsoft/capydeploy/commit/af73ef4e0a86ce49c3d8a7a13286c97fb19c6ae1))
* implementar connection manager del Hub ([#124](https://github.com/lobinuxsoft/capydeploy/issues/124)) ([f8e6959](https://github.com/lobinuxsoft/capydeploy/commit/f8e69597a9e8c9d96f644cc925ece2c309f700b5))
* port cliente SteamGridDB API a Rust ([#123](https://github.com/lobinuxsoft/capydeploy/issues/123)) ([84d87f3](https://github.com/lobinuxsoft/capydeploy/commit/84d87f30fa7a404348bc398d6ce626a9f896b7ad))
* port transfer crate to Rust ([#113](https://github.com/lobinuxsoft/capydeploy/issues/113)) ([7d4a8de](https://github.com/lobinuxsoft/capydeploy/commit/7d4a8deba103fe29182a6933646bb2f613bd38b1))


### Bug Fixes

* **agent:** notify Hub of telemetry/console-log status via WS and move EventsOn to always-mounted page ([ae07a3a](https://github.com/lobinuxsoft/capydeploy/commit/ae07a3a32913906bae9c24b1efea686701908072))
* **ci:** resolve clippy collapsible_if and needless_borrow (Rust 1.93) ([7b574ea](https://github.com/lobinuxsoft/capydeploy/commit/7b574ea48db46bbd22209314fee87919c2e155fb))
* **ci:** resolve lint, typecheck and Windows build failures ([dafce34](https://github.com/lobinuxsoft/capydeploy/commit/dafce34689cc60ca7d6ee259b2eb7151210b4bce))
* **ci:** resolve remaining typecheck and clippy failures ([63acf7a](https://github.com/lobinuxsoft/capydeploy/commit/63acf7a60a7c40ff74368dcccdc594bddfbc1839))
* **ci:** type agent invoke calls and derive ServerConfig Default ([8298a3f](https://github.com/lobinuxsoft/capydeploy/commit/8298a3f4f44f2c1f7b94b6bfd0be33bb5226ab33))
* **hub:** clean up TelemetryHub/ConsoleLogHub on agent disconnect and use VecDeque for console-log buffer ([a45208d](https://github.com/lobinuxsoft/capydeploy/commit/a45208dff4b04121d63d636ffcfd22159b7eb24d))
* **hub:** corregir reconexión WS, pong detection y gestión de estados ([#136](https://github.com/lobinuxsoft/capydeploy/issues/136)) ([bc866f1](https://github.com/lobinuxsoft/capydeploy/commit/bc866f16deed3537b7fca5daf4e8a09b4148f3fe))
* **hub:** corregir telemetría, header bar y deserialización de protocolo ([#131](https://github.com/lobinuxsoft/capydeploy/issues/131)) ([aa0e8e0](https://github.com/lobinuxsoft/capydeploy/commit/aa0e8e046e1222089c0f9eda0cf2ba0c9488a151))
* **hub:** quitar toggle de console log y detectar telemetría stale ([#136](https://github.com/lobinuxsoft/capydeploy/issues/136)) ([486dd11](https://github.com/lobinuxsoft/capydeploy/commit/486dd1134ef1c11b38b923a1ab377535b05ae924))
* **hub:** resolver panics de tokio y spam de mDNS ([#131](https://github.com/lobinuxsoft/capydeploy/issues/131)) ([6dfe34f](https://github.com/lobinuxsoft/capydeploy/commit/6dfe34fcbe9a4d3a5f9e30fb3991716fdf7e8d55))
* **hub:** telemetría agrupada, deploy protocol y panic en tokio tasks ([#138](https://github.com/lobinuxsoft/capydeploy/issues/138)) ([7b8cc52](https://github.com/lobinuxsoft/capydeploy/commit/7b8cc52087e9d625138fbf036283dec132c83366))
* **hub:** telemetría paridad visual con Svelte, layout 2 columnas y build.sh ([#138](https://github.com/lobinuxsoft/capydeploy/issues/138)) ([d24fe42](https://github.com/lobinuxsoft/capydeploy/commit/d24fe42fb009e61cdb282a9ba43c2087ce47c769))


### Refactoring

* flatten Rust workspace from rust/ subdirectory to project root ([4d9ce81](https://github.com/lobinuxsoft/capydeploy/commit/4d9ce81840b0440148363246841bcea2cf83c9e8))
* **hub:** overhaul visual, telemetría sin sparklines, artwork modular y WS reconnect ([#131](https://github.com/lobinuxsoft/capydeploy/issues/131)) ([7b61bc6](https://github.com/lobinuxsoft/capydeploy/commit/7b61bc6b5d854f1449aa3f8e36ddd5721f76a11d))
* **hub:** reemplazar canvas gauges con progress_bar y preparar sparklines consolidados ([#131](https://github.com/lobinuxsoft/capydeploy/issues/131)) ([55ab77f](https://github.com/lobinuxsoft/capydeploy/commit/55ab77fd612cb2d2cbef7d08b91375540a5c9bab))
* port entire codebase from Go/Wails to Rust/Tauri ([9b783d4](https://github.com/lobinuxsoft/capydeploy/commit/9b783d4d8a93c3e543e3dab297d9e15142c8d969))

## [0.6.0](https://github.com/lobinuxsoft/capydeploy/compare/v0.5.0...v0.6.0) (2026-02-12)


### Features

* add remote hardware telemetry streaming (Phase 1) ([51ff9ad](https://github.com/lobinuxsoft/capydeploy/commit/51ff9adf72332ae4c350db288b4c9df3f02e5198)), closes [#48](https://github.com/lobinuxsoft/capydeploy/issues/48)
* completar telemetría de hardware remota (Phase 2) ([75e6131](https://github.com/lobinuxsoft/capydeploy/commit/75e6131a555320aa909c8238bc4f44be5248ed03))
* console log remota + game log wrapper + protocol extensions ([#107](https://github.com/lobinuxsoft/capydeploy/issues/107)) ([820fb59](https://github.com/lobinuxsoft/capydeploy/commit/820fb59326ae0e80cc2db65dcf5e64141310e8dc))
* telemetría de hardware remota completa ([#48](https://github.com/lobinuxsoft/capydeploy/issues/48)) ([045cd8e](https://github.com/lobinuxsoft/capydeploy/commit/045cd8e0cc0222922c142dad10cf314bb5b5f35f))


### Bug Fixes

* agregar stubs de VRAM, MCLK y swap para Windows ([85dbab3](https://github.com/lobinuxsoft/capydeploy/commit/85dbab3ad7e42e4897e2660d381d62aca378fa44))
* corregir escritura concurrente en WebSocket del Hub ([ee13a4b](https://github.com/lobinuxsoft/capydeploy/commit/ee13a4beb2ca4695b3717f15069bad542b1d9a4b))
* corregir memory leaks y optimizar telemetría ([2ac9142](https://github.com/lobinuxsoft/capydeploy/commit/2ac91429aad2181f1f0b5fc5a2581ed19ad1555a))


### Refactoring

* remove Decky plugin build from monorepo CI/CD ([#103](https://github.com/lobinuxsoft/capydeploy/issues/103)) ([42302c9](https://github.com/lobinuxsoft/capydeploy/commit/42302c96cd5b63ea278ff9236ba8d8997cefc22e))

## [0.5.0](https://github.com/lobinuxsoft/capydeploy/compare/v0.4.0...v0.5.0) (2026-02-09)


### Features

* extract Decky plugin to standalone repo as submodule ([#102](https://github.com/lobinuxsoft/capydeploy/issues/102)) ([a98e765](https://github.com/lobinuxsoft/capydeploy/commit/a98e76594e7eb4c3b2a4e77158bea6e36ba6e39c))


### Documentation

* reorganize download section by platform ([feb0d38](https://github.com/lobinuxsoft/capydeploy/commit/feb0d3885110e22afad914e5c255f8639a644e6b))
* reorganize download section by platform ([193631e](https://github.com/lobinuxsoft/capydeploy/commit/193631e231e0d0f9f28b64254519c6ee464b72a3))
* reorganize download section by platform ([5de803e](https://github.com/lobinuxsoft/capydeploy/commit/5de803efb7005ec40a0dbb62e0f6ef5b660020e0))
* switch commit language convention to English ([b6aa524](https://github.com/lobinuxsoft/capydeploy/commit/b6aa52424cb27d3b36a7ec128cc60fd1bcdbb87d))
* switch commit language convention to English ([050e7e5](https://github.com/lobinuxsoft/capydeploy/commit/050e7e5e05edd5b77f58bbe9b3b778a762ec88f2))
* switch commit language convention to English ([71acee9](https://github.com/lobinuxsoft/capydeploy/commit/71acee91e8101884303f29220b89bcfaaeb44334))

## [0.4.0](https://github.com/lobinuxsoft/capydeploy/compare/v0.3.0...v0.4.0) (2026-02-09)


### Features

* **hub:** permitir editar artwork de juegos instalados ([65c2b99](https://github.com/lobinuxsoft/capydeploy/commit/65c2b99bc27ddd6d8b51ea5748fe7fbf066b72c7))
* **hub:** permitir editar artwork de juegos instalados ([c1b0700](https://github.com/lobinuxsoft/capydeploy/commit/c1b0700cf4bb0a5b6411019b208fcc4dccb07cb7)), closes [#35](https://github.com/lobinuxsoft/capydeploy/issues/35)


### Bug Fixes

* **decky:** aplicar artwork via SteamClient API para visibilidad inmediata ([f382371](https://github.com/lobinuxsoft/capydeploy/commit/f3823716b72ff26c8a459fe93166b0a0b43c3e7b))
* **decky:** leer versión desde package.json en vez de hardcodear ([535a5c4](https://github.com/lobinuxsoft/capydeploy/commit/535a5c4ad284171803b5b3d0920799889c4828da))
* **decky:** leer versión desde package.json en vez de hardcodear ([eda86ce](https://github.com/lobinuxsoft/capydeploy/commit/eda86ce5d708bbbf190cf152e309f1049e30dcf9)), closes [#81](https://github.com/lobinuxsoft/capydeploy/issues/81)
* **decky:** mostrar juegos instalados sin conexión activa ([85f8eef](https://github.com/lobinuxsoft/capydeploy/commit/85f8eefd313838ec2fcb78a7c742f40586f2e232))
* **decky:** mostrar juegos instalados sin conexión activa ([def8f5a](https://github.com/lobinuxsoft/capydeploy/commit/def8f5a2ed4962845139c167769a2c823a2cb226)), closes [#82](https://github.com/lobinuxsoft/capydeploy/issues/82)


### Refactoring

* eliminar campo capabilities redundante del protocolo ([e9ec24d](https://github.com/lobinuxsoft/capydeploy/commit/e9ec24d2981557f5f2bf866f910b653dfb28525b))
* eliminar campo capabilities redundante del protocolo ([7013b79](https://github.com/lobinuxsoft/capydeploy/commit/7013b796ab222e61778a9925d627f21ca03fb009))


### Documentation

* separar flujo de descarga del build from source ([aae9ece](https://github.com/lobinuxsoft/capydeploy/commit/aae9eceaac97c2775a144973cd237f9a8ca53225))
* separar flujo de descarga del build from source ([c02bfc8](https://github.com/lobinuxsoft/capydeploy/commit/c02bfc898c8dbd6ba6672673d63b9f033a20dc3f))

## [0.3.0](https://github.com/lobinuxsoft/capydeploy/compare/v0.2.0...v0.3.0) (2026-02-09)


### Features

* **agent-desktop:** aplicar Proton automáticamente al crear shortcuts en Linux ([013aa7f](https://github.com/lobinuxsoft/capydeploy/commit/013aa7f1c4a82e96a6f9e98c4048015925bc3d2c))
* **agent-desktop:** migrar List() de VDF a CEF API con fallback ([f1e1003](https://github.com/lobinuxsoft/capydeploy/commit/f1e1003827799696d8d53e37f7dd8b6f262b8208))
* **build:** unificar versionado con archivo VERSION como fuente única ([0c7236d](https://github.com/lobinuxsoft/capydeploy/commit/0c7236d7395006c2e297f1fa52c81c217d04fd5c))
* **build:** unificar versionado con VERSION file + release-please ([b27d617](https://github.com/lobinuxsoft/capydeploy/commit/b27d617028f047054a464b5264f20c6d4fd47bbc))
* **ci:** integrar release-please para versionado automático ([6c4c3b9](https://github.com/lobinuxsoft/capydeploy/commit/6c4c3b98904456356ad3cbd39f3aca4486cb94b5))
* **decky:** agregar soporte de artwork local al agente Decky ([aed34da](https://github.com/lobinuxsoft/capydeploy/commit/aed34da6e9b9e539a26ce9a60727a535e45d67e7))
* **hub:** enviar imágenes locales al Agent para artwork ([faadf71](https://github.com/lobinuxsoft/capydeploy/commit/faadf712d2ff1d4d380165577d04607b85f38dea))
* **hub:** enviar imágenes locales al Agent para artwork ([a758de4](https://github.com/lobinuxsoft/capydeploy/commit/a758de431967e2d853428418d0763acf79f45ecd)), closes [#34](https://github.com/lobinuxsoft/capydeploy/issues/34)
* Proton automático, List() via CEF y fixes Decky ([640045a](https://github.com/lobinuxsoft/capydeploy/commit/640045a1580c7237df6d172eb76303283d020297))


### Bug Fixes

* **agent-decky:** aplicar Proton a .exe, reducir toasts y subir límite WS ([e3956f3](https://github.com/lobinuxsoft/capydeploy/commit/e3956f3ec8840eb77faaec94b754b06088646f15))
* **agent-desktop:** almacenar artwork pendiente y aplicar en CompleteUpload ([6233f35](https://github.com/lobinuxsoft/capydeploy/commit/6233f354a2f41e4babe3aa4d529c23de8dc6c3a1))
* **agent-desktop:** aplicar artwork y shortcuts via Steam CEF API ([d81f1fd](https://github.com/lobinuxsoft/capydeploy/commit/d81f1fd65f162cc228cfb941d3c841344f03fe89))
* **agent-desktop:** aplicar artwork y shortcuts via Steam CEF API ([4847b74](https://github.com/lobinuxsoft/capydeploy/commit/4847b74c05e2ec502f7d7720f568b35701d4ff49)), closes [#64](https://github.com/lobinuxsoft/capydeploy/issues/64)
* **agent-desktop:** aplicar Proton solo a ejecutables .exe en Linux ([2b2a398](https://github.com/lobinuxsoft/capydeploy/commit/2b2a3980a59368983e0f85ca71cbda1e2e8539d4))
* **agent-desktop:** eliminar función evaluate sin uso en CEF client ([f0de337](https://github.com/lobinuxsoft/capydeploy/commit/f0de3371eb687cafecab1abc72f74b96184688e3))
* **agent:** corregir memory leak CEF y eliminar duplicación ([4ba1a48](https://github.com/lobinuxsoft/capydeploy/commit/4ba1a48aebcc9f90dd4b09b8c692b4a29c4dbaec))
* **agent:** corregir memory leak CEF y eliminar duplicación de código ([24b0d92](https://github.com/lobinuxsoft/capydeploy/commit/24b0d92bf6e06744452d0a21041865fdbc5d1216)), closes [#67](https://github.com/lobinuxsoft/capydeploy/issues/67)
* **agent:** restaurar fallback VDF para listado de shortcuts ([516ebc0](https://github.com/lobinuxsoft/capydeploy/commit/516ebc0e135f60db654f645887dda94ce3eed26d))
* **ci:** cambiar secciones del changelog a inglés ([3d4a0a0](https://github.com/lobinuxsoft/capydeploy/commit/3d4a0a0e1d603bd97cd6d537e16e6eef3bc6371b))
* **ci:** cambiar secciones del changelog a inglés ([2cebc65](https://github.com/lobinuxsoft/capydeploy/commit/2cebc65269c08fa15a3de71d3c57af5585d229c9))
* **ci:** disparar release solo desde release-please ([4a71078](https://github.com/lobinuxsoft/capydeploy/commit/4a7107839d9bc1784a6fce7aafe67e933e93717f))
* **ci:** disparar release solo desde release-please, no desde tags manuales ([b05503d](https://github.com/lobinuxsoft/capydeploy/commit/b05503dfbce9c5927bd76b8576bdb3def530da55))
* **core:** eliminar código muerto y corregir memory leaks ([d08a7e1](https://github.com/lobinuxsoft/capydeploy/commit/d08a7e17257eb2e57e7cb6d390bb08892ec27ae2))
* **core:** eliminar código muerto y corregir memory leaks ([00ff248](https://github.com/lobinuxsoft/capydeploy/commit/00ff248a4b808187cec02ec3ae8cdbe37524e710)), closes [#67](https://github.com/lobinuxsoft/capydeploy/issues/67)
* **decky:** corregir flujo de artwork local para agente Decky ([45e15a2](https://github.com/lobinuxsoft/capydeploy/commit/45e15a2b8e739d1d9592df9999fc5cd0a564c42c))
* **hub:** corregir previews de artwork local en ArtworkSelector ([0b4e61d](https://github.com/lobinuxsoft/capydeploy/commit/0b4e61d7628daa1dd8fdb742b3ea88acb4511b3b)), closes [#34](https://github.com/lobinuxsoft/capydeploy/issues/34)


### Refactoring

* **agent:** eliminar dependencia de steam-shortcut-manager ([3b1a845](https://github.com/lobinuxsoft/capydeploy/commit/3b1a845ba95717e1f7ec9c9545d674cd908bb18d))
* **agent:** eliminar dependencia de steam-shortcut-manager ([4fec422](https://github.com/lobinuxsoft/capydeploy/commit/4fec4226a776c52c09419f85726bc04bfca1fbff)), closes [#39](https://github.com/lobinuxsoft/capydeploy/issues/39)
* **agent:** reemplazar CEF list por tracking en memoria estilo Decky ([9bba9db](https://github.com/lobinuxsoft/capydeploy/commit/9bba9dbe9ad1af1f04ec0692cf9d14d7f94afcec))
* **agent:** unificar tipos AuthorizedHub entre config y auth ([2f9c240](https://github.com/lobinuxsoft/capydeploy/commit/2f9c240e831c21d7fa1e4e4e8ac28b616c844263))
* **agent:** unificar tipos AuthorizedHub entre config y auth ([490816b](https://github.com/lobinuxsoft/capydeploy/commit/490816b2b67a090a93b8acccdf2983035e1974a8)), closes [#67](https://github.com/lobinuxsoft/capydeploy/issues/67)
* **core:** eliminar duplicados menores e implementar cleanup ([049a47f](https://github.com/lobinuxsoft/capydeploy/commit/049a47f67d988f95c6f166c12eebdef7fe61db44))
* **core:** eliminar duplicados menores e implementar cleanup de uploads ([6bf5829](https://github.com/lobinuxsoft/capydeploy/commit/6bf5829b496603368e70ac196226ae12bd51a6d2)), closes [#67](https://github.com/lobinuxsoft/capydeploy/issues/67)
* **hub:** descomponer god object app.go en archivos por dominio ([8cbcffe](https://github.com/lobinuxsoft/capydeploy/commit/8cbcffe70084b19db242e3d3864046e61a324991))
* **hub:** descomponer god object app.go en archivos por dominio ([715c04b](https://github.com/lobinuxsoft/capydeploy/commit/715c04bb74d2df93777ca547ea717713bef330f9)), closes [#67](https://github.com/lobinuxsoft/capydeploy/issues/67)
* **hub:** eliminar transporte HTTP legacy y código muerto ([49edf6b](https://github.com/lobinuxsoft/capydeploy/commit/49edf6bc74dbe450a1ac98bc25f35821950bf55b))
* **hub:** eliminar transporte HTTP legacy y código muerto ([06dece1](https://github.com/lobinuxsoft/capydeploy/commit/06dece11d661aa13d869e82d062ec8c30b220877)), closes [#67](https://github.com/lobinuxsoft/capydeploy/issues/67)


### Documentation

* actualización completa de documentación ([a55612b](https://github.com/lobinuxsoft/capydeploy/commit/a55612b421b5ae005b0d17d344c59c25c84dbf09))
* actualización completa de documentación ([bfde9b4](https://github.com/lobinuxsoft/capydeploy/commit/bfde9b4a103a32adcfb6c506676a7edaa1b39654))


### Tests

* agregar suite de tests automatizados ([ce0973f](https://github.com/lobinuxsoft/capydeploy/commit/ce0973fbef565da09ba7cd83f0b9a7d59c07b2df))
* agregar suite de tests para paquetes sin cobertura ([bdd0c0c](https://github.com/lobinuxsoft/capydeploy/commit/bdd0c0ceb6b0d7aa368788cb903c2b167d577f1d)), closes [#31](https://github.com/lobinuxsoft/capydeploy/issues/31)
* agregar tests para funciones de artwork local ([c144c8d](https://github.com/lobinuxsoft/capydeploy/commit/c144c8db476f117e26cc37a15c6d8a12d2adcb79))
