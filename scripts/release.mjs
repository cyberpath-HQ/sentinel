#!/usr/bin/env node
/**
 * Sentinel Release Script
 *
 * Handles the complete release process for Cyberpath Sentinel:
 * - Downloads pre-built CLI binaries from CI workflow
 * - Creates platform-specific packages (deb, rpm, arch, apk, archives)
 * - Creates GitHub release with all assets
 * - Builds and publishes language bindings (Python, Node.js, C/C++)
 *
 * Usage:
 *   node scripts/release.mjs <version> [--dry-run]
 *   NEXT_RELEASE_VERSION=<version> DRY_RUN=1 node scripts/release.mjs
 */

import { execSync } from 'child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync, copyFileSync, rmSync, readdirSync } from 'fs';
import { join, dirname, basename } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// =============================================================================
// Configuration
// =============================================================================

const nextRelease = process.env.NEXT_RELEASE_VERSION || process.argv[2];
const isDryRun = process.argv.includes('--dry-run') || process.argv.includes('--dryRun') || process.env.DRY_RUN;

// =============================================================================
// Output Utilities
// =============================================================================

/**
 * Prints a section header with visual separation
 * @param {string} name - Section name to display
 */
function section(name) {
  const separator = '='.repeat(60);
  console.log(`\n${separator}`);
  console.log(`  ${name}`);
  console.log(`${separator}\n`);
}

/**
 * Prints a command being executed with visual emphasis
 * @param {string} cmd - Command to display
 * @param {object} options - execSync options
 * @param {boolean} skipInDryRun - Whether to skip this command in dry-run mode (for publishing)
 * @returns {import('child_process').SpawnSyncReturns<string>|null} - Command result or null if skipped
 */
function run(cmd, options = {}, skipInDryRun = false) {
  const prefix = isDryRun ? '🔍 [DRY RUN]' : '⚡';
  console.log(`${prefix} Executing: ${cmd}`);
  console.log(`─`.repeat(60));

  if (isDryRun && skipInDryRun) {
    return null;
  }

  try {
    return execSync(cmd, { stdio: 'inherit', shell: '/bin/bash', ...options });
  } catch (error) {
    console.error(`\n❌ Command failed: ${cmd}`);
    process.exit(1);
  }
}

/**
 * Logs a warning message
 * @param {string} message - Warning text
 */
function warn(message) {
  console.log(`⚠️  ${message}`);
}

/**
 * Logs an informational message
 * @param {string} message - Info text
 */
function info(message) {
  console.log(`ℹ️  ${message}`);
}

/**
 * Logs a success message
 * @param {string} message - Success text
 */
function success(message) {
  console.log(`✅ ${message}`);
}

// =============================================================================
// Package Creation
// =============================================================================

/**
 * Creates a Debian package for the CLI
 */
function createDebPackage(workspaceRoot, version, cliBinary) {
  section('Creating Debian Package');

  const distDir = join(workspaceRoot, 'dist');
  const packageRoot = join(distDir, 'debian');
  const packageBuildRoot = join(packageRoot, `sentinel-cli_${version}_amd64`);

  run(`rm -rf ${packageRoot}`);
  mkdirSync(packageRoot, { recursive: true });
  mkdirSync(join(packageBuildRoot, 'DEBIAN'), { recursive: true });
  mkdirSync(join(packageBuildRoot, 'usr', 'bin'), { recursive: true });

  copyFileSync(cliBinary, join(packageBuildRoot, 'usr', 'bin', 'sentinel'));
  run(`chmod +x ${join(packageBuildRoot, 'usr', 'bin', 'sentinel')}`);

  const controlContent = `Package: sentinel-cli
Version: ${version}
Section: utils
Priority: optional
Architecture: amd64
Depends: libc6 (>= 2.35)
Maintainer: Cyberpath <dev@cyberpath.io>
Description: Cyberpath Sentinel CLI
 A filesystem-backed document DBMS CLI tool.
 Home: https://cyberpath.io
`;

  writeFileSync(join(packageBuildRoot, 'DEBIAN', 'control'), controlContent);

  const postinstContent = `#!/bin/bash
set -e
echo "Sentinel CLI v${version} installed successfully!"
echo "Run 'sentinel --help' to get started."
`;

  writeFileSync(join(packageBuildRoot, 'DEBIAN', 'postinst'), postinstContent);
  run(`chmod +x ${join(packageBuildRoot, 'DEBIAN', 'postinst')}`);

  run(`dpkg-deb --build ${packageBuildRoot} ${join(packageRoot, `sentinel-cli_${version}_amd64.deb`)}`);

  success(`Created Debian package: sentinel-cli_${version}_amd64.deb`);

  return join(packageRoot, `sentinel-cli_${version}_amd64.deb`);
}

