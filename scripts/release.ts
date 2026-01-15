#!/usr/bin/env npx tsx
/**
 * Release script for Kiri
 *
 * Usage:
 *   npx tsx scripts/release.ts <version>
 *   npx tsx scripts/release.ts patch    # 0.0.1 -> 0.0.2
 *   npx tsx scripts/release.ts minor    # 0.0.1 -> 0.1.0
 *   npx tsx scripts/release.ts major    # 0.0.1 -> 1.0.0
 *   npx tsx scripts/release.ts 1.2.3    # Set specific version
 */

import { execSync } from 'child_process';
import { readFileSync, writeFileSync } from 'fs';
import { join } from 'path';

const ROOT_DIR = join(import.meta.dirname, '..');
const PACKAGE_JSON = join(ROOT_DIR, 'package.json');
const TAURI_CONF = join(ROOT_DIR, 'src-tauri', 'tauri.conf.json');
const CARGO_TOML = join(ROOT_DIR, 'src-tauri', 'Cargo.toml');

function getCurrentVersion(): string {
  const pkg = JSON.parse(readFileSync(PACKAGE_JSON, 'utf-8'));
  return pkg.version;
}

function bumpVersion(current: string, type: string): string {
  const [major, minor, patch] = current.split('.').map(Number);

  switch (type) {
    case 'major':
      return `${major + 1}.0.0`;
    case 'minor':
      return `${major}.${minor + 1}.0`;
    case 'patch':
      return `${major}.${minor}.${patch + 1}`;
    default:
      // Assume it's a specific version
      if (/^\d+\.\d+\.\d+$/.test(type)) {
        return type;
      }
      throw new Error(`Invalid version type: ${type}`);
  }
}

function updatePackageJson(version: string): void {
  const pkg = JSON.parse(readFileSync(PACKAGE_JSON, 'utf-8'));
  pkg.version = version;
  writeFileSync(PACKAGE_JSON, JSON.stringify(pkg, null, 2) + '\n');
  console.log(`✅ Updated package.json to ${version}`);
}

function updateTauriConf(version: string): void {
  const conf = JSON.parse(readFileSync(TAURI_CONF, 'utf-8'));
  conf.version = version;
  writeFileSync(TAURI_CONF, JSON.stringify(conf, null, 2) + '\n');
  console.log(`✅ Updated tauri.conf.json to ${version}`);
}

function updateCargoToml(version: string): void {
  let content = readFileSync(CARGO_TOML, 'utf-8');
  content = content.replace(/^version = ".*"$/m, `version = "${version}"`);
  writeFileSync(CARGO_TOML, content);
  console.log(`✅ Updated Cargo.toml to ${version}`);
}

function createGitTag(version: string, push: boolean): void {
  const tag = `v${version}`;

  // Stage changes
  execSync('git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml', {
    cwd: ROOT_DIR,
    stdio: 'inherit',
  });

  // Commit
  execSync(`git commit -m "chore: release ${tag}"`, {
    cwd: ROOT_DIR,
    stdio: 'inherit',
  });
  console.log(`✅ Created commit for ${tag}`);

  // Create tag
  execSync(`git tag -a ${tag} -m "Release ${tag}"`, {
    cwd: ROOT_DIR,
    stdio: 'inherit',
  });
  console.log(`✅ Created tag ${tag}`);

  if (push) {
    execSync(`git push && git push origin ${tag}`, {
      cwd: ROOT_DIR,
      stdio: 'inherit',
    });
    console.log(`✅ Pushed to remote`);
  } else {
    console.log(`\n📦 To push the release, run:`);
    console.log(`   git push && git push origin ${tag}`);
  }
}

function main(): void {
  const args = process.argv.slice(2);
  const versionArg = args[0];
  const shouldPush = args.includes('--push');

  if (!versionArg) {
    console.log('Usage: npx tsx scripts/release.ts <patch|minor|major|x.y.z> [--push]');
    console.log('\nExamples:');
    console.log('  npx tsx scripts/release.ts patch        # Bump patch version');
    console.log('  npx tsx scripts/release.ts minor        # Bump minor version');
    console.log('  npx tsx scripts/release.ts 1.0.0        # Set specific version');
    console.log('  npx tsx scripts/release.ts patch --push # Bump and push');
    process.exit(1);
  }

  const currentVersion = getCurrentVersion();
  const newVersion = bumpVersion(currentVersion, versionArg);

  console.log(`\n🚀 Releasing Kiri`);
  console.log(`   Current version: ${currentVersion}`);
  console.log(`   New version: ${newVersion}\n`);

  // Update version in all files
  updatePackageJson(newVersion);
  updateTauriConf(newVersion);
  updateCargoToml(newVersion);

  // Create git tag
  createGitTag(newVersion, shouldPush);

  console.log(`\n🎉 Release ${newVersion} complete!`);
}

main();
