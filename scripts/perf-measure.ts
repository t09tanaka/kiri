#!/usr/bin/env tsx
/**
 * Performance Measurement Script
 *
 * Connects to a running Kiri app via MCP Bridge and measures performance metrics.
 *
 * Usage:
 *   1. Start the app: npm run tauri dev
 *   2. Run this script: npm run perf:measure
 *
 * Options:
 *   --port <number>  MCP Bridge port (default: 9225)
 *   --json           Output as JSON instead of formatted table
 */

import { WebSocket } from 'ws';
import { existsSync, readdirSync, statSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const projectRoot = join(__dirname, '..');

interface PerformanceResults {
  timestamp: string;
  memory: {
    rss_mb: number;
    vms_gb: number;
    platform: string;
  };
  startup: {
    first_contentful_paint_ms: number | null;
  };
  operations: {
    name: string;
    duration_ms: number;
  }[];
  bundleSize: {
    size_mb: number;
    target_mb: number;
  };
  targets: {
    startup_ms: number;
    memory_mb: number;
    bundle_mb: number;
  };
}

/**
 * Calculate directory size
 */
function getDirectorySize(dirPath: string): number {
  if (!existsSync(dirPath)) return 0;

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
 * MCP Bridge client for communicating with Tauri app
 */
class McpBridgeClient {
  private ws: WebSocket | null = null;
  private messageId = 0;
  private pendingRequests = new Map<
    string,
    { resolve: (value: unknown) => void; reject: (error: Error) => void }
  >();

  async connect(port: number): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(`ws://localhost:${port}`);

      this.ws.on('open', () => {
        console.log(`✅ Connected to MCP Bridge on port ${port}`);
        resolve();
      });

      this.ws.on('message', (data) => {
        try {
          const response = JSON.parse(data.toString());
          // MCP Bridge protocol uses string IDs
          const id = String(response.id);
          if (id && this.pendingRequests.has(id)) {
            const { resolve, reject } = this.pendingRequests.get(id)!;
            this.pendingRequests.delete(id);
            if (response.success === false || response.error) {
              reject(new Error(response.error || 'Unknown error'));
            } else {
              resolve(response.data);
            }
          }
        } catch {
          // Ignore non-JSON messages
        }
      });

      this.ws.on('error', (error) => {
        reject(new Error(`Failed to connect to MCP Bridge: ${error.message}`));
      });

      setTimeout(() => {
        reject(new Error('Connection timeout - is the app running?'));
      }, 5000);
    });
  }

  async executeJs(script: string): Promise<unknown> {
    if (!this.ws) throw new Error('Not connected');

    const id = String(++this.messageId);

    return new Promise((resolve, reject) => {
      this.pendingRequests.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
      });

      // Use MCP Bridge native protocol
      this.ws!.send(
        JSON.stringify({
          command: 'execute_js',
          args: {
            script,
          },
          id,
        })
      );

      setTimeout(() => {
        if (this.pendingRequests.has(id)) {
          this.pendingRequests.delete(id);
          reject(new Error('Timeout waiting for JS execution'));
        }
      }, 30000);
    });
  }

  close(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }
}

/**
 * Measure performance metrics
 */