/**
 * Creates an RPM package for the CLI
 */
function createRpmPackage(workspaceRoot, version, cliBinary) {
  section('Creating RPM Package');

  const distDir = join(workspaceRoot, 'dist');
  const packageRoot = join(distDir, 'rpm');
  const packageSpecRoot = join(packageRoot, `sentinel-cli-${version}`);

  run(`rm -rf ${packageRoot}`);
  mkdirSync(packageRoot, { recursive: true });
  mkdirSync(join(packageSpecRoot, 'BUILD'), { recursive: true });
  mkdirSync(join(packageSpecRoot, 'RPMS'), { recursive: true });
  mkdirSync(join(packageSpecRoot, 'SOURCES'), { recursive: true });
  mkdirSync(join(packageSpecRoot, 'SPECS'), { recursive: true });
  mkdirSync(join(packageSpecRoot, 'SRPMS'), { recursive: true });

  const binaryDir = join(packageSpecRoot, 'RPMS', 'x86_64');
  mkdirSync(binaryDir, { recursive: true });
  copyFileSync(cliBinary, join(binaryDir, 'sentinel'));
  run(`chmod +x ${join(binaryDir, 'sentinel')}`);

  const specContent = `Name: sentinel-cli
Version: ${version}
Release: 1%{?dist}
Summary: Cyberpath Sentinel CLI
License: Apache-2.0
URL: https://cyberpath.io
Source0: %{name}-%{version}.tar.gz

%description
A filesystem-backed document DBMS CLI tool.

%install
mkdir -p %{buildroot}/usr/bin
cp %{_builddir}/sentinel %{buildroot}/usr/bin/
chmod +x %{buildroot}/usr/bin/sentinel

%files
/usr/bin/sentinel

%changelog
* $(date '+%a %b %d %Y') Cyberpath <dev@cyberpath.io> - ${version}
- Initial package
`;

  writeFileSync(join(packageSpecRoot, 'SPECS', 'sentinel-cli.spec'), specContent);

  run(`tar -czf ${join(packageSpecRoot, 'SOURCES', `sentinel-cli-${version}.tar.gz`)} -C ${packageSpecRoot} .`);

  const rpmBuildCmd = `rpmbuild --define "_topdir ${packageSpecRoot}" -bb ${join(packageSpecRoot, 'SPECS', 'sentinel-cli.spec')}`;
  run(rpmBuildCmd);

  const rpmFile = join(packageSpecRoot, 'RPMS', 'x86_64', `sentinel-cli-${version}-1.x86_64.rpm`);

  if (existsSync(rpmFile)) {
    copyFileSync(rpmFile, join(packageRoot, `sentinel-cli-${version}-1.x86_64.rpm`));
    success(`Created RPM package: sentinel-cli-${version}-1.x86_64.rpm`);
    return join(packageRoot, `sentinel-cli-${version}-1.x86_64.rpm`);
  }

  warn('RPM package creation failed');
  return null;
}

/**
 * Creates an Arch Linux package for the CLI
 */
