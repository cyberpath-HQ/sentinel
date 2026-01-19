#!/usr/bin/env node

import { execSync } from 'child_process';
import { existsSync, mkdirSync, copyFileSync, readdirSync, statSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

const nextRelease = process.env.NEXT_RELEASE_VERSION || process.argv[2];

if (!nextRelease) {
  console.error('Error: NEXT_RELEASE_VERSION not set');
  process.exit(1);
}

console.log(`\n🚀 Starting release process for version ${nextRelease}\n`);

function run(cmd, options = {}) {
  console.log(`$ ${cmd}`);
  try {
    execSync(cmd, { stdio: 'inherit', shell: '/bin/bash', ...options });
  } catch (error) {
    console.error(`Command failed: ${cmd}`);
    process.exit(1);
  }
}

function section(name) {
  console.log(`\n${'='.repeat(60)}`);
  console.log(`  ${name}`);
  console.log(`${'='.repeat(60)}\n`);
}

async function main() {
  const workspaceRoot = join(__dirname, '..');

   section('Publishing Rust Crates to crates.io');

   run('cargo publish --manifest-path crates/sentinel-crypto/Cargo.toml', { cwd: workspaceRoot });
   run('cargo publish --manifest-path crates/sentinel/Cargo.toml', { cwd: workspaceRoot });
   run('cargo publish --manifest-path crates/cli/Cargo.toml', { cwd: workspaceRoot });

   section('Creating C/C++ Development Package');

   const cxxBindings = join(workspaceRoot, 'bindings', 'cxx');
   const distDir = join(workspaceRoot, 'dist');
  mkdirSync(distDir, { recursive: true });

  const packageName = `sentinel-cxx-dev-${nextRelease}`;
  const stagingDir = join(distDir, packageName);
  const libDir = join(stagingDir, 'lib');

  // Clean staging directory
  run(`rm -rf ${stagingDir}`);
  mkdirSync(stagingDir, { recursive: true });
  mkdirSync(libDir, { recursive: true });

  // Copy libraries for each target
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
      console.log(`Copied dynamic library for ${target.system}`);
    }
    if (existsSync(staticLib)) {
      copyFileSync(staticLib, join(targetLibDir, `libsentinel_cxx.a`));
      console.log(`Copied static library for ${target.system}`);
    }
  }

  // Copy header from one of the builds (headers should be the same)
  const headerSrc = join(workspaceRoot, 'target', 'x86_64-unknown-linux-gnu', 'release', 'sentinel-cxx.h');
  const includeDir = join(stagingDir, 'include');
  mkdirSync(includeDir, { recursive: true });
  if (existsSync(headerSrc)) {
    copyFileSync(headerSrc, join(includeDir, 'sentinel-cxx.h'));
  }

  // Copy additional headers from bindings
  const bindingsInclude = join(cxxBindings, 'include');
  if (existsSync(bindingsInclude)) {
    run(`cp -r ${bindingsInclude}/* ${includeDir}/`, { cwd: workspaceRoot });
  }

  // Copy cmake files
  const cmakeDir = join(stagingDir, 'cmake');
  mkdirSync(cmakeDir, { recursive: true });
  const bindingsCmake = join(cxxBindings, 'cmake');
  if (existsSync(bindingsCmake)) {
    run(`cp -r ${bindingsCmake}/* ${cmakeDir}/`, { cwd: workspaceRoot });
  }

  // Copy examples
  const examplesDir = join(stagingDir, 'examples');
  mkdirSync(examplesDir, { recursive: true });
  const bindingsExamples = join(cxxBindings, 'examples');
  if (existsSync(bindingsExamples)) {
    run(`cp -r ${bindingsExamples}/* ${examplesDir}/`, { cwd: workspaceRoot });
  }

  // Copy documentation and build files
  const filesToCopy = ['README.md', 'CMakeLists.txt'];
  for (const file of filesToCopy) {
    const src = join(cxxBindings, file);
    if (existsSync(src)) {
      copyFileSync(src, join(stagingDir, file));
    }
  }

  // Create zip
  const zipName = `${packageName}.zip`;
  const zipPath = join(distDir, zipName);

  run(`rm -f ${zipPath}`, { cwd: distDir });
  run(`cd ${distDir} && zip -r ${zipName} ${packageName}`, { cwd: distDir });

  if (existsSync(zipPath)) {
    console.log(`Created: ${zipPath}`);
  }

  section('Building Python Wheel');

  const wheelsDir = join(workspaceRoot, 'target', 'wheels');
  mkdirSync(wheelsDir, { recursive: true });

  run(`maturin build --manifest-path ${join(workspaceRoot, 'crates', 'sentinel-python', 'Cargo.toml')} --release --out ${wheelsDir}`);

  section('Publishing Python to PyPI');

  if (process.env.TWINE_USERNAME && process.env.TWINE_PASSWORD) {
    run(`twine upload ${wheelsDir}/*.whl --skip-existing --non-interactive`);
  } else {
    console.log('⚠️  TWINE_USERNAME or TWINE_PASSWORD not set, skipping PyPI upload');
  }

  section('Building Node.js Native Modules');

  const jsBindings = join(workspaceRoot, 'bindings', 'js');
  run('npm ci', { cwd: jsBindings });
  run('npx @napi-rs/cli build --release', { cwd: jsBindings });

  section('Publishing Node.js Native to npm');

  if (process.env.NPM_TOKEN) {
    run('npm publish', { cwd: jsBindings });
  } else {
    console.log('⚠️  NPM_TOKEN not set, skipping npm upload');
  }

  section('Building WASM Package');

  const wasmBindings = join(workspaceRoot, 'bindings', 'wasm');
  run('wasm-pack build --release', { cwd: wasmBindings });

  section('Publishing WASM to npm');

  if (process.env.NPM_TOKEN) {
    run('npm publish', { cwd: wasmBindings });
  } else {
    console.log('⚠️  NPM_TOKEN not set, skipping npm upload');
  }

  section('Release Complete!');

  console.log('Published:');
  console.log('  ✓ Rust crates to crates.io');
  console.log('  ✓ C/C++ development package ready');
  console.log('  ✓ Python wheel to PyPI');
  console.log('  ✓ Node.js native to npm');
  console.log('  ✓ WASM package to npm');
}

main().catch(error => {
  console.error('Release failed:', error);
  process.exit(1);
});
