#!/usr/bin/env npx tsx

import { cpSync, existsSync, rmSync } from 'fs';
import { join } from 'path';

const ROOT_DIR = join(import.meta.dirname, '..');
const SOURCE_APP = join(ROOT_DIR, 'target', 'release', 'bundle', 'macos', 'kiri.app');
const DEST_APP = '/Applications/kiri.app';

function main(): void {
  if (process.platform !== 'darwin') {
    throw new Error('install:app is only supported on macOS');
  }

  if (!existsSync(SOURCE_APP)) {
    throw new Error(`Built app not found at ${SOURCE_APP}. Run npm run build:app first.`);
  }

  rmSync(DEST_APP, { recursive: true, force: true });
  cpSync(SOURCE_APP, DEST_APP, { recursive: true, preserveTimestamps: true });

  console.log(`Installed ${DEST_APP}`);
  console.log('Launch kiri once to refresh ~/.kiri/bin/kiri from the bundled CLI.');
}

main();