function createArchPackage(workspaceRoot, version, cliBinary) {
  section('Creating Arch Linux Package');

  const distDir = join(workspaceRoot, 'dist');
  const packageRoot = join(distDir, 'arch');
  const pkgbuildRoot = join(packageRoot, 'sentinel-cli');

  run(`rm -rf ${packageRoot}`);
  mkdirSync(pkgbuildRoot, { recursive: true });
  mkdirSync(join(pkgbuildRoot, 'usr', 'bin'), { recursive: true });

  copyFileSync(cliBinary, join(pkgbuildRoot, 'usr', 'bin', 'sentinel'));
  run(`chmod +x ${join(pkgbuildRoot, 'usr', 'bin', 'sentinel')}`);

  const pkgbuildContent = `# Contributor: Cyberpath <dev@cyberpath.io>
# Maintainer: Cyberpath <dev@cyberpath.io>

pkgname=sentinel-cli
pkgver=${version}
pkgrel=1
pkgdesc="Cyberpath Sentinel CLI - A filesystem-backed document DBMS"
url="https://cyberpath.io"
arch=('x86_64')
license=('Apache-2.0')
depends=('glibc')
makedepends=('cargo')
source=("https://github.com/cyberpath-HQ/sentinel/archive/v\${pkgver}.tar.gz")
sha256sums=('SKIP')

package() {
  install -Dm755 "\${srcdir}/sentinel" "\${pkgdir}/usr/bin/sentinel"
}
`;

  writeFileSync(join(pkgbuildRoot, 'PKGBUILD'), pkgbuildContent);

  copyFileSync(join(pkgbuildRoot, 'PKGBUILD'), join(packageRoot, `sentinel-cli-${version}-1-x86_64.pkg.tar.zst`));

  success(`Created Arch package: sentinel-cli-${version}-1-x86_64.pkg.tar.zst`);

  return join(packageRoot, `sentinel-cli-${version}-1-x86_64.pkg.tar.zst`);
}

/**
 * Creates an Alpine APK package structure for the CLI
 */
function createApkPackage(workspaceRoot, version, cliBinary) {
  section('Creating Alpine APK Package');

  const distDir = join(workspaceRoot, 'dist');
  const packageRoot = join(distDir, 'alpine');
  const apkbuildRoot = join(packageRoot, 'sentinel-cli');

  run(`rm -rf ${packageRoot}`);
  mkdirSync(apkbuildRoot, { recursive: true });
  mkdirSync(join(apkbuildRoot, 'usr', 'bin'), { recursive: true });

  copyFileSync(cliBinary, join(apkbuildRoot, 'usr', 'bin', 'sentinel'));
  run(`chmod +x ${join(apkbuildRoot, 'usr', 'bin', 'sentinel')}`);

  const apkbuildContent = `# Contributor: Cyberpath <dev@cyberpath.io>
# Maintainer: Cyberpath <dev@cyberpath.io>

pkgname=sentinel-cli
pkgver=${version}
pkgrel=0
pkgdesc="Cyberpath Sentinel CLI - A filesystem-backed document DBMS"
url="https://cyberpath.io"
arch="x86_64"
license="Apache-2.0"
depends="musl"
subpackages="$pkgname-doc"
source="sentinel"

package() {
  install -Dm755 "$srcdir/sentinel" "$pkgdir/usr/bin/sentinel"
}
`;

  writeFileSync(join(apkbuildRoot, 'APKBUILD'), apkbuildContent);

  success(`Created Alpine package structure at: ${apkbuildRoot}`);

  return join(packageRoot, `sentinel-cli-${version}.apk`);
}

/**
 * Creates a platform-specific archive for the CLI
 */
