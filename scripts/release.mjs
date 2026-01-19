#!/usr/bin/env node

import { execSync } from 'child_process';
import { existsSync, mkdirSync, copyFileSync } from 'fs';
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

  section('Building Python Wheel');

  const wheelsDir = join(workspaceRoot, 'target', 'wheels');
  if (!existsSync(wheelsDir)) {
    mkdirSync(wheelsDir, { recursive: true });
  }

  run(`maturin build --manifest-path ${join(workspaceRoot, 'crates', 'sentinel-python', 'Cargo.toml')} --release --out ${wheelsDir}`);

  const wheelFile = execSync(`ls ${wheelsDir}/*.whl 2>/dev/null | head -1`).toString().trim();
  if (wheelFile) {
    console.log(`Built wheel: ${wheelFile}`);
  }

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

  section('Generating C/C++ Development Package');

  const cxxBindings = join(workspaceRoot, 'bindings', 'cxx');
  run('cargo run --bin sentinel-cxx-generator --release', { cwd: cxxBindings });

  const zipName = `sentinel-cxx-dev-${nextRelease}.zip`;
  run(`zip -r ${zipName} sentinel-cxx-*`, { cwd: cxxBindings });

  const zipPath = join(cxxBindings, zipName);
  if (existsSync(zipPath)) {
    console.log(`Created: ${zipPath}`);

    const distDir = join(workspaceRoot, 'dist');
    if (!existsSync(distDir)) {
      mkdirSync(distDir, { recursive: true });
    }
    copyFileSync(zipPath, join(distDir, zipName));
    console.log(`Copied to: ${join(distDir, zipName)}`);
  }

  section('Release Complete!');

  console.log('Published:');
  console.log('  ✓ Rust crates to crates.io');
  console.log('  ✓ Python wheel to PyPI');
  console.log('  ✓ Node.js native to npm');
  console.log('  ✓ WASM package to npm');
  console.log('  ✓ C/C++ development package ready');
}

main().catch(error => {
  console.error('Release failed:', error);
  process.exit(1);
});
