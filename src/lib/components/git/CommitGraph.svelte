<script lang="ts">
  import type { CommitInfo } from '@/lib/services/gitService';

  interface Props {
    commits: CommitInfo[];
    selectedHash: string | null;
    onSelectCommit: (commit: CommitInfo) => void;
  }

  let { commits, selectedHash, onSelectCommit }: Props = $props();

  const NODE_RADIUS = 5;
  const ROW_HEIGHT = 40;
  const COL_WIDTH = 24;
  const GRAPH_WIDTH = 60;
  const TEXT_START = GRAPH_WIDTH + 8;

  // Build hash -> row index map for drawing connections
  const hashToRow = $derived(new Map(commits.map((c, i) => [c.full_hash, i])));

  function getNodeColor(commit: CommitInfo): string {
    if (!commit.is_pushed) return '#fcd34d';
    if (commit.branch_type === 'current' || commit.branch_type === 'both') return '#7dd3fc';
    return '#c4b5fd';
  }

  function getNodeX(commit: CommitInfo): number {
    return GRAPH_WIDTH / 2 + commit.graph_column * COL_WIDTH;
  }

  function getNodeY(index: number): number {
    return index * ROW_HEIGHT + ROW_HEIGHT / 2;
  }

  function formatDate(timestamp: number): string {
    const now = new Date();
    const date = new Date(timestamp * 1000);
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays === 0) return 'today';
    if (diffDays === 1) return 'yesterday';
    if (diffDays < 7) return `${diffDays}d ago`;
    if (diffDays < 365) {
      return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
    }
    return date.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    });
  }

  function truncate(text: string, max: number): string {
    return text.length > max ? text.slice(0, max) + '...' : text;
  }

  // Build parent connections for drawing lines
  interface Connection {
    x1: number;
    y1: number;
    x2: number;
    y2: number;
    color: string;
    dashed: boolean;
  }

  const connections = $derived.by(() => {
    const result: Connection[] = [];
    for (let i = 0; i < commits.length; i++) {
      const commit = commits[i];
      const x1 = getNodeX(commit);
      const y1 = getNodeY(i);
      const color = getNodeColor(commit);

      for (const parentId of commit.parent_ids) {
        const parentRow = hashToRow.get(parentId);
        if (parentRow === undefined) continue;

        const parent = commits[parentRow];
        const x2 = getNodeX(parent);
        const y2 = getNodeY(parentRow);

        result.push({
          x1,
          y1,
          x2,
          y2,
          color,
          dashed: !commit.is_pushed,
        });
      }
    }
    return result;
  });
</script>

<div class="commit-graph">
  <div class="graph-scroll">
    <svg width="100%" height={commits.length * ROW_HEIGHT}>
      <defs>
        <filter id="glow-filter">
          <feGaussianBlur stdDeviation="2" result="coloredBlur" />
          <feMerge>
            <feMergeNode in="coloredBlur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      <!-- Connection lines -->
      {#each connections as conn, connIdx (connIdx)}
        {#if conn.x1 === conn.x2}
          <!-- Straight vertical line -->
          <line
            x1={conn.x1}
            y1={conn.y1}
            x2={conn.x2}
            y2={conn.y2}
            stroke={conn.color}
            stroke-width="2"
            stroke-dasharray={conn.dashed ? '4,3' : 'none'}
            opacity="0.6"
          />
        {:else}
          <!-- Bezier curve for different columns -->
          {@const midY = (conn.y1 + conn.y2) / 2}
          <path
            d="M {conn.x1},{conn.y1} C {conn.x1},{midY} {conn.x2},{midY} {conn.x2},{conn.y2}"
            fill="none"
            stroke={conn.color}
            stroke-width="2"
            stroke-dasharray={conn.dashed ? '4,3' : 'none'}
            opacity="0.6"
          />
        {/if}
      {/each}

      <!-- Commit rows -->
      {#each commits as commit, i (commit.full_hash)}
        {@const nodeX = getNodeX(commit)}
        {@const nodeY = getNodeY(i)}
        {@const nodeColor = getNodeColor(commit)}
        {@const isSelected = selectedHash === commit.full_hash}

        <!-- Row background (hover/selection) -->
        <rect
          x="0"
          y={i * ROW_HEIGHT}
          width="100%"
          height={ROW_HEIGHT}
          fill={isSelected ? 'rgba(125, 211, 252, 0.1)' : 'transparent'}
          class="row-bg"
          onclick={() => onSelectCommit(commit)}
        />

        <!-- Node circle -->
        <circle
          cx={nodeX}
          cy={nodeY}
          r={NODE_RADIUS}
          fill={nodeColor}
          filter={!commit.is_pushed ? 'url(#glow-filter)' : undefined}
          style="pointer-events: none;"
        />

        <!-- Commit message text -->
        <text x={TEXT_START} y={nodeY - 4} class="commit-message" fill="var(--text-primary)">
          {truncate(commit.message.split('\n')[0], 28)}
        </text>

        <!-- Date text -->
        <text x={TEXT_START} y={nodeY + 12} class="commit-date" fill="var(--text-muted)">
          {formatDate(commit.date)}
        </text>
      {/each}
    </svg>
  </div>
</div>

<style>
  .commit-graph {
    height: 100%;
    overflow: hidden;
    background: transparent;
  }

  .graph-scroll {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: thin;
    scrollbar-color: rgba(125, 211, 252, 0.2) transparent;
  }

  .graph-scroll::-webkit-scrollbar {
    width: 6px;
  }

  .graph-scroll::-webkit-scrollbar-track {
    background: transparent;
  }

  .graph-scroll::-webkit-scrollbar-thumb {
    background: rgba(125, 211, 252, 0.2);
    border-radius: 3px;
  }

  .graph-scroll::-webkit-scrollbar-thumb:hover {
    background: rgba(125, 211, 252, 0.3);
  }

  .row-bg {
    cursor: pointer;
    transition: fill 0.15s ease;
  }

  .row-bg:hover {
    fill: rgba(125, 211, 252, 0.05);
  }

  .commit-message {
    font-size: 12px;
    font-family: var(--font-body);
    pointer-events: none;
  }

  .commit-date {
    font-size: 10px;
    font-family: var(--font-mono);
    pointer-events: none;
  }
</style>