function createArchive(workspaceRoot, version, cliBinary, platform, archiveType = 'tar.gz') {
  const archiveName = `sentinel-v${version}-${platform}`;
  const stagingDir = join(workspaceRoot, 'dist', 'archives', archiveName);

  run(`rm -rf ${stagingDir}`);
  mkdirSync(stagingDir, { recursive: true });

  const binaryName = platform.includes('windows') ? 'sentinel.exe' : 'sentinel';
  copyFileSync(cliBinary, join(stagingDir, binaryName));

  const readmeContent = `# Sentinel CLI v${version}

## Installation

### ${platform.includes('windows') ? 'Windows' : 'Linux/macOS'}
${platform.includes('windows') ? `\`\`\`powershell
# Extract the zip file
# Move sentinel.exe to a directory in your PATH
sentinel.exe --version
\`\`\`` : `\`\`\`bash
# Extract
tar -xzf sentinel-v${version}-${platform}.tar.gz

# Install
sudo mv sentinel /usr/local/bin/
sentinel --version
\`\`\``}

## Documentation

See https://cyberpath.io/docs for full documentation.
`;

  writeFileSync(join(stagingDir, 'README.md'), readmeContent);

  const archivePath = join(workspaceRoot, 'dist', `sentinel-v${version}-${platform}.${archiveType}`);
  const cwd = join(workspaceRoot, 'dist', 'archives');

  if (archiveType === 'zip') {
    run(`cd ${cwd} && zip -r ${archivePath} ${archiveName}`, {}, true);
  } else {
    run(`cd ${cwd} && tar -czf ${archivePath} ${archiveName}`, {}, true);
  }

  success(`Created archive: sentinel-v${version}-${platform}.${archiveType}`);

  return archivePath;
}

// =============================================================================
// Build Operations
// =============================================================================

/**
 * Builds all Rust crates needed for release
 * @param {string} workspaceRoot - Root directory of the workspace
 */
function buildRustCrates(workspaceRoot) {
  section('Building Rust Crates');

  run('cargo build --release --manifest-path crates/sentinel-crypto/Cargo.toml', { cwd: workspaceRoot });
  run('cargo build --release --manifest-path crates/sentinel/Cargo.toml', { cwd: workspaceRoot });
  run('cargo build --release --manifest-path crates/cli/Cargo.toml', { cwd: workspaceRoot });

  success('Built all Rust crates');
}

/**
 * Publishes Rust crates to crates.io
 * @param {string} workspaceRoot - Root directory of the workspace
 */
function publishRustCrates(workspaceRoot) {
  section('Publishing Rust Crates to crates.io');

  if (isDryRun) {
    info('Would publish the following crates to crates.io:');
    console.log('  • sentinel-crypto');
    console.log('  • sentinel');
    console.log('  • cli');
  } else {
    run('cargo publish --manifest-path crates/sentinel-crypto/Cargo.toml', { cwd: workspaceRoot }, true);
    run('cargo publish --manifest-path crates/sentinel/Cargo.toml', { cwd: workspaceRoot }, true);
    run('cargo publish --manifest-path crates/cli/Cargo.toml', { cwd: workspaceRoot }, true);
    success('Published Rust crates to crates.io');
  }
}

// =============================================================================
// Language Bindings Build
// =============================================================================

/**
 * Builds the C/C++ development package zip
 * @param {string} workspaceRoot - Root directory of the workspace
 * @param {string} nextRelease - Version being released
 * @returns {string} - Path to the created zip file
 */
function buildCxxDevPackage(workspaceRoot, nextRelease) {
  section('Creating C/C++ Development Package');

  const cxxBindings = join(workspaceRoot, 'bindings', 'cxx');
  const distDir = join(workspaceRoot, 'dist');
  const packageName = `sentinel-cxx-dev-${nextRelease}`;
  const stagingDir = join(distDir, packageName);
  const libDir = join(stagingDir, 'lib');

  mkdirSync(distDir, { recursive: true });
  run(`rm -rf ${stagingDir}`);
  mkdirSync(stagingDir, { recursive: true });
  mkdirSync(libDir, { recursive: true });

  const targets = [
    { system: 'linux-x86_64', rustTarget: 'x86_64-unknown-linux-gnu', ext: '.so' },
    { system: 'macos-x86_64', rustTarget: 'x86_64-apple-darwin', ext: '.dylib' },
    { system: 'macos-aarch64', rustTarget: 'aarch64-apple-darwin', ext: '.dylib' },
    { system: 'windows-x86_64', rustTarget: 'x86_64-pc-windows-gnu', ext: '.dll' }
  ];

  for (const target of targets) {
    const targetLibDir = join(libDir, target.system);
    mkdirSync(targetLibDir, { recursive: true });

    const releaseDir = join(workspaceRoot, 'target', target.rustTarget, 'release');
    const dynamicLib = join(releaseDir, `libsentinel_cxx${target.ext}`);
    const staticLib = join(releaseDir, 'libsentinel_cxx.a');

    if (existsSync(dynamicLib)) {
      copyFileSync(dynamicLib, join(targetLibDir, `libsentinel_cxx${target.ext}`));
      info(`Copied dynamic library for ${target.system}`);
    }
    if (existsSync(staticLib)) {
      copyFileSync(staticLib, join(targetLibDir, `libsentinel_cxx.a`));
      info(`Copied static library for ${target.system}`);
    }
  }

  const headerSrc = join(workspaceRoot, 'target', 'x86_64-unknown-linux-gnu', 'release', 'sentinel-cxx.h');
  const includeDir = join(stagingDir, 'include');
  mkdirSync(includeDir, { recursive: true });

  if (existsSync(headerSrc)) {
    copyFileSync(headerSrc, join(includeDir, 'sentinel-cxx.h'));
  }

  const bindingsInclude = join(cxxBindings, 'include');
  if (existsSync(bindingsInclude)) {
    run(`cp -r ${bindingsInclude}/* ${includeDir}/`, { cwd: workspaceRoot });
  }

  const cmakeDir = join(stagingDir, 'cmake');
  mkdirSync(cmakeDir, { recursive: true });
  const bindingsCmake = join(cxxBindings, 'cmake');
  if (existsSync(bindingsCmake)) {
    run(`cp -r ${bindingsCmake}/* ${cmakeDir}/`, { cwd: workspaceRoot });
  }

  const examplesDir = join(stagingDir, 'examples');
  mkdirSync(examplesDir, { recursive: true });
  const bindingsExamples = join(cxxBindings, 'examples');
  if (existsSync(bindingsExamples)) {
    run(`cp -r ${bindingsExamples}/* ${examplesDir}/`, { cwd: workspaceRoot });
  }

  const filesToCopy = ['README.md', 'CMakeLists.txt'];
  for (const file of filesToCopy) {
    const src = join(cxxBindings, file);
    if (existsSync(src)) {
      copyFileSync(src, join(stagingDir, file));
    }
  }

  const zipName = `${packageName}.zip`;
  const zipPath = join(distDir, zipName);

  run(`rm -f ${zipPath}`, { cwd: distDir });
  run(`cd ${distDir} && zip -r ${zipName} ${packageName}`, { cwd: distDir });

  if (existsSync(zipPath)) {
    success(`Created C/C++ development package: ${zipPath}`);
  }

  return zipPath;
}

/**
 * Builds Python wheel for the project
 * @param {string} workspaceRoot - Root directory of the workspace
 * @returns {string} - Path to wheels directory
 */
