<script lang="ts">
  import type { CommitInfo } from '@/lib/services/gitService';

  interface Props {
    commits: CommitInfo[];
    selectedHash: string | null;
    onSelectCommit: (commit: CommitInfo) => void;
    isLoadingMore?: boolean;
    hasMore?: boolean;
    onLoadMore?: () => void;
    unreadHashes?: Set<string>;
  }

  let {
    commits,
    selectedHash,
    onSelectCommit,
    isLoadingMore = false,
    hasMore = false,
    onLoadMore,
    unreadHashes = new Set(),
  }: Props = $props();

  let scrollContainer: HTMLDivElement | undefined = $state();

  const NODE_RADIUS = 4;
  const NODE_RADIUS_ACTIVE = 5;
  const ROW_HEIGHT = 48;
  const COL_WIDTH = 24;
  const GRAPH_PADDING = 16;
  const BRANCH_GAP = 16;

  // Index where current branch ends and base/shared begins (-1 if no boundary)
  const branchBoundaryIndex = $derived(commits.findIndex((c) => isGrayedOut(c)));
  const hasBranchGap = $derived(branchBoundaryIndex > 0);

  // Compute graph width dynamically based on actual column count
  const maxColumn = $derived(Math.max(0, ...commits.map((c) => c.graph_column)));
  const graphWidth = $derived(GRAPH_PADDING + (maxColumn + 1) * COL_WIDTH);
  const textStart = $derived(graphWidth + 10);

  // Build hash -> row index map for drawing connections
  const hashToRow = $derived(new Map(commits.map((c, i) => [c.full_hash, i])));

  function isGrayedOut(commit: CommitInfo): boolean {
    return commit.branch_type === 'base' || commit.branch_type === 'shared';
  }

  // Explicit colors - no opacity-based dimming
  // Mist palette: dim colors blend into the dark background naturally
  const COLORS = {
    // Active nodes
    unpushed: '#fcd34d',
    pushed: '#7dd3fc',
    // Grayed-out (mist-faded)
    fadedNode: '#2d3545',
    fadedLine: '#232b38',
    fadedText: '#3d4554',
    fadedDate: '#2d3544',
    // Lines
    activeLine: '#4a6a85',
    unpushedLine: '#7a6a30',
    // Selection
    selectedBg: 'rgba(125, 211, 252, 0.08)',
    hoverBg: 'rgba(125, 211, 252, 0.04)',
  } as const;

  function getNodeColor(commit: CommitInfo): string {
    if (isGrayedOut(commit)) return COLORS.fadedNode;
    if (!commit.is_pushed) return COLORS.unpushed;
    return COLORS.pushed;
  }

  function getLineColor(commit: CommitInfo): string {
    if (isGrayedOut(commit)) return COLORS.fadedLine;
    if (!commit.is_pushed) return COLORS.unpushedLine;
    return COLORS.activeLine;
  }

  function getMessageColor(commit: CommitInfo): string {
    if (isGrayedOut(commit)) return COLORS.fadedText;
    return '#c9d1d9';
  }

  function getDateColor(commit: CommitInfo): string {
    if (isGrayedOut(commit)) return COLORS.fadedDate;
    return '#484f58';
  }

  function getNodeX(commit: CommitInfo): number {
    return GRAPH_PADDING / 2 + commit.graph_column * COL_WIDTH + COL_WIDTH / 2;
  }

  function getGapOffset(index: number): number {
    return hasBranchGap && index >= branchBoundaryIndex ? BRANCH_GAP : 0;
  }

  function getNodeY(index: number): number {
    return index * ROW_HEIGHT + ROW_HEIGHT / 2 + getGapOffset(index);
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
    if (text.length <= max) return text;
    const cut = text.lastIndexOf(' ', max);
    return (cut > max * 0.5 ? text.slice(0, cut) : text.slice(0, max)) + '...';
  }

  function parseCommitMessage(message: string): { prefix: string | null; subject: string } {
    const firstLine = message.split('\n')[0];
    const colonIndex = firstLine.indexOf(':');
    if (colonIndex > 0 && colonIndex < 30) {
      return {
        prefix: firstLine.slice(0, colonIndex + 1),
        subject: firstLine.slice(colonIndex + 1).trim(),
      };
    }
    return { prefix: null, subject: firstLine };
  }

  function getPrefixColor(commit: CommitInfo): string {
    if (isGrayedOut(commit)) return COLORS.fadedDate;
    return '#8b949e';
  }

  function handleScroll() {
    if (!scrollContainer || !hasMore || isLoadingMore || !onLoadMore) return;
    const { scrollTop, scrollHeight, clientHeight } = scrollContainer;
    if (scrollHeight - scrollTop - clientHeight < 200) {
      onLoadMore();
    }
  }

  // Build parent connections for drawing lines
  interface Connection {
    x1: number;
    y1: number;
    x2: number;
    y2: number;
    color: string;
  }

  const connections = $derived.by(() => {
    const result: Connection[] = [];
    for (let i = 0; i < commits.length; i++) {
      const commit = commits[i];
      const x1 = getNodeX(commit);
      const y1 = getNodeY(i);
      const color = getLineColor(commit);

      for (const parentId of commit.parent_ids) {
        const parentRow = hashToRow.get(parentId);
        if (parentRow === undefined) continue;

        const parent = commits[parentRow];
        const x2 = getNodeX(parent);
        const y2 = getNodeY(parentRow);

        const isGrayConnection = isGrayedOut(commit) || isGrayedOut(parent);
        result.push({
          x1,
          y1,
          x2,
          y2,
          color: isGrayConnection ? COLORS.fadedLine : color,
        });
      }
    }
    return result;
  });
