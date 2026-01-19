#!/usr/bin/env node
/**
 * Sentinel Release Script
 *
 * Handles the complete release process for Cyberpath Sentinel including:
 * - Publishing Rust crates to crates.io
 * - Building and packaging C/C++ development libraries
 * - Building Python wheels and publishing to PyPI
 * - Building Node.js native modules and publishing to npm
 *
 * Usage:
 *   node scripts/release.mjs <version> [--dry-run]
 *   NEXT_RELEASE_VERSION=<version> DRY_RUN=1 node scripts/release.mjs
 */

import { execSync } from 'child_process';
import { existsSync, mkdirSync, copyFileSync } from 'fs';
import { join, dirname } from 'path';
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
 * @returns {import('child_process').SpawnSyncReturns<string>|null} - Command result or null if dry-run
 */
function run(cmd, options = {}) {
  const prefix = isDryRun ? '🔍 [DRY RUN]' : '⚡';
  console.log(`${prefix} Executing: ${cmd}`);
  console.log(`─`.repeat(60));

  if (isDryRun) {
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
// Build Operations
// =============================================================================

/**
 * Publishes Rust crates to crates.io
 * @param {string} workspaceRoot - Root directory of the workspace
 */
function publishRustCrates(workspaceRoot) {
  section('Publishing Rust Crates to crates.io');

  if (isDryRun) {
    info('Would publish the following crates:');
    console.log('  • sentinel-crypto');
    console.log('  • sentinel');
    console.log('  • cli');
  } else {
    run('cargo publish --manifest-path crates/sentinel-crypto/Cargo.toml', { cwd: workspaceRoot });
    run('cargo publish --manifest-path crates/sentinel/Cargo.toml', { cwd: workspaceRoot });
    run('cargo publish --manifest-path crates/cli/Cargo.toml', { cwd: workspaceRoot });
    success('Published Rust crates to crates.io');
  }
}

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
    return wheelsDir;
  }

  mkdirSync(wheelsDir, { recursive: true });
  run(`maturin build --manifest-path ${join(workspaceRoot, 'crates', 'sentinel-python', 'Cargo.toml')} --release --out ${wheelsDir}`);

  success(`Built Python wheel in ${wheelsDir}`);

  return wheelsDir;
}

/**
 * Publishes Python wheel to PyPI
 * @param {string} wheelsDir - Directory containing wheel files
 */
function publishPythonToPypi(wheelsDir) {
  section('Publishing Python to PyPI');

  const pythonBindings = join(join(__dirname, '..'), 'bindings', 'python');

  if (!existsSync(pythonBindings)) {
    warn('bindings/python not found, skipping PyPI upload');
    return;
  }

  if (isDryRun) {
    info('Would upload Python wheel to PyPI');
    return;
  }

  if (process.env.TWINE_USERNAME && process.env.TWINE_PASSWORD) {
    run(`twine upload ${wheelsDir}/*.whl --skip-existing --non-interactive`);
    success('Published Python wheel to PyPI');
  } else {
    warn('TWINE_USERNAME or TWINE_PASSWORD not set, skipping PyPI upload');
  }
}

/**
 * Builds Node.js native module
 * @param {string} workspaceRoot - Root directory of the workspace
 */
function buildNodeJsModule(workspaceRoot) {
  section('Building Node.js Native Modules');

  const jsBindings = join(workspaceRoot, 'bindings', 'js');

  if (!existsSync(jsBindings)) {
    warn('bindings/js not found, skipping Node.js native module build');
    return;
  }

  run('npm ci', { cwd: jsBindings });
  run(`cargo build --release -p sentinel-js`, { cwd: workspaceRoot });
  run(`cp ${join(workspaceRoot, 'crates', 'sentinel-js', 'target', 'release', '*.node')} ${join(jsBindings, 'native')}/ 2>/dev/null || true`, { cwd: workspaceRoot });

  success('Built Node.js native module');
}

/**
 * Publishes Node.js module to npm
 * @param {string} workspaceRoot - Root directory of the workspace
 */
function publishNodeJsToNpm(workspaceRoot) {
  section('Publishing Node.js Native to npm');

  const jsBindings = join(workspaceRoot, 'bindings', 'js');

  if (!existsSync(jsBindings)) {
    warn('bindings/js not found, skipping npm upload');
    return;
  }

  if (isDryRun) {
    info('Would publish Node.js native module to npm');
    return;
  }

  if (process.env.NPM_TOKEN) {
    run('npm publish', { cwd: jsBindings });
    success('Published Node.js native module to npm');
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

  console.log(`\n🚀 Sentinel Release v${nextRelease}`);
  console.log(`   Mode: ${isDryRun ? '🔍 DRY RUN (no publishing)' : '⚡ LIVE RELEASE'}\n`);

  // Phase 1: Publish Rust crates
  publishRustCrates(workspaceRoot);

  // Phase 2: Build C/C++ development package
  const cxxZipPath = buildCxxDevPackage(workspaceRoot, nextRelease);

  // Phase 3: Build and publish Python wheel
  const wheelsDir = buildPythonWheel(workspaceRoot);
  publishPythonToPypi(wheelsDir);

  // Phase 4: Build and publish Node.js native module
  buildNodeJsModule(workspaceRoot);
  publishNodeJsToNpm(workspaceRoot);

  // Completion
  section(isDryRun ? 'Dry Run Complete!' : 'Release Complete!');

  if (isDryRun) {
    console.log('🔍 Dry run completed. No packages were published.');
    console.log('\nThe following would have been published:');
    console.log('  • Rust crates to crates.io');
    console.log(`  • C/C++ dev package: ${cxxZipPath}`);
    console.log('  • Python wheel to PyPI');
    console.log('  • Node.js native module to npm');
  } else {
    console.log('✅ Release completed successfully!');
    console.log('\nPublished:');
    console.log('  • Rust crates to crates.io');
    console.log('  • C/C++ development package');
    console.log('  • Python wheel to PyPI');
    console.log('  • Node.js native module to npm');
  }
}

// Execute main function
main().catch(error => {
  console.error('\n❌ Release failed:', error.message);
  process.exit(1);
});
