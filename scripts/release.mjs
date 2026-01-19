#!/usr/bin/env node

import { execSync } from 'child_process';
import { existsSync, mkdirSync, copyFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

const nextRelease = process.env.NEXT_RELEASE_VERSION || process.argv[2];
const isDryRun = process.argv.includes('--dry-run') || process.argv.includes('--dryRun') || process.env.DRY_RUN;

if (!nextRelease) {
  console.error('Error: NEXT_RELEASE_VERSION not set');
  process.exit(1);
}

console.log(`\n🚀 Starting release process for version ${nextRelease}${isDryRun ? ' (DRY RUN)' : ''}\n`);

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

  if (isDryRun) {
    console.log('[DRY RUN] Would publish Rust crates to crates.io');
  } else {
    run('cargo publish --manifest-path crates/sentinel-crypto/Cargo.toml', { cwd: workspaceRoot });
    run('cargo publish --manifest-path crates/sentinel/Cargo.toml', { cwd: workspaceRoot });
    run('cargo publish --manifest-path crates/cli/Cargo.toml', { cwd: workspaceRoot });
  }

  section('Creating C/C++ Development Package');

  const cxxBindings = join(workspaceRoot, 'bindings', 'cxx');
  const distDir = join(workspaceRoot, 'dist');
  mkdirSync(distDir, { recursive: true });

  const packageName = `sentinel-cxx-dev-${nextRelease}`;
  const stagingDir = join(distDir, packageName);
  const libDir = join(stagingDir, 'lib');

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
      console.log(`Copied dynamic library for ${target.system}`);
    }
    if (existsSync(staticLib)) {
      copyFileSync(staticLib, join(targetLibDir, `libsentinel_cxx.a`));
      console.log(`Copied static library for ${target.system}`);
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
    console.log(`Created: ${zipPath}`);
  }

  section('Building Python Wheel');

  const wheelsDir = join(workspaceRoot, 'target', 'wheels');
  const pythonBindings = join(workspaceRoot, 'bindings', 'python');

  if (!existsSync(pythonBindings)) {
    console.log('⚠️  bindings/python not found, skipping Python wheel');
  } else {
    mkdirSync(wheelsDir, { recursive: true });
    run(`maturin build --manifest-path ${join(workspaceRoot, 'crates', 'sentinel-python', 'Cargo.toml')} --release --out ${wheelsDir}`);
  }

  section('Publishing Python to PyPI');

  if (!existsSync(pythonBindings)) {
    console.log('⚠️  bindings/python not found, skipping PyPI upload');
  } else if (isDryRun) {
    console.log('[DRY RUN] Would upload Python wheel to PyPI');
  } else if (process.env.TWINE_USERNAME && process.env.TWINE_PASSWORD) {
    run(`twine upload ${wheelsDir}/*.whl --skip-existing --non-interactive`);
  } else {
    console.log('⚠️  TWINE_USERNAME or TWINE_PASSWORD not set, skipping PyPI upload');
  }

  section('Building Node.js Native Modules');

  const jsBindings = join(workspaceRoot, 'bindings', 'js');

  if (!existsSync(jsBindings)) {
    console.log('⚠️  bindings/js not found, skipping Node.js native module');
  } else {
    run('npm ci', { cwd: jsBindings });
    run(`cargo build --release -p sentinel-js`, { cwd: workspaceRoot });
    run(`cp ${join(workspaceRoot, 'crates', 'sentinel-js', 'target', 'release', '*.node')} ${join(jsBindings, 'native')}/ 2>/dev/null || true`, { cwd: workspaceRoot });
  }

  section('Publishing Node.js Native to npm');

  if (!existsSync(jsBindings)) {
    console.log('⚠️  bindings/js not found, skipping npm upload');
  } else if (isDryRun) {
    console.log('[DRY RUN] Would publish Node.js native module to npm');
  } else if (process.env.NPM_TOKEN) {
    run('npm publish', { cwd: jsBindings });
  } else {
    console.log('⚠️  NPM_TOKEN not set, skipping npm upload');
  }

  section(isDryRun ? 'Dry Run Complete!' : 'Release Complete!');

  if (isDryRun) {
    console.log('Dry run completed. No packages were published.');
  } else {
    console.log('Release completed successfully.');
  }
}

main().catch(error => {
  console.error('Release failed:', error);
  process.exit(1);
});