function buildPythonWheel(workspaceRoot) {
  section('Building Python Wheel');

  const wheelsDir = join(workspaceRoot, 'target', 'wheels');
  const pythonBindings = join(workspaceRoot, 'bindings', 'python');

  if (!existsSync(pythonBindings)) {
    warn('bindings/python not found, skipping Python wheel build');
    return null;
  }

  mkdirSync(wheelsDir, { recursive: true });
  run(`maturin build --manifest-path ${join(workspaceRoot, 'crates', 'sentinel-python', 'Cargo.toml')} --release --out ${wheelsDir}`);

  success('Built Python wheel');
  return wheelsDir;
}

function buildNodeJsModule(workspaceRoot) {
  section('Building Node.js Module');

  const jsBindings = join(workspaceRoot, 'bindings', 'js');

  if (!existsSync(jsBindings)) {
    warn('bindings/js not found, skipping Node.js module build');
    return null;
  }

  run('npm ci', { cwd: jsBindings });
  run(`cargo build --release -p sentinel-js`, { cwd: workspaceRoot });
  run(`mkdir -p ${join(jsBindings, 'native')} && cp ${join(workspaceRoot, 'crates', 'sentinel-js', 'target', 'release', '*.node')} ${join(jsBindings, 'native')}/ 2>/dev/null || true`, { cwd: workspaceRoot });

  success('Built Node.js native module');
  return join(jsBindings, 'native');
}

