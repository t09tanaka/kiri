#!/usr/bin/env tsx
/**
 * Bundle size report
 *
 * Walks `dist/assets/` and prints every emitted chunk sorted by size.
 * Useful for confirming that lazy-loaded modules (modals, editor
 * language packs, xterm addons) actually live in their own chunks
 * instead of being folded back into the startup graph.
 *
 * Run AFTER `npm run build`. Prints a markdown-friendly table so the
 * output is easy to paste into a PR description or commit message.
 *
 * Usage:
 *   npm run build && tsx scripts/perf-bundle-report.ts
 *   # or: tsx scripts/perf-bundle-report.ts --json
 */

import { existsSync, readdirSync, statSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const projectRoot = join(__dirname, '..');
const assetsDir = join(projectRoot, 'dist', 'assets');

interface Chunk {
  name: string;
  bytes: number;
  ext: string;
}

function bytesToKb(n: number): string {
  return (n / 1024).toFixed(2);
}

function categorize(name: string): string {
  if (name.startsWith('vendor-')) return 'vendor';
  if (name.startsWith('addon-')) return 'xterm-addon';
  if (name.startsWith('lang-') || name.startsWith('@codemirror_lang')) return 'codemirror-lang';
  if (/Modal\./.test(name)) return 'modal';
  if (name.startsWith('index-')) return 'app-or-route';
  if (name === 'xterm') return 'terminal';
  return 'other';
}

function readChunks(): Chunk[] {
  if (!existsSync(assetsDir)) {
    throw new Error(`No dist/assets directory at ${assetsDir}. Run \`npm run build\` first.`);
  }
  return readdirSync(assetsDir).map((file) => {
    const fullPath = join(assetsDir, file);
    const size = statSync(fullPath).size;
    const dotIdx = file.indexOf('.');
    return {
      name: dotIdx > 0 ? file.slice(0, dotIdx) : file,
      bytes: size,
      ext: dotIdx > 0 ? file.slice(dotIdx + 1) : '',
    };
  });
}

function main(): void {
  const jsonOutput = process.argv.includes('--json');
  const chunks = readChunks().sort((a, b) => b.bytes - a.bytes);

  if (jsonOutput) {
    console.log(JSON.stringify(chunks, null, 2));
    return;
  }

  // Group totals by category.
  const totals = new Map<string, number>();
  for (const c of chunks) {
    const cat = categorize(c.name);
    totals.set(cat, (totals.get(cat) ?? 0) + c.bytes);
  }

  console.log('# Bundle size report');
  console.log('');
  console.log('| chunk | size (kb) | ext | category |');
  console.log('| --- | ---: | --- | --- |');
  for (const c of chunks) {
    console.log(`| ${c.name} | ${bytesToKb(c.bytes)} | ${c.ext} | ${categorize(c.name)} |`);
  }
  console.log('');
  console.log('## Totals by category');
  console.log('| category | total (kb) |');
  console.log('| --- | ---: |');
  for (const [cat, bytes] of [...totals.entries()].sort((a, b) => b[1] - a[1])) {
    console.log(`| ${cat} | ${bytesToKb(bytes)} |`);
  }
  console.log('');
  console.log(
    `Total bundle: ${bytesToKb(chunks.reduce((sum, c) => sum + c.bytes, 0))} kb across ${chunks.length} files`
  );
}

main();
