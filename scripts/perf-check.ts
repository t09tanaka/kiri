#!/usr/bin/env tsx
/**
 * Performance check script for CI
 *
 * This script checks build artifacts against performance baselines.
 * Run with: npx tsx scripts/perf-check.ts [options]
 *
 * Options:
 *   --check-size     Check frontend bundle size
 *   --compare-baseline  Compare metrics against baseline
 *   --update-baseline   Update baseline with current values
 */

import { readFileSync, readdirSync, statSync, existsSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const projectRoot = join(__dirname, '..');

// Performance targets from CLAUDE.md
const TARGETS = {
  MAX_BUNDLE_SIZE: 10 * 1024 * 1024, // 10MB
  MAX_STARTUP_MS: 1000,
  MAX_MEMORY_BYTES: 50 * 1024 * 1024, // 50MB
};

interface Baselines {
  version: number;
  updated: string;
  targets: {
    [key: string]: {
      max: number;
      description?: string;
    };
  };
  current: {
    [key: string]: number | null;
  };
  history: Array<{
    date: string;
    commit?: string;
    startup_ms?: number;
    memory_bytes?: number;
    bundle_size_bytes?: number;
  }>;
}

/**
 * Calculate total size of a directory
 */
function getDirectorySize(dirPath: string): number {
  if (!existsSync(dirPath)) {
    return 0;
  }

  let totalSize = 0;

  function walkDir(currentPath: string): void {
    const entries = readdirSync(currentPath, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = join(currentPath, entry.name);
      if (entry.isDirectory()) {
        walkDir(fullPath);
      } else if (entry.isFile()) {
        totalSize += statSync(fullPath).size;
      }
    }
  }

  walkDir(dirPath);
  return totalSize;
}

/**
 * Format bytes as human-readable string
 */
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
}

/**
 * Check frontend bundle size
 */
async function checkBundleSize(): Promise<boolean> {
  console.log('\n📦 Checking bundle size...\n');

  const distPath = join(projectRoot, 'dist');

  if (!existsSync(distPath)) {
    console.log('⚠️  dist/ directory not found. Run "npm run build" first.');
    console.log('   Skipping bundle size check.\n');
    return true;
  }

  const bundleSize = getDirectorySize(distPath);
  const maxSize = TARGETS.MAX_BUNDLE_SIZE;
  const percentage = ((bundleSize / maxSize) * 100).toFixed(1);

  console.log(
    `Bundle size: ${formatBytes(bundleSize)} (${percentage}% of ${formatBytes(maxSize)} limit)`
  );

  if (bundleSize > maxSize) {
    console.log(`\n❌ FAIL: Bundle size exceeds limit!`);
    console.log(`   Current: ${formatBytes(bundleSize)}`);
    console.log(`   Maximum: ${formatBytes(maxSize)}`);
    return false;
  }

  console.log(`\n✅ PASS: Bundle size is within limits\n`);
  return true;
}

/**
 * Load baselines from file
 */
function loadBaselines(): Baselines | null {
  const baselinePath = join(projectRoot, 'perf-baselines', 'baselines.json');

  if (!existsSync(baselinePath)) {
    console.log('⚠️  baselines.json not found');
    return null;
  }

  try {
    const content = readFileSync(baselinePath, 'utf-8');
    return JSON.parse(content);
  } catch (error) {
    console.error('Failed to load baselines:', error);
    return null;
  }
}

/**
 * Save baselines to file
 */
function saveBaselines(baselines: Baselines): void {
  const baselinePath = join(projectRoot, 'perf-baselines', 'baselines.json');
  writeFileSync(baselinePath, JSON.stringify(baselines, null, 2) + '\n');
}

/**
 * Compare current metrics against baselines
 */
async function compareBaseline(): Promise<boolean> {
  console.log('\n📊 Comparing against baseline...\n');

  const baselines = loadBaselines();
  if (!baselines) {
    console.log('⚠️  No baselines to compare against');
    return true;
  }

  let allPassed = true;

  // Check bundle size against baseline target
  const distPath = join(projectRoot, 'dist');
  if (existsSync(distPath)) {
    const bundleSize = getDirectorySize(distPath);
    const maxBundleSize = baselines.targets.bundle_size_bytes?.max || TARGETS.MAX_BUNDLE_SIZE;

    console.log(`Bundle size: ${formatBytes(bundleSize)}`);
    console.log(`  Target: < ${formatBytes(maxBundleSize)}`);

    if (bundleSize > maxBundleSize) {
      console.log(`  ❌ FAIL: Exceeds target\n`);
      allPassed = false;
    } else {
      console.log(`  ✅ PASS\n`);
    }
  }

  // Summary
  if (allPassed) {
    console.log('✅ All baseline checks passed\n');
  } else {
    console.log('❌ Some baseline checks failed\n');
  }

  return allPassed;
}

/**
 * Update baselines with current values
 */
async function updateBaseline(): Promise<void> {
  console.log('\n📝 Updating baselines...\n');

  let baselines = loadBaselines();
  if (!baselines) {
    baselines = {
      version: 1,
      updated: new Date().toISOString(),
      targets: {
        startup_ms: { max: TARGETS.MAX_STARTUP_MS },
        memory_idle_bytes: { max: TARGETS.MAX_MEMORY_BYTES },
        bundle_size_bytes: { max: TARGETS.MAX_BUNDLE_SIZE },
      },
      current: {},
      history: [],
    };
  }

  // Update bundle size
  const distPath = join(projectRoot, 'dist');
  if (existsSync(distPath)) {
    const bundleSize = getDirectorySize(distPath);
    baselines.current.bundle_size_bytes = bundleSize;
    console.log(`Updated bundle_size_bytes: ${formatBytes(bundleSize)}`);
  }

  // Update timestamp
  baselines.updated = new Date().toISOString();

  // Add to history
  const historyEntry: Baselines['history'][0] = {
    date: new Date().toISOString().split('T')[0],
  };
  if (baselines.current.bundle_size_bytes) {
    historyEntry.bundle_size_bytes = baselines.current.bundle_size_bytes;
  }
  baselines.history.push(historyEntry);

  // Keep only last 30 history entries
  if (baselines.history.length > 30) {
    baselines.history = baselines.history.slice(-30);
  }

  saveBaselines(baselines);
  console.log('\n✅ Baselines updated\n');
}

/**
 * Main entry point
 */
async function main(): Promise<void> {
  const args = process.argv.slice(2);

  console.log('🔍 Performance Check');
  console.log('====================');

  let exitCode = 0;

  if (args.includes('--check-size')) {
    const passed = await checkBundleSize();
    if (!passed) exitCode = 1;
  }

  if (args.includes('--compare-baseline')) {
    const passed = await compareBaseline();
    if (!passed) exitCode = 1;
  }

  if (args.includes('--update-baseline')) {
    await updateBaseline();
  }

  if (args.length === 0) {
    console.log('\nUsage: npx tsx scripts/perf-check.ts [options]');
    console.log('\nOptions:');
    console.log('  --check-size        Check frontend bundle size');
    console.log('  --compare-baseline  Compare metrics against baseline');
    console.log('  --update-baseline   Update baseline with current values');
  }

  process.exit(exitCode);
}

main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});