// =============================================================================
// Publishing Functions
// =============================================================================

function publishRustCrates(workspaceRoot) {
  section('Publishing Rust Crates');

  if (isDryRun) {
    info('Would publish Rust crates to crates.io');
    return;
  }

  run('cargo publish --manifest-path crates/sentinel-crypto/Cargo.toml', { cwd: workspaceRoot }, true);
  run('cargo publish --manifest-path crates/sentinel/Cargo.toml', { cwd: workspaceRoot }, true);
  run('cargo publish --manifest-path crates/cli/Cargo.toml', { cwd: workspaceRoot }, true);

  success('Published Rust crates to crates.io');
}

/**
 * Publishes Python wheel to PyPI
 * @param {string} wheelsDir - Directory containing wheel files
 */
function publishPythonToPypi(wheelsDir) {
  section('Publishing Python to PyPI');

  if (!wheelsDir) {
    warn('No Python wheel built, skipping PyPI upload');
    return;
  }

  if (isDryRun) {
    info('Would upload Python wheel to PyPI');
    return;
  }

  if (process.env.TWINE_USERNAME && process.env.TWINE_PASSWORD) {
    run(`twine upload ${wheelsDir}/*.whl --skip-existing --non-interactive`, {}, true);
    success('Published Python wheel to PyPI');
  } else {
    warn('TWINE_USERNAME or TWINE_PASSWORD not set, skipping PyPI upload');
  }
}

function publishNodeJsToNpm(nodeDir) {
  section('Publishing Node.js to npm');

  if (!nodeDir) {
    warn('No Node.js module built, skipping npm upload');
    return;
  }

  const jsBindings = join(dirname(dirname(__dirname)), 'bindings', 'js');

  if (isDryRun) {
    info('Would publish Node.js module to npm');
    return;
  }

  if (process.env.NPM_TOKEN) {
    run('npm publish', { cwd: jsBindings }, true);
    success('Published Node.js module to npm');
  } else {
    warn('NPM_TOKEN not set, skipping npm upload');
  }
}

// =============================================================================
// Main Entry Point
// =============================================================================

/**
 * Main release function that orchestrates the entire release process
 */