</script>

<div class="commit-graph">
  <div class="graph-scroll" bind:this={scrollContainer} onscroll={handleScroll}>
    <svg width="100%" height={commits.length * ROW_HEIGHT + (isLoadingMore ? ROW_HEIGHT : 0)}>
      <!-- Connection lines -->
      {#each connections as conn, connIdx (connIdx)}
        {#if conn.x1 === conn.x2}
          <line
            x1={conn.x1}
            y1={conn.y1}
            x2={conn.x2}
            y2={conn.y2}
            stroke={conn.color}
            stroke-width="1.5"
          />
        {:else}
          <!-- From child straight down, then diagonal to parent -->
          {@const dx = Math.abs(conn.x2 - conn.x1)}
          {@const dy = conn.y2 - conn.y1}
          {#if dy > dx}
            <!-- Enough vertical space: straight down then 45° diagonal -->
            <path
              d="M {conn.x1},{conn.y1} L {conn.x1},{conn.y2 - dx} L {conn.x2},{conn.y2}"
              fill="none"
              stroke={conn.color}
              stroke-width="1.5"
              stroke-linejoin="round"
            />
          {:else}
            <!-- Not enough vertical space: direct diagonal -->
            <line
              x1={conn.x1}
              y1={conn.y1}
              x2={conn.x2}
              y2={conn.y2}
              stroke={conn.color}
              stroke-width="1.5"
            />
          {/if}
        {/if}
      {/each}

      <!-- Commit rows -->
      {#each commits as commit, i (commit.full_hash)}
        {@const nodeX = getNodeX(commit)}
        {@const nodeY = getNodeY(i)}
        {@const nodeColor = getNodeColor(commit)}
        {@const isSelected = selectedHash === commit.full_hash}
        {@const grayed = isGrayedOut(commit)}
        {@const radius = grayed ? NODE_RADIUS : NODE_RADIUS_ACTIVE}

        <!-- Row background (hover/selection) -->
        <rect
          x="0"
          y={i * ROW_HEIGHT + getGapOffset(i)}
          width="100%"
          height={ROW_HEIGHT}
          fill={isSelected ? COLORS.selectedBg : 'transparent'}
          class="row-bg"
          onclick={() => onSelectCommit(commit)}
        />

        <!-- Node circle -->
        <circle cx={nodeX} cy={nodeY} r={radius} fill={nodeColor} style="pointer-events: none;" />

        {@const parsed = parseCommitMessage(commit.message)}

        <!-- Commit type prefix (e.g., "feat(git):") -->
        {#if parsed.prefix}
          <text x={textStart} y={nodeY - 10} class="commit-prefix" fill={getPrefixColor(commit)}>
            {parsed.prefix}
          </text>
        {/if}

        <!-- Commit subject text -->
        <text x={textStart} y={nodeY + 4} class="commit-message" fill={getMessageColor(commit)}>
          {truncate(parsed.subject || parsed.prefix || '', 35)}
        </text>

        <!-- Date text -->
        <text x={textStart} y={nodeY + 16} class="commit-date" fill={getDateColor(commit)}>
          {formatDate(commit.date)}
        </text>

        <!-- Unread indicator dot (4px right of date text) -->
        {#if unreadHashes.has(commit.full_hash) && !grayed}
          <circle
            cx={textStart + formatDate(commit.date).length * 6 + 6.5}
            cy={nodeY + 12}
            r="2.5"
            fill="#7dd3fc"
            style="pointer-events: none;"
          />
        {/if}
      {/each}

      <!-- Loading more indicator -->
      {#if isLoadingMore}
        <text
          x={textStart}
          y={commits.length * ROW_HEIGHT + ROW_HEIGHT / 2}
          class="loading-text"
          fill={COLORS.fadedText}
        >
          Loading more...
        </text>
      {/if}
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
    scrollbar-color: rgba(125, 211, 252, 0.15) transparent;
  }

  .graph-scroll::-webkit-scrollbar {
    width: 5px;
  }

  .graph-scroll::-webkit-scrollbar-track {
    background: transparent;
  }

  .graph-scroll::-webkit-scrollbar-thumb {
    background: rgba(125, 211, 252, 0.15);
    border-radius: 3px;
  }

  .graph-scroll::-webkit-scrollbar-thumb:hover {
    background: rgba(125, 211, 252, 0.25);
  }

  .row-bg {
    cursor: pointer;
    transition: fill 0.15s ease;
  }

  .row-bg:hover {
    fill: rgba(125, 211, 252, 0.04);
  }

  .commit-prefix {
    font-size: 10px;
    font-family: var(--font-mono);
    pointer-events: none;
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

  .loading-text {
    font-size: 11px;
    font-family: var(--font-body);
    pointer-events: none;
  }
</style>
