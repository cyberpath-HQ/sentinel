/**
 * @type {import('semantic-release').GlobalConfig}
 */
export default {
  branches: [
    "main",
    "+([0-9])?(.{+([0-9]),x}).x",
    { name: "beta", prerelease: true },
    { name: "alpha", prerelease: true },
    { name: "rc", prerelease: true },
  ],
  plugins: [
    [
      "@semantic-release/commit-analyzer",
      {
        preset: "angular",
        releaseRules: [
          { type: "perf", release: "minor" },
          { type: "chore", release: "patch" },
          { type: "style", release: "patch" },
          { type: "refactor", release: "patch" },
          { type: "docs", release: false },
          { type: "test", release: false },
        ],
        parserOpts: {
          noteKeywords: ["BREAKING CHANGE", "BREAKING CHANGES"],
        },
      },
    ],
    [
      "@semantic-release/release-notes-generator",
      {
        preset: "angular",
        parserOpts: {
          noteKeywords: ["BREAKING CHANGE", "BREAKING CHANGES", "BREAKING"],
        },
        writerOpts: {
          commitsSort: ["subject", "scope"],
        },
      },
    ],
    "@semantic-release/changelog",
    [
      "@semantic-release/exec",
      {
        prepareCmd: 'sed -i \'s/^version = ".*"/version = "${nextRelease.version}"/\' Cargo.toml && node update-deps.js ${nextRelease.version}',
      },
    ],
    [
      "@semantic-release/git",
      {
        assets: ["CHANGELOG.md", "Cargo.toml", "crates/**/Cargo.toml"],
      },
    ],
    [
      "@semantic-release/exec",
      {
        publishCmd: "node scripts/release.mjs ${nextRelease.version}",
      },
    ],
    [
      "@semantic-release/github",
      {
        assets: [
          // CLI Platform Archives
          { path: "dist/sentinel-v${nextRelease.version}-windows-x86_64.zip", label: "Sentinel CLI for Windows x64" },
          { path: "dist/sentinel-v${nextRelease.version}-macos-x86_64.tar.gz", label: "Sentinel CLI for macOS x64 (Intel)" },
          { path: "dist/sentinel-v${nextRelease.version}-macos-aarch64.tar.gz", label: "Sentinel CLI for macOS ARM64 (Apple Silicon)" },
          { path: "dist/sentinel-v${nextRelease.version}-linux-x86_64.tar.gz", label: "Sentinel CLI for Linux x64 (glibc)" },
          { path: "dist/sentinel-v${nextRelease.version}-linux-aarch64.tar.gz", label: "Sentinel CLI for Linux ARM64 (glibc)" },
          { path: "dist/sentinel-v${nextRelease.version}-alpine-x86_64.tar.gz", label: "Sentinel CLI for Alpine Linux x64 (musl)" },
          { path: "dist/sentinel-v${nextRelease.version}-alpine-aarch64.tar.gz", label: "Sentinel CLI for Alpine Linux ARM64 (musl)" },
          // Distribution Packages
          { path: "dist/debian/sentinel-cli_${nextRelease.version}_amd64.deb", label: "Sentinel CLI Debian/Ubuntu Package" },
          { path: "dist/rpm/sentinel-cli-${nextRelease.version}-1.x86_64.rpm", label: "Sentinel CLI Fedora/RHEL Package" },
          { path: "dist/arch/sentinel-cli-${nextRelease.version}-1-x86_64.pkg.tar.zst", label: "Sentinel CLI Arch Linux Package" },
          { path: "dist/alpine/sentinel-cli-${nextRelease.version}.apk", label: "Sentinel CLI Alpine Package" },
          // Language Bindings
          { path: "dist/sentinel-cxx-dev-${nextRelease.version}.zip", label: "C/C++ Development Package" },
          { path: "target/wheels/*.whl", label: "Python Wheel Package" },
          { path: "bindings/js/native/*.node", label: "Node.js Native Binary" },
        ],
        failTitle: false,
        failComment: false,
      },
    ],
  ],
};
