# [1.1.0](https://github.com/cyberpath-HQ/sentinel/compare/v1.0.6...v1.1.0) (2026-01-15)


### Bug Fixes

* Replace thread_rng with rng for nonce and salt generation in encryption and key derivation implementations ([b59fabd](https://github.com/cyberpath-HQ/sentinel/commit/b59fabd8cdb59194eb8db385771d38a9b4a24c27))


### Features

* Add accordion animations for smooth content transitions ([9f16764](https://github.com/cyberpath-HQ/sentinel/commit/9f16764bc243309be2eecb390c622c3ec5c8a31d))
* Add API routes for generating metadata JSON and Fuse.js configuration ([eb9ba6e](https://github.com/cyberpath-HQ/sentinel/commit/eb9ba6e4fd5cc41f6c92d472a849b74bf0f9b68a))
* Add BaseLayout and DocsLayout for improved documentation structure and navigation ([c4be089](https://github.com/cyberpath-HQ/sentinel/commit/c4be089b19805b4093d8dc589d9213e59b70fff5))
* Add CLI Commands and CLI Reference documentation ([df4e877](https://github.com/cyberpath-HQ/sentinel/commit/df4e877e6daf6bff968c332826113f1907277ef6))
* Add code component with clipboard copy functionality for improved documentation interactivity ([fada0fb](https://github.com/cyberpath-HQ/sentinel/commit/fada0fb68c4b4d9e7627b09daac361ac1898c95d))
* Add DocsMetadata and DocsMetadataCollection types for documentation structure ([705b5aa](https://github.com/cyberpath-HQ/sentinel/commit/705b5aaf7b562a7451956d597de00380e0b56c37))
* Add documentation collection schema for structured content management ([4a5d471](https://github.com/cyberpath-HQ/sentinel/commit/4a5d47176294ad498825433d9ee3ee908a85ebed))
* Add dynamic documentation pages and enhance homepage layout for Sentinel DBMS ([b5e3b7b](https://github.com/cyberpath-HQ/sentinel/commit/b5e3b7b085ef36a27cfe52b17b7fcc6c50dccdc1))
* Add fuse.js dependency for enhanced search functionality ([c9f66e1](https://github.com/cyberpath-HQ/sentinel/commit/c9f66e1c12b6ab2339dc578ee95a00cbf3c56295))
* Add margin to code block styling for improved spacing in documentation ([287c60e](https://github.com/cyberpath-HQ/sentinel/commit/287c60e9a2ef223f6a63892acd57067c20f28e99))
* Add MDX integration for enhanced documentation capabilities ([6dd6e65](https://github.com/cyberpath-HQ/sentinel/commit/6dd6e65b2e8da535a2687fe0f5aadeeabc594926))
* Add missing dependencies for enhanced documentation features ([03938af](https://github.com/cyberpath-HQ/sentinel/commit/03938af545ac45cc0d3820f6ef7a92ff5711d483))
* Add missing dependencies for enhanced documentation features ([b5dca60](https://github.com/cyberpath-HQ/sentinel/commit/b5dca60f41215b0babfdb92c417353e5f35d57ea))
* Add SiteHeader and SiteFooter components for improved documentation layout and navigation ([e513795](https://github.com/cyberpath-HQ/sentinel/commit/e513795165e52f2d00d703ac42a6e74ab0ad4fe0))
* Add Table of Contents component for documentation pages ([fcb5186](https://github.com/cyberpath-HQ/sentinel/commit/fcb518678a045f551a87677bfe7d2d83b0359e40))
* Adjust margin for Table of Contents list for improved layout ([25f73c5](https://github.com/cyberpath-HQ/sentinel/commit/25f73c565de89d8897566929fe1d1a2776b7e5ac))
* Clean up SidebarNavigation component by removing console log and formatting code ([9a0d916](https://github.com/cyberpath-HQ/sentinel/commit/9a0d9165466260e7a94e4537abb8ac7c7ded6d46))
* Enhance code styling with highlighted background and line numbering ([6f6dc18](https://github.com/cyberpath-HQ/sentinel/commit/6f6dc187d84f255977d03025e84491610728a85d))
* Enhance DocsLayout with Table of Contents, Search Modal, and Sidebar Navigation components ([60dc3f4](https://github.com/cyberpath-HQ/sentinel/commit/60dc3f4cb40e66f67373ea5143d182980ec3f355))
* Enhance Document and Collection structs with additional derive traits for improved functionality ([7eb8351](https://github.com/cyberpath-HQ/sentinel/commit/7eb8351b39232ba2d7f4c9bf389fa71c131de55c))
* Enhance global styles with new font settings and improved dark theme colors ([e286abe](https://github.com/cyberpath-HQ/sentinel/commit/e286abed41cb1305519f6681f571d9b783d7853b))
* Implement search modal with Fuse.js fuzzy search functionality ([4b0de50](https://github.com/cyberpath-HQ/sentinel/commit/4b0de504eceddf0a2d5aeb4a8088b061d0f8a5ca))
* Implement SidebarNavigation component with accordion functionality for improved document navigation ([36b72e6](https://github.com/cyberpath-HQ/sentinel/commit/36b72e6ef188dab802cddb816bea83f2a2e15fd9))
* Integrate code component for enhanced documentation rendering ([73a280f](https://github.com/cyberpath-HQ/sentinel/commit/73a280f7d2f7cb5b0af3156be1f5c76241da9fbb))
* Refactor DocsLayout component to streamline header and footer structure ([60b4ed0](https://github.com/cyberpath-HQ/sentinel/commit/60b4ed091b35ad697272aa914baceced04664706))
* Refactor DocsLayout to improve layout and structure of main content and Table of Contents ([43e7298](https://github.com/cyberpath-HQ/sentinel/commit/43e72985679240d15eaf36bf916d6aa0ba821c71))
* Remove section icons mapping from DocsLayout ([e0e9274](https://github.com/cyberpath-HQ/sentinel/commit/e0e9274078a27935cdad16735370c18cba139e46))
* Remove unnecessary 'relative' class from main content wrapper for improved layout ([0acfc7c](https://github.com/cyberpath-HQ/sentinel/commit/0acfc7cffed678013e1e7aa4b4d1f089cdd502a3))
* Remove unused Code import from index.astro ([d1ca82d](https://github.com/cyberpath-HQ/sentinel/commit/d1ca82dccb8f5bb2948c5162af5a4b3d07eed7fc))
* Reorder Node.js installation step in deployment workflow ([f474559](https://github.com/cyberpath-HQ/sentinel/commit/f47455961d52fda3fe6a3839c10e4fded4c19575))
* Replace logo placeholder with actual logo image in SiteHeader component ([034675f](https://github.com/cyberpath-HQ/sentinel/commit/034675f0b54a11ab04b32e4b28b9eaff4b560ba1))
* Standardize formatting in documentation for consistency ([01d55a5](https://github.com/cyberpath-HQ/sentinel/commit/01d55a5ac207afd02151584a4624b7df9a00cf2f))
* Update Astro configuration for improved documentation and add markdown enhancements ([289566a](https://github.com/cyberpath-HQ/sentinel/commit/289566a112ce4b3f9e5b2f40bf0bcc59fd838b42))
* Update date formatting in JSON-LD for better consistency ([a3d351d](https://github.com/cyberpath-HQ/sentinel/commit/a3d351d1a04d3ab7ed7d9bf810d3fcfccc8f3f68))
* Update formatting for consistency in cryptography documentation links ([9f4fe1b](https://github.com/cyberpath-HQ/sentinel/commit/9f4fe1b5744a7f622319b225de0c62d03a565db6))
* Update Node.js installation step in deployment workflow ([7daa021](https://github.com/cyberpath-HQ/sentinel/commit/7daa0219f017a138d11303cc104c386de138342e))
* Update package.json and pnpm-lock.yaml to add new dependencies for enhanced documentation features ([5dea6b7](https://github.com/cyberpath-HQ/sentinel/commit/5dea6b7cf65b455aff23468f702a114a45b5ac04))
* Update Rust version requirement to 1.92 in installation documentation ([5ccd070](https://github.com/cyberpath-HQ/sentinel/commit/5ccd07072138e7eeba79eb5f9bbee6eaa0b77b02))
* Update SearchModal and Site components for improved functionality and layout ([84dbdf0](https://github.com/cyberpath-HQ/sentinel/commit/84dbdf0a40eefecb0659c3d9a83290517b970d14))
* Update TableOfContents component styles for improved layout and visibility ([d937781](https://github.com/cyberpath-HQ/sentinel/commit/d9377812031721a16347a0d4cbcb137b96e18b0e))
* Update TableOfContents component styles for improved layout and visibility ([44cdd95](https://github.com/cyberpath-HQ/sentinel/commit/44cdd95398b8aad1a4f9d0bfac6f5dc5cab61f4e))

## [1.0.6](https://github.com/cyberpath-HQ/sentinel/compare/v1.0.5...v1.0.6) (2026-01-15)


### Bug Fixes

* update regex patterns to include version in path dependencies ([e724acd](https://github.com/cyberpath-HQ/sentinel/commit/e724acde2989a3af3694269a7d9a36822b31a4e2))
* update sentinel package versions to 1.0.5 ([16943e5](https://github.com/cyberpath-HQ/sentinel/commit/16943e53b2423b1ecd1803aa0dfe9a5c4e216e0f))

## [1.0.5](https://github.com/cyberpath-HQ/sentinel/compare/v1.0.4...v1.0.5) (2026-01-15)


### Bug Fixes

* update sentinel package versions to 1.0.4 and adjust asset globbing in release config ([af325b8](https://github.com/cyberpath-HQ/sentinel/commit/af325b8c1d78ab9142d90d56f5d342e475729063))

## [1.0.4](https://github.com/cyberpath-HQ/sentinel/compare/v1.0.3...v1.0.4) (2026-01-15)


### Bug Fixes

* update dependencies and refactor code to use sentinel-dbms ([302d05c](https://github.com/cyberpath-HQ/sentinel/commit/302d05c75e04ead38c2027607e16db3abedf7469))

## [1.0.3](https://github.com/cyberpath-HQ/sentinel/compare/v1.0.2...v1.0.3) (2026-01-15)


### Bug Fixes

* bump version for sentinel, sentinel-cli, and sentinel-crypto to 1.0.1 ([da38dc4](https://github.com/cyberpath-HQ/sentinel/commit/da38dc48cf8f07c1678206f362284f72078ebb67))
* bump version for sentinel, sentinel-cli, and sentinel-crypto to 1.0.2; update release configuration for command simplification ([9859fd6](https://github.com/cyberpath-HQ/sentinel/commit/9859fd630dbd3ac43471742dcc82cbc06def7e62))
* remove redundant command in prepareCmd for release configuration ([d9a3c89](https://github.com/cyberpath-HQ/sentinel/commit/d9a3c898342eb01b935f54624e9c351121deab3f))
* specify version for sentinel and sentinel-crypto dependencies in Cargo.toml ([8e9ffb0](https://github.com/cyberpath-HQ/sentinel/commit/8e9ffb0bc5b845c74c0e58a9bb88e3656e2ae2d3))
* update release configuration to include dependency version updates ([05a027e](https://github.com/cyberpath-HQ/sentinel/commit/05a027e5b9b1c10e4f703d4e83c0af719dfb4d7e))

## [1.0.2](https://github.com/cyberpath-HQ/sentinel/compare/v1.0.1...v1.0.2) (2026-01-15)


### Bug Fixes

* bump version for sentinel, sentinel-cli, and sentinel-crypto to 1.0.0 ([4b4aef4](https://github.com/cyberpath-HQ/sentinel/commit/4b4aef4d1a6788ab6f790ccd801a33919c52cc16))
* update Ascon128 import to use AsconAead128 for clarity ([4c50dd6](https://github.com/cyberpath-HQ/sentinel/commit/4c50dd676100dfd0b884c77c23f4fafe59313dda))

## [1.0.1](https://github.com/cyberpath-HQ/sentinel/compare/v1.0.0...v1.0.1) (2026-01-15)


### Bug Fixes

* remove CARGO_REGISTRY_TOKEN from publish command in release configuration ([663c39c](https://github.com/cyberpath-HQ/sentinel/commit/663c39c7fdaf99cfd43f22a8a49e55b82bd4004c))
* update readme paths in Cargo.toml files for cli, sentinel-crypto, and sentinel crates ([ed97933](https://github.com/cyberpath-HQ/sentinel/commit/ed9793369d6219d7303f978c895879238be13e55))

# 1.0.0 (2026-01-15)


### Bug Fixes

* add missing readme entry in Cargo.toml for cli package ([37ef797](https://github.com/cyberpath-HQ/sentinel/commit/37ef797441ed0eb7fd8d4c890874fafefbca71b3))
* Correct doctest import path and add test for update validation ([7f9d95e](https://github.com/cyberpath-HQ/sentinel/commit/7f9d95e734483f571c439831db6fbbfabd592ad6))
* Correct document data serialization in get command ([2e36867](https://github.com/cyberpath-HQ/sentinel/commit/2e3686738fd3add3dc6a9761513836a5bc987f72))
* Format command for running flamegraph in profiling workflow ([5ae9348](https://github.com/cyberpath-HQ/sentinel/commit/5ae934849998ca2c1fd8cd0d2eb7d02e84eb9d1e))
* Handle potential errors in hash and signature generation in Document methods ([f6ba79c](https://github.com/cyberpath-HQ/sentinel/commit/f6ba79c5e94bf5ac14f8c191b2b87e3243a9a474))
* Remove CLI binary configuration from Cargo.toml ([4d9b3e9](https://github.com/cyberpath-HQ/sentinel/commit/4d9b3e98a9024a2916ae80e722ec6f0d9d0ae7ba))
* Remove commented-out languages from CodeQL workflow configuration ([16d0d82](https://github.com/cyberpath-HQ/sentinel/commit/16d0d8266ec949f8af8bec09eb0fcdb46c8a25bb))
* Remove coverage threshold check from CI workflow ([dbf3114](https://github.com/cyberpath-HQ/sentinel/commit/dbf31145450fe7d026d9a3b54698543c1dff4c0b))
* Remove unnecessary --no-inline option from flamegraph command ([6389240](https://github.com/cyberpath-HQ/sentinel/commit/63892404f2841ed2fe2694b113ac283f32666d80))
* Remove unused clap dependency from Cargo.toml ([2fb3ce9](https://github.com/cyberpath-HQ/sentinel/commit/2fb3ce9523492c861fd339b0a50bc9b02247e52d))
* Remove unused import of Collection and update CLI description ([1a40619](https://github.com/cyberpath-HQ/sentinel/commit/1a406192789ecc008e2d62c75abc627ba9b7a9ab))
* Remove unused import of Store in collection.rs ([a1cf7c8](https://github.com/cyberpath-HQ/sentinel/commit/a1cf7c83e7e7cd71d809a067f868024644b3e64f))
* Replace KeyManager with SigningKeyManager in benchmarking functions ([536bb42](https://github.com/cyberpath-HQ/sentinel/commit/536bb425c511074f10752c07122275bae14da9fd))
* Simplify cargo tarpaulin command in coverage workflow ([e63d557](https://github.com/cyberpath-HQ/sentinel/commit/e63d557ed16742c952a06cb236ada061aabfdb92))
* Simplify flamegraph command in profiling workflow ([0ab0f94](https://github.com/cyberpath-HQ/sentinel/commit/0ab0f947a3942826714120748213530d1e9bfb7d))
* Specify full path for flamegraph executable in profiling workflow ([b808dec](https://github.com/cyberpath-HQ/sentinel/commit/b808dec4466cc6b05ad8d4a95d54549ac663ed1c))
* Suppress clippy warning for stdout printing in get command ([93c9e35](https://github.com/cyberpath-HQ/sentinel/commit/93c9e35bad8ccf70e95a981f631d1322ef9b2ea3))
* update branch regex pattern in release configuration ([e554943](https://github.com/cyberpath-HQ/sentinel/commit/e5549438541c5b1b92cea14a94573b36164d6a31))
* Update Codecov badge URL to use the correct path for GitHub ([835057c](https://github.com/cyberpath-HQ/sentinel/commit/835057c9d26c748d6c04378ad80f3465c5b09cae))
* Update coverage report file path in GitHub Actions workflow ([4b848e5](https://github.com/cyberpath-HQ/sentinel/commit/4b848e581742f77469150f9316ac06b2a69ae723))
* Update coverage report generation and upload paths in CI workflow ([33ac212](https://github.com/cyberpath-HQ/sentinel/commit/33ac2128d3885eb3adb25fa9d87b1178d027cbdc))
* Update coverage report output format to XML and adjust upload directory ([1203c94](https://github.com/cyberpath-HQ/sentinel/commit/1203c940b7bb4e7c85a9805be9749fa0d17cce8c))
* update dependencies in Cargo.lock and enhance release workflow configuration ([6a8b8a1](https://github.com/cyberpath-HQ/sentinel/commit/6a8b8a112eea7f702bf28f1d9adb6fe17aee6cb6))
* update GitHub App token configuration and bump dependencies in Cargo.lock ([eaae965](https://github.com/cyberpath-HQ/sentinel/commit/eaae96570d0272f378f0de1efc3a460e4305690f))
* update GITHUB_TOKEN handling in release workflow and specify rand_core versions in Cargo.lock ([644a30b](https://github.com/cyberpath-HQ/sentinel/commit/644a30b22cd5a4a850702ce7e1f112b0bf13572c))
* update GITHUB_TOKEN to use SEMANTIC_GH_TOKEN in release workflow ([233ba28](https://github.com/cyberpath-HQ/sentinel/commit/233ba28b5adf46851d6feb0bc1ae9ea3c0b15da3))
* Update lint rules for clarity and consistency in Cargo.toml ([e972f35](https://github.com/cyberpath-HQ/sentinel/commit/e972f35b1133a6ca7b065d0b1866c4976bdf2e9e))


### Features

* Add benchmark profile configuration to Cargo.toml ([72976f1](https://github.com/cyberpath-HQ/sentinel/commit/72976f19d9e6c5a8dde835d94fd15c900367bac4))
* Add benchmark tests for collection operations using Criterion ([1187ca8](https://github.com/cyberpath-HQ/sentinel/commit/1187ca8098650fc70518a9a174e4fb8da521db43))
* Add benchmarking for cryptographic functions using Criterion ([d1c2c88](https://github.com/cyberpath-HQ/sentinel/commit/d1c2c88de5767ee47888ebe1995a6811a82cf8b6))
* Add CLI commands for managing Sentinel collections and documents ([2eab056](https://github.com/cyberpath-HQ/sentinel/commit/2eab056c04950038bd7b41c08320f0b03647ac35))
* Add Cyberpath Sentinel AI Coding Guidelines documentation ([f26f13c](https://github.com/cyberpath-HQ/sentinel/commit/f26f13c41cd40bbd47998dbeefb9e091b63fc93d))
* Add document ID validation to reject non-filename-safe characters ([23ba02b](https://github.com/cyberpath-HQ/sentinel/commit/23ba02b38d47613be1445eb03ff5d5c06959aa54))
* Add filesystem-safe collection name validation ([19ed4ec](https://github.com/cyberpath-HQ/sentinel/commit/19ed4ec4c5036516ff2b8b6ce0db552e52d13b73))
* Add GitHub Actions workflows for benchmarks, clippy, coverage, formatting, profiling, publishing, and testing ([e157207](https://github.com/cyberpath-HQ/sentinel/commit/e1572071de34380efa2476e0378f3a47e3c2e832))
* Add GitHub funding, Dependabot, CodeQL, and SonarCloud workflows ([a79e490](https://github.com/cyberpath-HQ/sentinel/commit/a79e490c0db82c3f84760e5c6b5c7eceb7b1dd39))
* Add initial Cargo.toml, Cargo.lock, and main.rs files for the sentinel package ([c3c180b](https://github.com/cyberpath-HQ/sentinel/commit/c3c180b6bb0cb58e3d76597c2a104a783ad76712))
* Add initial Codecov configuration for coverage reporting and status checks ([eed4f08](https://github.com/cyberpath-HQ/sentinel/commit/eed4f08e18085403a998977a35417b6737edad3f))
* Add logo SVG for Cyberpath and Sentinel+dot branding ([3460203](https://github.com/cyberpath-HQ/sentinel/commit/34602038ab41a244ef0215656d03b400deeeba75))
* Add rayon-core, thiserror, and zeroize dependencies to enhance concurrency and error handling ([d90dab8](https://github.com/cyberpath-HQ/sentinel/commit/d90dab825d363de1a0a796baddae7924e58f832f))
* Add release configuration files for versioning and changelog management ([bdf0305](https://github.com/cyberpath-HQ/sentinel/commit/bdf030536b0be813c94baf52baf7bc25dd7ffe6b))
* Add sentinel-crypto crate with hashing and signing functionality ([c482070](https://github.com/cyberpath-HQ/sentinel/commit/c4820706d3d1c992f688e36c672c2aff5af810f9))
* Add tarpaulin configuration file for test coverage reporting ([770f81f](https://github.com/cyberpath-HQ/sentinel/commit/770f81f9c8bdb0c145d88a4f507f3a4d5c3d0ae7))
* Add tempfile as a development dependency for CLI ([9c5092d](https://github.com/cyberpath-HQ/sentinel/commit/9c5092d3ae8818f269d663f7ddae96aab8434dc5))
* Add thiserror and thiserror-impl packages for improved error handling ([f02a97c](https://github.com/cyberpath-HQ/sentinel/commit/f02a97c2aa6d198092125ff79784e8cb9b67da08))
* Enhance command argument structures and add comprehensive tests for CLI commands ([703de18](https://github.com/cyberpath-HQ/sentinel/commit/703de181381f2465ee6a4660012c59d236671088))
* Enhance Document structure with versioning, timestamps, and cryptographic features ([9d6919f](https://github.com/cyberpath-HQ/sentinel/commit/9d6919f5d61dd3d20b67a73ef63851a7fcd879f7))
* Enhance sentinel-crypto with comprehensive error handling and modular hashing/signing traits ([abdde85](https://github.com/cyberpath-HQ/sentinel/commit/abdde854ec5438af44c814670331686471313985))
* Implement Blake3 hashing and Ed25519 signing functionalities with key management utilities ([3b55193](https://github.com/cyberpath-HQ/sentinel/commit/3b5519361eee8a2b128a14102e56717a3bb49681))
* Implement Blake3 hashing and Ed25519 signing functionalities with key management utilities ([62bc694](https://github.com/cyberpath-HQ/sentinel/commit/62bc694fb3bd9c060e1f7d87a700b523108c663d))
* Implement document ID validation and add validation module ([41a40a9](https://github.com/cyberpath-HQ/sentinel/commit/41a40a984866635ade753233746119338fd35a3c))
* Implement document storage and management with Collection and Store modules ([51b0b7b](https://github.com/cyberpath-HQ/sentinel/commit/51b0b7b9053dd4ddd2682ee0806c22416adb20a1))
* Implement initial CLI structure with command handling and logging ([9b666d6](https://github.com/cyberpath-HQ/sentinel/commit/9b666d69f47e922afcd1c25cb0f4acdb100532c4))
* Implement initial library structure with add function and tests ([bf5269e](https://github.com/cyberpath-HQ/sentinel/commit/bf5269ee8edd129a19ae37aae4c5cce5c9cf4a57))
* Initialize Cyberpath Sentinel project with core files and implementation plan ([34e9dc7](https://github.com/cyberpath-HQ/sentinel/commit/34e9dc727d53e2c978c909f965d19e13a6f5971b))
* Introduce SentinelError type for structured error handling and update Result type across modules ([4d5f3b1](https://github.com/cyberpath-HQ/sentinel/commit/4d5f3b11396e95bb44fdb608565c665583766aca))
* Update .gitignore to include flamegraph and performance data files ([d2bdba7](https://github.com/cyberpath-HQ/sentinel/commit/d2bdba7a9ff909ed9898ce8202c96903eb1b6096))
* Update Cargo.toml with dependencies and configuration for benchmarks and binaries ([42e4fc5](https://github.com/cyberpath-HQ/sentinel/commit/42e4fc5dbe6588c105a3b0080378ce382b446789))
* Update implementation plan with completed tasks and async handling in tests ([4f0aa87](https://github.com/cyberpath-HQ/sentinel/commit/4f0aa87aad76fd179720eefa27056b029882593f))
* Update project structure and add Code of Conduct, NOTICE, and rustfmt configuration ([af5e53f](https://github.com/cyberpath-HQ/sentinel/commit/af5e53ff62d7298641bab7b6ae293f32843bfb46))
* Update sentinel-crypto dependencies and add benchmarking configuration ([3f9cfc1](https://github.com/cyberpath-HQ/sentinel/commit/3f9cfc15acdc5f67f83c3c8017432c0fa0c5e275))
