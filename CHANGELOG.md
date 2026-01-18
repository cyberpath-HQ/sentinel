# [2.0.0](https://github.com/cyberpath-HQ/sentinel/compare/v1.2.0...v2.0.0) (2026-01-18)


### Bug Fixes

* add async_futures feature to criterion dependency for enhanced benchmarking capabilities ([6ca85e9](https://github.com/cyberpath-HQ/sentinel/commit/6ca85e9d06ddb1cc45d7c802e93c0673feb28a55))
* add futures dependency and improve async benchmarking in crypto_benches ([f181964](https://github.com/cyberpath-HQ/sentinel/commit/f1819643f0fbca5d0082ecffcf0299151c26f600))
* add missing allow attribute for code coverage in compare_json_values function ([2079f8a](https://github.com/cyberpath-HQ/sentinel/commit/2079f8a9536ae09010d45afd7fb28da1da98fed1))
* change type_order function to a constant function for improved performance ([bb2197e](https://github.com/cyberpath-HQ/sentinel/commit/bb2197e46b37211d53f2307d5b2ec1449e6aed1d))
* correct formatting in comparison function for JSON values ([b1b936f](https://github.com/cyberpath-HQ/sentinel/commit/b1b936ff045e36432efbe522b4da64a9f7fbd045))
* correct ID handling in data import function for better compatibility ([73b42af](https://github.com/cyberpath-HQ/sentinel/commit/73b42af046d41bbe4e2ec976873090bcce31922a))
* correct wording for clarity in introduction section ([330d4dd](https://github.com/cyberpath-HQ/sentinel/commit/330d4ddb59aacae24c76b2f57111401e598ca3c4))
* correct wording in UNIX Philosophy section for clarity ([4a673f3](https://github.com/cyberpath-HQ/sentinel/commit/4a673f3538c12e97bfd95af4d0a9fc62100a1c5b))
* enhance document ID extraction and improve sorting and pagination logic in collection operations ([ba009c3](https://github.com/cyberpath-HQ/sentinel/commit/ba009c3ec42bb90d36d397b69b8e010f036c6df5))
* enhance document retrieval logic in mixed concurrent operations benchmark ([c2c8299](https://github.com/cyberpath-HQ/sentinel/commit/c2c8299e783d89f1cff09ca7838262991fc09889))
* enhance filter matching logic by using references and improving type handling ([701f0ba](https://github.com/cyberpath-HQ/sentinel/commit/701f0ba98fc39831570cd2d8f05fcf6ee20718ad))
* ensure only files are processed by checking metadata for directories ([ba85cc0](https://github.com/cyberpath-HQ/sentinel/commit/ba85cc09f7449a59cce17e27ba1e5c2b21fcde05))
* handle potential None value for total_count in query result logging ([808e636](https://github.com/cyberpath-HQ/sentinel/commit/808e6364db632b72bfe63c77d35174c78077b85f))
* improve code formatting and clippy lints in query and filtering modules ([3003e0d](https://github.com/cyberpath-HQ/sentinel/commit/3003e0d3e3d9247e080fb17c8cede83e60894269))
* improve count handling in list command to prevent overflow ([75f341a](https://github.com/cyberpath-HQ/sentinel/commit/75f341a28daeca317e96c49d330ad8a04a9c5060))
* improve documentation for Query and QueryBuilder structs ([2f61f9e](https://github.com/cyberpath-HQ/sentinel/commit/2f61f9e1cb12bbaafdebbf7c52b517177f726987))
* improve error handling and code coverage annotations in JSON serialization and document merging ([7e4e2bd](https://github.com/cyberpath-HQ/sentinel/commit/7e4e2bdd89e9acda32c89dfb6e9a24d8c15a2623))
* improve error handling in document serialization and file writing ([8619652](https://github.com/cyberpath-HQ/sentinel/commit/86196529959526bfcca5c9db88296dd521cae25e))
* improve error handling in parse_sort function for better sort format validation ([48dd243](https://github.com/cyberpath-HQ/sentinel/commit/48dd243008673574e24b0c1823d52f6ec329229d))
* improve error handling in signature verification mode parsing ([8004cdf](https://github.com/cyberpath-HQ/sentinel/commit/8004cdf86ab59add3c545797643df2addd33c1a6))
* improve error handling in verification mode parsing for GetArgs ([1287b30](https://github.com/cyberpath-HQ/sentinel/commit/1287b3074e5121d7349c07f3864c09cf31907b56))
* improve error handling in verification mode parsing for signature and hash modes ([0d422e1](https://github.com/cyberpath-HQ/sentinel/commit/0d422e1447b7dbe72925d8025d4a2ef640eab60a))
* improve formatting in collection retrieval logic for better readability ([b78e6df](https://github.com/cyberpath-HQ/sentinel/commit/b78e6df247e00dbd9c84cdc08ec493e99fd522c3))
* improve JSON value comparison by dereferencing values and enhancing number handling ([1ac0617](https://github.com/cyberpath-HQ/sentinel/commit/1ac06171cd66f476730c14ed876842c2bb69cae1))
* improve logging format in query results and enhance JSON comparison logic ([dee0c43](https://github.com/cyberpath-HQ/sentinel/commit/dee0c43588204d986936028520c9f2ec4f992d0f))
* refine error handling for permission denied in safe_operation function ([7389632](https://github.com/cyberpath-HQ/sentinel/commit/7389632425c25260e677de51a9539ebcdf7ad2ad))
* **dependencies:** remove unused futures dependency from CLI ([000e511](https://github.com/cyberpath-HQ/sentinel/commit/000e51136ddd80e3bad470b1b666e7f1818c70bd))
* **dependencies:** remove unused futures dependency from sentinel-cli ([f4a1a46](https://github.com/cyberpath-HQ/sentinel/commit/f4a1a469687ac0d6a271f6a230986254078da480))
* simplify access to first element in parse_sort function for improved readability ([8289fc0](https://github.com/cyberpath-HQ/sentinel/commit/8289fc0c830fa69206d8c3541aecfec931b4871d))
* simplify argument handling and enhance filter parsing in query command ([d38f224](https://github.com/cyberpath-HQ/sentinel/commit/d38f2242a579a940361643ba72c7331b578a75fa))
* simplify document ID extraction by using strip_suffix for file names ([ce9bc88](https://github.com/cyberpath-HQ/sentinel/commit/ce9bc887da00e0edad3cd0316432dcdc77aa8fe7))
* streamline error handling in document serialization ([1e0898b](https://github.com/cyberpath-HQ/sentinel/commit/1e0898b148f3b4799b1c4411fa6e2aeeb1b689d0))
* **collection:** test logic in example blocks ([2ae9e40](https://github.com/cyberpath-HQ/sentinel/commit/2ae9e40faeb8ae6267562cd16b7ea5648102bdd3))
* update benchmarks to support async operations for sign, encrypt, and decrypt functions ([d4e9e81](https://github.com/cyberpath-HQ/sentinel/commit/d4e9e81ad46ed8acfb406c8b47df31571f1bbe13))
* **dependencies:** update chrono to version 0.4.43 for compatibility ([babe9b6](https://github.com/cyberpath-HQ/sentinel/commit/babe9b639a093726e4049d85631b10538d17e329))
* **Cargo:** update crate-type to only include rlib and add tracing-subscriber as a dev-dependency ([c162f10](https://github.com/cyberpath-HQ/sentinel/commit/c162f10cd2076e77065ea9905c7091db16e0ed7e))
* update default value for Accordion to reflect current section based on currentSlug ([8a01f2e](https://github.com/cyberpath-HQ/sentinel/commit/8a01f2e67fa576c40836cf4bf84dfaf2d150dfc8))
* update document version handling to use META_SENTINEL_VERSION constant ([396d1f7](https://github.com/cyberpath-HQ/sentinel/commit/396d1f73234048bdeb9c1be78b461bf87a86a302))
* **dependencies:** update futures usage to use sentinel_dbms for consistency ([45eca06](https://github.com/cyberpath-HQ/sentinel/commit/45eca063a7e57b020799a6bbf4c023027773026f))
* update global crypto config function to be asynchronous ([3b684bc](https://github.com/cyberpath-HQ/sentinel/commit/3b684bc7b58ddba3f055f8e011a205d01fff8f56))
* update import statement for better stream handling in best practices guide ([74058e5](https://github.com/cyberpath-HQ/sentinel/commit/74058e5081f73fd61fd5847ae9bc698767bcc950))
* update module documentation for clarity and consistency ([fd88bbd](https://github.com/cyberpath-HQ/sentinel/commit/fd88bbde40677db0c0f5d0faad67645cff8ed5ba))


### Features

* add Aggregation enum for enhanced query operations ([5672cf9](https://github.com/cyberpath-HQ/sentinel/commit/5672cf944d877704083f9dbf2059c661bc9b2980))
* add Aggregation to Query exports for enhanced query capabilities ([dcd4837](https://github.com/cyberpath-HQ/sentinel/commit/dcd48370679d95bbea834f9678b85a7884fa29ca))
* **dependencies:** add async-stream and futures packages for enhanced async support ([5265de2](https://github.com/cyberpath-HQ/sentinel/commit/5265de2aab359ef4b43d0839a52d7e208e2c0624))
* add collection deletion and listing methods to Store ([a3c6c04](https://github.com/cyberpath-HQ/sentinel/commit/a3c6c046466aace338f344b937aafe93bdb9b1c5))
* add comprehensive benchmarking for collection operations and memory usage ([1024467](https://github.com/cyberpath-HQ/sentinel/commit/1024467afd3dc3192ce4fdd3dce33f902580122f))
* add comprehensive tests for collection methods including count, upsert, and aggregation ([4d12398](https://github.com/cyberpath-HQ/sentinel/commit/4d123981409424140c168ce448d2e473c60f4858))
* **tests:** add comprehensive tests for filtering, projection, and store functionalities ([328e720](https://github.com/cyberpath-HQ/sentinel/commit/328e720934e24b0cab971e20ed6b394e76bd9bf9))
* **lib:** add crate-type specification for library output formats ([ed8436c](https://github.com/cyberpath-HQ/sentinel/commit/ed8436cc568621cee05c4431534014a2d6315524))
* **collection:** add document filtering and querying capabilities ([1feddd5](https://github.com/cyberpath-HQ/sentinel/commit/1feddd567cee47ba05ee2c3f076ce0ef7e15c279))
* **projection:** add document projection utility to include specified fields ([5b1a38b](https://github.com/cyberpath-HQ/sentinel/commit/5b1a38b93a1ad2f50295d6c3f9b0a4d12db70425))
* add document verification errors and options ([61f917e](https://github.com/cyberpath-HQ/sentinel/commit/61f917e7653996a3c036858183fab48e0956b6ec))
* **scripts:** add documentation script to package.json ([7662380](https://github.com/cyberpath-HQ/sentinel/commit/7662380d2b7afec2509c9c995be276843f116c50))
* add empty signature handling mode to GetArgs for improved document verification ([14e0f4a](https://github.com/cyberpath-HQ/sentinel/commit/14e0f4a1ea26a74ba1b931b8682ec9e18457ceab))
* add empty signature handling mode to ListArgs for improved document verification ([f48e8da](https://github.com/cyberpath-HQ/sentinel/commit/f48e8dadcc96a4bffc7cbbfb8976c4a1301af952))
* add empty signature handling mode to QueryArgs for improved document processing ([ea7e8ce](https://github.com/cyberpath-HQ/sentinel/commit/ea7e8ce69ff91f54f749c95b30e47a4d57f97e94))
* add empty signature handling mode to VerificationOptions for improved document processing ([3907e3c](https://github.com/cyberpath-HQ/sentinel/commit/3907e3c18260d4e8b4b6698551eae2075743795b))
* **dependencies:** add futures and async-stream for improved async support ([3779494](https://github.com/cyberpath-HQ/sentinel/commit/3779494ece68f7e215e2c1377db7212a472f906f))
* add futures::StreamExt import for enhanced stream handling in collection example ([d5763f3](https://github.com/cyberpath-HQ/sentinel/commit/d5763f36368212307c8eb1ea5b72388599d91d15))
* add GitHub Actions workflow for opencode integration ([b23eff1](https://github.com/cyberpath-HQ/sentinel/commit/b23eff13ed134381778dbe57728d060f1d4a5ef2))
* add initial review guidelines and development instructions for opencode ([b9ee673](https://github.com/cyberpath-HQ/sentinel/commit/b9ee6735ae75d5a99e813fb632b4fd8848a46766))
* add lazy_static and tokio dependencies to Cargo.lock ([f17aaee](https://github.com/cyberpath-HQ/sentinel/commit/f17aaee25de9cb24ee0e0146de997fea05fc1854))
* add serde_json import for JSON handling in quick start guide ([d326f5d](https://github.com/cyberpath-HQ/sentinel/commit/d326f5d29e4ea25ed9c48c299ec873201044987b))
* add serde_json import for JSON handling in quick start guide ([db288e5](https://github.com/cyberpath-HQ/sentinel/commit/db288e5b93782741be5cd03069c296593396e887))
* add serial test support for async key derivation and encryption tests ([2fcf7a8](https://github.com/cyberpath-HQ/sentinel/commit/2fcf7a8ce6e7be8da574b06d3757001887028031))
* add serial_test dependency for improved testing support ([4150f3e](https://github.com/cyberpath-HQ/sentinel/commit/4150f3ee7b40a63c0cfde8d05aa27e8c6d092ae1))
* add streaming method to retrieve all documents in the collection ([aebe9fb](https://github.com/cyberpath-HQ/sentinel/commit/aebe9fb8184c0a53eea19c0f1a0e3b32decb833e))
* **streaming:** add streaming utility for processing document IDs from a collection directory ([58b47af](https://github.com/cyberpath-HQ/sentinel/commit/58b47afd717d157c46ae26a7aff211c471206287))
* add tests for collection deletion and listing functionality ([bb73d48](https://github.com/cyberpath-HQ/sentinel/commit/bb73d4823bbd596c018f8487d66eb1918218d41a))
* add tests for comparing large negative numbers and NaN in JSON values ([c0d80cc](https://github.com/cyberpath-HQ/sentinel/commit/c0d80cc1bfbc293d47c7054cd042436aaacccf4a))
* add tests for document ID streaming and error handling ([2ac7c98](https://github.com/cyberpath-HQ/sentinel/commit/2ac7c98d2f6ae69807a599fffa46bf3004a2bfd6))
* add tests for document insertion, deletion, and filtering with non-number values ([e172af1](https://github.com/cyberpath-HQ/sentinel/commit/e172af10e8d130f1582aad34b75aeb52bce8594a))
* add tests for filtering and validation functions ([991d0d0](https://github.com/cyberpath-HQ/sentinel/commit/991d0d08bcdd30de43677379269339ed227a9667))
* add tests for handling invalid signature mode in get, list, and query commands ([fa6576c](https://github.com/cyberpath-HQ/sentinel/commit/fa6576cbc8714ea48f4bbda2c0d3ef1ff16eb6c1))
* **Cargo:** add tracing-subscriber as a dependency ([aa3f22c](https://github.com/cyberpath-HQ/sentinel/commit/aa3f22cce1330ba0f8573a8ad60bdadbe6e8e171))
* **dependencies:** add tracing-subscriber as a dev-dependency ([a830ad8](https://github.com/cyberpath-HQ/sentinel/commit/a830ad8d117d378bc85f66cd1b0190170a293697))
* **tests:** add tracing-subscriber for enhanced logging in test cases ([cecf0ca](https://github.com/cyberpath-HQ/sentinel/commit/cecf0caa8713244ff28677d7e2977539b708e927))
* **comparison:** add utilities for comparing and sorting JSON values ([aff6c1a](https://github.com/cyberpath-HQ/sentinel/commit/aff6c1a7cab4b726025d63b2f838e2b7784c9515))
* **filtering:** complete implementation of basic filtering features ([fc5bb84](https://github.com/cyberpath-HQ/sentinel/commit/fc5bb847f4670de526a1575b0f91d4790fc55eb2))
* enhance benchmarking for cryptographic functions with additional scenarios ([0bd46d8](https://github.com/cyberpath-HQ/sentinel/commit/0bd46d8b60b70115025a2b7cc929cd7222364af5))
* **collection:** enhance document filtering and querying with improved utility functions and streaming support ([6952581](https://github.com/cyberpath-HQ/sentinel/commit/69525816f64d516cb8f2577e40c315d2ef7afcde))
* enhance document verification options with empty signature mode and refactor verification methods ([49dab78](https://github.com/cyberpath-HQ/sentinel/commit/49dab783159fefde5a2e9acd3576aab3bf83e013))
* **query:** enhance filtering capabilities with additional operators and syntax ([c889593](https://github.com/cyberpath-HQ/sentinel/commit/c889593f89648471af5e6d05101a23bcce9a3b2a))
* enhance get, list, and query commands with verification options and improved argument structure ([7001848](https://github.com/cyberpath-HQ/sentinel/commit/7001848e5120f3f2cb9e7ebf5caacbc59cb617e2))
* enhance number comparison in compare_json_values for large numbers ([00f53af](https://github.com/cyberpath-HQ/sentinel/commit/00f53afc7fa3da704451338f7df327a98f599642))
* enhance opencode job conditions to restrict access to specific author associations ([1e9cd52](https://github.com/cyberpath-HQ/sentinel/commit/1e9cd52bde202332db4ed2c19618eb4ddb8cb907))
* enhance VerificationMode with string parsing and error handling ([833304c](https://github.com/cyberpath-HQ/sentinel/commit/833304caa060737595aa0910835ba66c0b13d171))
* **query:** implement basic query structure and filtering capabilities ([7db0b9a](https://github.com/cyberpath-HQ/sentinel/commit/7db0b9ad2f75d9b7f0c198432ba1b8d982099579))
* implement document update with merging capabilities and aggregation support ([f9da297](https://github.com/cyberpath-HQ/sentinel/commit/f9da2974cfc967700a971b9bec88619c4f30063a))
* **filtering:** implement filtering utilities for document matching ([51894ce](https://github.com/cyberpath-HQ/sentinel/commit/51894cea631828ff5483fac0d41205e0c0a3051d))
* **query:** implement query command with filtering and sorting capabilities ([5703275](https://github.com/cyberpath-HQ/sentinel/commit/570327547f0d69b4fe72158bf36506968457d02a))
* improve verification mode handling and add empty signature mode support ([850fe81](https://github.com/cyberpath-HQ/sentinel/commit/850fe818a33ead077c2865500a6be6336c18b98f))
* make main function asynchronous to support crypto configuration ([1a8d6cb](https://github.com/cyberpath-HQ/sentinel/commit/1a8d6cb2948c88ed16a3508f9e8d9c73449cc269))
* make main function asynchronous to support global crypto configuration ([0312681](https://github.com/cyberpath-HQ/sentinel/commit/0312681b7bac120a75f6ebdf7c2385649bcf2738))
* **collection:** optimize document filtering and querying with streaming approach ([728bf6b](https://github.com/cyberpath-HQ/sentinel/commit/728bf6b080f2f37c2b7a27a7b29a7d615b29fae6))
* **query:** optimize document handling in query results using TryStreamExt ([6ed715d](https://github.com/cyberpath-HQ/sentinel/commit/6ed715d477b78dcc87611d1049503d10d650d58d))
* optimize filter processing by precomputing filter references ([f5ecd4b](https://github.com/cyberpath-HQ/sentinel/commit/f5ecd4bb4d560284adf0db8d6b7a05c84be7d431))
* **lib:** re-export serde_json for improved JSON handling and streaming utility ([853cb46](https://github.com/cyberpath-HQ/sentinel/commit/853cb4653360fd4a3c6c9fc53f594c62d171a374))
* reduce data size in large data benchmarks for improved performance ([9046692](https://github.com/cyberpath-HQ/sentinel/commit/90466922db640078209b9152cd38060630597f6a))
* **collection:** refactor document ID listing to return a stream for improved efficiency ([dfadc8f](https://github.com/cyberpath-HQ/sentinel/commit/dfadc8f8494db03e505bffcbd4619d7f56da1025))
* **collection:** refactor document processing utilities for improved modularity ([5a893c2](https://github.com/cyberpath-HQ/sentinel/commit/5a893c2b4727f0e0b82839c76bf09d49b281a372))
* refactor encryption key management and hashing functions for async support ([dbe26a1](https://github.com/cyberpath-HQ/sentinel/commit/dbe26a1296ea72c403bdad04269466417cbf922d))
* **collection, query:** refactor filtering and querying to use streaming for improved performance ([64af6b9](https://github.com/cyberpath-HQ/sentinel/commit/64af6b90397c61ecb93096b955f2e658c213e6fd))
* refine benchmarking code by removing unused variables and simplifying expressions ([9f87c67](https://github.com/cyberpath-HQ/sentinel/commit/9f87c67542c7a93de8d76396ee1cfff28d98fc6d))
* remove redundant error handling section from quick start guide ([5872941](https://github.com/cyberpath-HQ/sentinel/commit/58729416d749ae492a847c160451ee19def6d76d))
* remove redundant update method and add document count functionality ([3d3c502](https://github.com/cyberpath-HQ/sentinel/commit/3d3c5023be15488cd75f64bdda753d50003fb585))
* **lib:** reorganize external crate re-exports for improved clarity ([524d1ba](https://github.com/cyberpath-HQ/sentinel/commit/524d1ba1c4babf41cf7372bbd16458ed8515057c))
* **lib:** reorganize module exports for improved structure and clarity ([4e9a000](https://github.com/cyberpath-HQ/sentinel/commit/4e9a0005cb3609ebe999b6b441f39da4cfedfabf))
* simplify projection syntax in QueryBuilder examples ([7f7c248](https://github.com/cyberpath-HQ/sentinel/commit/7f7c248c085900ccda5b0cdec21f88148ccb0f00))
* simplify signature representation in document metadata ([1f1ff9f](https://github.com/cyberpath-HQ/sentinel/commit/1f1ff9fa1d956f05a22a9481a726180459be4514))
* switch global config to use TokioRwLock for async support and enhance configuration management ([4436fc2](https://github.com/cyberpath-HQ/sentinel/commit/4436fc2e4ab517c8cc961c3b643c33ce783bc242))
* update .gitignore and add .ignore to manage review files and markdown exclusions ([96b325a](https://github.com/cyberpath-HQ/sentinel/commit/96b325a4f2c43660a3d9829529cb59620335a600))
* **bench:** update collection list benchmark to use try_collect for improved error handling ([d8bf18b](https://github.com/cyberpath-HQ/sentinel/commit/d8bf18b5773383d1e51b57be3b9318855ccb3a1f))
* update decryption benchmarks to use async execution ([5017cb4](https://github.com/cyberpath-HQ/sentinel/commit/5017cb4c9f98bc225eb9999b23df94788c589bbe))
* update document and collection handling for async support ([28d0057](https://github.com/cyberpath-HQ/sentinel/commit/28d00572b76b220e835b36635b5e574efc3e6cf0))
* update document projection to return Result type for error handling ([7f9136e](https://github.com/cyberpath-HQ/sentinel/commit/7f9136ee5411d2dc7344d9eb5719f39970da8d71))
* update document retrieval to use verification options for enhanced security ([74b2e66](https://github.com/cyberpath-HQ/sentinel/commit/74b2e66536de369129d32f163a41fec0c85f4393))
* update get, list, and query commands to disable signature and hash verification by default ([169ae98](https://github.com/cyberpath-HQ/sentinel/commit/169ae98f493acb60aa6c485ec27ddb2acd500e08))
* update init command to support async key derivation and encryption ([0f02287](https://github.com/cyberpath-HQ/sentinel/commit/0f022873f97d72b3b43a84f8a296ea53b12f9666))
* update key derivation benchmarks to use async execution ([5025a69](https://github.com/cyberpath-HQ/sentinel/commit/5025a696117c7bc3811df5136f87cbb8ff4f7612))
* update lazy_static to version 1.5 and add serial_test dependency ([e0dac83](https://github.com/cyberpath-HQ/sentinel/commit/e0dac8364ca4978b9adaf8909b0e4145ce3ad3c8))
* **cli:** update list and insert commands to use try_collect for improved error handling ([b006cce](https://github.com/cyberpath-HQ/sentinel/commit/b006cceeb61a4b9dd22826ba55bb96adf8f0815e))
* update matches_filters function to accept references for improved performance ([ebe9601](https://github.com/cyberpath-HQ/sentinel/commit/ebe96019f5a6e191fb4654bacad19a4cb0b9ce98))
* update README for improved installation and usage instructions ([7ad1720](https://github.com/cyberpath-HQ/sentinel/commit/7ad172097ad10b4a30100bcf001ecf85266edb1c))
* update section order in DocsLayout for improved navigation ([35c0973](https://github.com/cyberpath-HQ/sentinel/commit/35c09734f236f8b8f640005d01686c7a73a332b6))
* **dependencies:** update sentinel packages to version 1.2.0 ([c133269](https://github.com/cyberpath-HQ/sentinel/commit/c133269b1da988712730e4cb8d8e73f95439dc2f))
* update serde_json and criterion dependencies with additional features ([c221d73](https://github.com/cyberpath-HQ/sentinel/commit/c221d7351591f4784adbae8c5dcff0081e0b4c6b))
* update set_global_crypto_config function to be asynchronous ([1e9b18c](https://github.com/cyberpath-HQ/sentinel/commit/1e9b18c81441bc0524e8cad65a684705acceb35f))
* update store initialization to use async execution ([c04abd3](https://github.com/cyberpath-HQ/sentinel/commit/c04abd3b8441134cd3ddcfabe61dad8e470543ec))
* update stream_document_ids to filter out directories when streaming document IDs ([b0d1813](https://github.com/cyberpath-HQ/sentinel/commit/b0d1813740598d7ebd5c2217f3e50d3b6ba26734))
* update tests to use async functions and add new dependencies ([1fc9ef4](https://github.com/cyberpath-HQ/sentinel/commit/1fc9ef4cfc677e20cccb7e7da81c63de3d5eec7c))
* update Tokio dependency to include macros and runtime features ([b05a159](https://github.com/cyberpath-HQ/sentinel/commit/b05a1597fa8d9814a1544375afe28ef2a8edc711))


### BREAKING CHANGES

* this requires migration of lots of dependents functions to async causing a waterfall effect on the `sentinel` crate
* **collection:** CHANGE

# [1.2.0](https://github.com/cyberpath-HQ/sentinel/compare/v1.1.0...v1.2.0) (2026-01-16)


### Bug Fixes

* Add margin-bottom to code display container for improved spacing ([111edfb](https://github.com/cyberpath-HQ/sentinel/commit/111edfb9863f5a34e1630f9d633f8326a69e76dc))
* Add padding to sidebar content and remove GitHub icon from navigation ([598a512](https://github.com/cyberpath-HQ/sentinel/commit/598a51284334294b82ceb8789e4f26f07d8cf5ce))
* Add search button to SiteHeader for improved documentation navigation ([61b7c5a](https://github.com/cyberpath-HQ/sentinel/commit/61b7c5a76c111bd7b63e4b63e4281ccd66ec7371))
* Adjust layout and styling for code display sections in index.astro ([8503c49](https://github.com/cyberpath-HQ/sentinel/commit/8503c49b0266bfbddcd1fafb69031036dafbf8a3))
* Adjust search button layout in SiteHeader for improved visibility and alignment ([990fc89](https://github.com/cyberpath-HQ/sentinel/commit/990fc896ee4691964e0e7fb4ced7c6076d28b74b))
* Enhance code display section with improved styling and overflow handling ([0cf8410](https://github.com/cyberpath-HQ/sentinel/commit/0cf841021a092d7c4be3855cacf5e5ec133bf90c))
* Remove border from code block styling for improved aesthetics ([e45a6a5](https://github.com/cyberpath-HQ/sentinel/commit/e45a6a552e19f8f6091191ccd732482fb61d9c9a))
* Remove Header and Footer components from the project ([ff26d02](https://github.com/cyberpath-HQ/sentinel/commit/ff26d02fa1d14b5ac1bdb3c2a4da43aa992924cd))
* Replace mobile navigation button with PanelRightClose icon and remove mobile search button ([441509a](https://github.com/cyberpath-HQ/sentinel/commit/441509ac11d7f9553cfc8b37aa1901ed3f1fe596))
* Update code block syntax from 'text' to 'plaintext' for consistency in documentation ([e1f91a7](https://github.com/cyberpath-HQ/sentinel/commit/e1f91a7071d9045d08cecdfb754077a1ff8f29e1))
* Update code display section to improve layout and enable horizontal scrolling ([2690a3c](https://github.com/cyberpath-HQ/sentinel/commit/2690a3c9e4f47cab469c12c637dea3214751499e))
* Update lint command and correct metadata field names for consistency ([6df10f1](https://github.com/cyberpath-HQ/sentinel/commit/6df10f190e18816cfdcfd2e953d61ed7036fb141))


### Features

* Add benchmarking for collection list operation ([6f81110](https://github.com/cyberpath-HQ/sentinel/commit/6f8111023816116f20148ae5bdfcbf06533fef3e))
* Add benchmarking for encrypt, decrypt, and derive key operations ([a32d6a1](https://github.com/cyberpath-HQ/sentinel/commit/a32d6a1d9b4eeeaa9ebe54372c863218cc2d282d))
* Add List and BulkInsert commands to the Sentinel CLI ([69ace81](https://github.com/cyberpath-HQ/sentinel/commit/69ace8194f4d3c6ede05ae2950c3254f6e617159))
* Add list command to retrieve and display documents in a Sentinel collection ([d4f7f90](https://github.com/cyberpath-HQ/sentinel/commit/d4f7f9037bcbaa0c5152e303066aac568aabaa82))
* Add tracing dependency to sentinel and sentinel-crypto ([a6f1963](https://github.com/cyberpath-HQ/sentinel/commit/a6f19633b5b0881a036e98c940d915bf5ae57612))
* Add tracing support across the crypto and collection modules for improved logging ([fb81149](https://github.com/cyberpath-HQ/sentinel/commit/fb811495c5ebf3ee13d27cf0ca510fa212a62f78))
* Bump version of sentinel-cli, sentinel-crypto, and sentinel-dbms to 1.1.0 ([2ff08e8](https://github.com/cyberpath-HQ/sentinel/commit/2ff08e89bc9b55afc962b376ab831a980ebf2b61))
* Enhance error handling and add tests for insert command and signature verification ([ddee06e](https://github.com/cyberpath-HQ/sentinel/commit/ddee06e7d072202b793762aa85a7ea629e6bf6bc))
* Enhance insert command to support bulk insert from JSON file ([c748f49](https://github.com/cyberpath-HQ/sentinel/commit/c748f49d859ee9c5acd4fdb177db9f63269b30af))
* Fix collection entry validation to ensure only files are processed ([b3a780a](https://github.com/cyberpath-HQ/sentinel/commit/b3a780aae717386883e7d5676d97ded2b774b01f))
* Implement bulk insert command for documents in a Sentinel collection ([61436a5](https://github.com/cyberpath-HQ/sentinel/commit/61436a5a169e81591442f3d828853c427c5eccdc))
* Implement soft delete for documents and add list functionality in Collection ([aa5b8f2](https://github.com/cyberpath-HQ/sentinel/commit/aa5b8f2d53fd71abc129909f8a83b506a29165fa))
* Refactor code for improved readability and consistency in insert, mod, crypto, and collection operations ([dd4256e](https://github.com/cyberpath-HQ/sentinel/commit/dd4256e854ebcbee04585f94a654394be2a85a79))
* Remove bulk insert command implementation from CLI ([a616c34](https://github.com/cyberpath-HQ/sentinel/commit/a616c34da86c47fa60c677e91afbab8b7a4bc3df))
* Update implementation plan to reflect completion of directory operations ([ca9b5fc](https://github.com/cyberpath-HQ/sentinel/commit/ca9b5fc5bb2ffa2bed5ce60abbbe548724d3274f))
* Update insert command to use Option for id and data fields in tests ([51e2008](https://github.com/cyberpath-HQ/sentinel/commit/51e2008e973d48e5a6cc01ca41cc85effeca7200))

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