async function measurePerformance(client: McpBridgeClient): Promise<PerformanceResults> {
  const projectRootEscaped = projectRoot.replace(/\\/g, '\\\\').replace(/'/g, "\\'");

  const measureScript = `
    (async () => {
      const invoke = window.__TAURI__.core.invoke;
      const results = {};

      // Memory metrics
      const memoryMetrics = await invoke('get_memory_metrics');
      results.memory = {
        rss_mb: memoryMetrics.rss / 1024 / 1024,
        vms_gb: memoryMetrics.vms / 1024 / 1024 / 1024,
        platform: memoryMetrics.platform
      };

      // First Contentful Paint
      let fcp = null;
      try {
        const paintEntries = performance.getEntriesByType('paint');
        const fcpEntry = paintEntries.find(e => e.name === 'first-contentful-paint');
        if (fcpEntry) fcp = fcpEntry.startTime;
      } catch (e) {}
      results.startup = { first_contentful_paint_ms: fcp };

      // Operation benchmarks
      const operations = [];
      const rootPath = '${projectRootEscaped}';

      // File operations
      let start = performance.now();
      await invoke('read_directory', { path: rootPath });
      operations.push({ name: 'read_directory', duration_ms: performance.now() - start });

      start = performance.now();
      await invoke('read_file', { path: rootPath + '/package.json' });
      operations.push({ name: 'read_file (small)', duration_ms: performance.now() - start });

      start = performance.now();
      await invoke('read_file', { path: rootPath + '/package-lock.json' });
      operations.push({ name: 'read_file (large)', duration_ms: performance.now() - start });

      // Search operations
      start = performance.now();
      await invoke('search_files', { rootPath: rootPath, query: 'performance', maxResults: 50 });
      operations.push({ name: 'search_files', duration_ms: performance.now() - start });

      start = performance.now();
      await invoke('search_content', { rootPath: rootPath, query: 'invoke', maxResults: 50 });
      operations.push({ name: 'search_content', duration_ms: performance.now() - start });

      // Git operations
      start = performance.now();
      await invoke('get_git_status', { path: rootPath });
      operations.push({ name: 'get_git_status', duration_ms: performance.now() - start });

      results.operations = operations;

      return JSON.stringify(results);
    })()
  `;

  const rawResult = (await client.executeJs(measureScript)) as string;

  // Parse the result - it might be wrapped in content array
  let jsResults: {
    memory: { rss_mb: number; vms_gb: number; platform: string };
    startup: { first_contentful_paint_ms: number | null };
    operations: { name: string; duration_ms: number }[];
  };

  if (typeof rawResult === 'string') {
    jsResults = JSON.parse(rawResult);
  } else if (typeof rawResult === 'object' && rawResult !== null) {
    // Handle MCP response format
    const content = (rawResult as { content?: Array<{ text?: string }> }).content;
    if (content && content[0] && content[0].text) {
      jsResults = JSON.parse(content[0].text);
    } else {
      throw new Error('Unexpected response format');
    }
  } else {
    throw new Error('Unexpected response type');
  }

  // Get bundle size
  const distPath = join(projectRoot, 'dist');
  const bundleSize = getDirectorySize(distPath);

  return {
    timestamp: new Date().toISOString(),
    memory: jsResults.memory,
    startup: jsResults.startup,
    operations: jsResults.operations,
    bundleSize: {
      size_mb: bundleSize / 1024 / 1024,
      target_mb: 10,
    },
    targets: {
      startup_ms: 1000,
      memory_mb: 50,
      bundle_mb: 10,
    },
  };
}

/**
 * Format results as table
 */
function formatResults(results: PerformanceResults): string {
  const lines: string[] = [];

  lines.push('');
  lines.push('╔════════════════════════════════════════════════════════════════╗');
  lines.push('║              Kiri Performance Measurement Report               ║');
  lines.push('╠════════════════════════════════════════════════════════════════╣');
  lines.push(`║  Timestamp: ${results.timestamp.padEnd(49)}║`);
  lines.push(`║  Platform:  ${results.memory.platform.padEnd(49)}║`);
  lines.push('╠════════════════════════════════════════════════════════════════╣');

  // Memory
  const memStatus = results.memory.rss_mb <= results.targets.memory_mb ? '✅' : '⚠️ ';
  lines.push('║  MEMORY                                                        ║');
  lines.push(
    `║    RSS:         ${results.memory.rss_mb.toFixed(2).padStart(8)} MB  (target: < ${results.targets.memory_mb} MB) ${memStatus.padEnd(7)}║`
  );

  // Startup
  lines.push('╠════════════════════════════════════════════════════════════════╣');
  lines.push('║  STARTUP                                                       ║');
  const fcp = results.startup.first_contentful_paint_ms;
  const fcpStatus = fcp !== null && fcp <= results.targets.startup_ms ? '✅' : '⚠️ ';
  lines.push(
    `║    FCP:         ${fcp !== null ? fcp.toFixed(0).padStart(8) : '     N/A'} ms  (target: < ${results.targets.startup_ms} ms) ${fcpStatus.padEnd(7)}║`
  );

  // Bundle size
  lines.push('╠════════════════════════════════════════════════════════════════╣');
  lines.push('║  BUNDLE SIZE                                                   ║');
  const bundleStatus = results.bundleSize.size_mb <= results.bundleSize.target_mb ? '✅' : '❌';
  lines.push(
    `║    Size:        ${results.bundleSize.size_mb.toFixed(2).padStart(8)} MB  (target: < ${results.bundleSize.target_mb} MB) ${bundleStatus.padEnd(7)}║`
  );

  // Operations
  lines.push('╠════════════════════════════════════════════════════════════════╣');
  lines.push('║  OPERATIONS                                                    ║');
  for (const op of results.operations) {
    const opStatus = op.duration_ms <= 100 ? '✅' : '⚠️ ';
    lines.push(
      `║    ${op.name.padEnd(20)} ${op.duration_ms.toFixed(2).padStart(8)} ms ${opStatus.padEnd(24)}║`
    );
  }

  lines.push('╚════════════════════════════════════════════════════════════════╝');
  lines.push('');

  return lines.join('\n');
}

/**
 * Main entry point
 */
async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const portIndex = args.indexOf('--port');
  const port = portIndex !== -1 ? parseInt(args[portIndex + 1], 10) : 9225;
  const jsonOutput = args.includes('--json');

  console.log('🔍 Kiri Performance Measurement');
  console.log('================================\n');

  const client = new McpBridgeClient();

  try {
    // Connect to running app
    await client.connect(port);

    // Measure performance
    console.log('📊 Measuring performance...\n');
    const results = await measurePerformance(client);

    // Output results
    if (jsonOutput) {
      console.log(JSON.stringify(results, null, 2));
    } else {
      console.log(formatResults(results));
    }

    // Summary
    const issues: string[] = [];
    if (results.memory.rss_mb > results.targets.memory_mb) {
      issues.push(
        `Memory (${results.memory.rss_mb.toFixed(1)}MB) exceeds target - expected in dev mode`
      );
    }
    if (results.bundleSize.size_mb > results.bundleSize.target_mb) {
      issues.push(`Bundle size (${results.bundleSize.size_mb.toFixed(1)}MB) exceeds target`);
    }

    if (issues.length === 0) {
      console.log('✅ All performance targets met!\n');
    } else {
      console.log('⚠️  Notes:');
      issues.forEach((issue) => console.log(`   - ${issue}`));
      console.log('');
    }
  } catch (error) {
    console.error(`\n❌ Error: ${(error as Error).message}`);
    console.log('\nMake sure the app is running: npm run tauri dev');
    process.exit(1);
  } finally {
    client.close();
  }
}

main();