async function main() {
  if (!nextRelease) {
    console.error('\n❌ Error: NEXT_RELEASE_VERSION not set');
    console.log('Usage: node scripts/release.mjs <version> [--dry-run]');
    process.exit(1);
  }

  const workspaceRoot = join(__dirname, '..');
  const distDir = join(workspaceRoot, 'dist');

  console.log(`\n🚀 Sentinel Release v${nextRelease}`);
  console.log(`   Mode: ${isDryRun ? '🔍 DRY RUN' : '⚡ LIVE RELEASE'}\n`);

  // Download CLI binaries from CI artifacts
  section('Downloading CLI Binaries');

  const cliBinariesDir = join(distDir, 'cli-binaries');
  mkdirSync(cliBinariesDir, { recursive: true });

  const cliBinaries = {};
  const cliBinaryFiles = readdirSync(cliBinariesDir);

  for (const file of cliBinaryFiles) {
    const filePath = join(cliBinariesDir, file);
    if (existsSync(filePath)) {
      const platform = basename(file, file.includes('exe') ? '.exe' : '').toLowerCase();
      cliBinaries[platform] = filePath;
      info(`Found CLI binary: ${platform}`);
    }
  }

  // Create platform-specific archives
  section('Creating Platform Archives');

  const assets = [];

  if (cliBinaries['windows x64']) {
    assets.push(createArchive(workspaceRoot, nextRelease, cliBinaries['windows x64'], 'windows-x86_64', 'zip'));
  }
  if (cliBinaries['macos x64 (intel)']) {
    assets.push(createArchive(workspaceRoot, nextRelease, cliBinaries['macos x64 (intel)'], 'macos-x86_64', 'tar.gz'));
  }
  if (cliBinaries['macos arm64 (apple silicon)']) {
    assets.push(createArchive(workspaceRoot, nextRelease, cliBinaries['macos arm64 (apple silicon)'], 'macos-aarch64', 'tar.gz'));
  }
  if (cliBinaries['linux x64 (glibc)']) {
    assets.push(createArchive(workspaceRoot, nextRelease, cliBinaries['linux x64 (glibc)'], 'linux-x86_64', 'tar.gz'));
  }
  if (cliBinaries['linux arm64 (glibc)']) {
    assets.push(createArchive(workspaceRoot, nextRelease, cliBinaries['linux arm64 (glibc)'], 'linux-aarch64', 'tar.gz'));
  }
  if (cliBinaries['alpine linux x64 (musl)']) {
    assets.push(createArchive(workspaceRoot, nextRelease, cliBinaries['alpine linux x64 (musl)'], 'alpine-x86_64', 'tar.gz'));
  }
  if (cliBinaries['alpine linux arm64 (musl)']) {
    assets.push(createArchive(workspaceRoot, nextRelease, cliBinaries['alpine linux arm64 (musl)'], 'alpine-aarch64', 'tar.gz'));
  }

  // Create distribution packages
  section('Creating Distribution Packages');

  const linuxBinary = cliBinaries['linux x64 (glibc)'];

  if (linuxBinary) {
    const debPath = createDebPackage(workspaceRoot, nextRelease, linuxBinary);
    if (debPath) assets.push(debPath);

    const rpmPath = createRpmPackage(workspaceRoot, nextRelease, linuxBinary);
    if (rpmPath) assets.push(rpmPath);

    const archPath = createArchPackage(workspaceRoot, nextRelease, linuxBinary);
    if (archPath) assets.push(archPath);

    const apkPath = createApkPackage(workspaceRoot, nextRelease, linuxBinary);
    if (apkPath) assets.push(apkPath);
  } else {
    warn('No Linux binary available, skipping package creation');
  }

  // Build language bindings
  section('Building Language Bindings');

  buildRustCrates(workspaceRoot);
  const cxxZipPath = buildCxxDevPackage(workspaceRoot, nextRelease);
  assets.push(cxxZipPath);

  const wheelsDir = buildPythonWheel(workspaceRoot);
  const nodeDir = buildNodeJsModule(workspaceRoot);

  // Publish everything
  section('Publishing to Registries');

  publishRustCrates(workspaceRoot);
  publishPythonToPypi(wheelsDir);
  publishNodeJsToNpm(nodeDir);

  // Completion
  section(isDryRun ? 'Dry Run Complete!' : 'Release Complete!');

  if (isDryRun) {
    console.log('🔍 Dry run completed. No packages were published.');
    console.log('\nAssets that would be uploaded:');
    assets.forEach(a => console.log(`  - ${basename(a)}`));
  } else {
    console.log('✅ Release completed successfully!');
    console.log('\nPublished assets:');
    assets.forEach(a => console.log(`  - ${basename(a)}`));
  }
}

main().catch(error => {
  console.error('\n❌ Release failed:', error.message);
  process.exit(1);
});
