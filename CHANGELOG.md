# Changelog

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
