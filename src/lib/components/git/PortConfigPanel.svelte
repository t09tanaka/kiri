<script lang="ts">
  import { Spinner } from '@/lib/components/ui';
  import type {
    DetectedPorts,
    PortAssignment,
    CustomPortRule,
  } from '@/lib/services/portIsolationService';
  import { portIsolationService } from '@/lib/services/portIsolationService';

  interface Props {
    detected: DetectedPorts;
    startPort: number;
    customRules: CustomPortRule[];
    onApply: (assignments: PortAssignment[], nextPort: number) => void;
    onSkip: () => void;
    onUpdateCustomRules: (rules: CustomPortRule[]) => void;
  }

  let { detected, startPort, customRules, onApply, onSkip, onUpdateCustomRules }: Props = $props();

  // Local state for port selections (using regular Map, replaced on each change)
  let selectedPorts = $state<Map<string, boolean>>(new Map());
  let assignments = $state<PortAssignment[]>([]);
  let isAllocating = $state(false);
  let nextAllocatedPort = $state(startPort);

  // Custom rules local state
  let localCustomRules = $state<CustomPortRule[]>([]);
  let newRulePattern = $state('');
  let newRuleSearch = $state('');
  let showCustomRules = $state(false);

  // Initialize on mount
  $effect(() => {
    // Initialize selected ports (all checked by default)
    const uniquePorts = portIsolationService.getUniqueEnvPorts(detected);
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- intentionally using Map
    const newSelected = new Map<string, boolean>();
    for (const port of uniquePorts) {
      newSelected.set(port.variable_name, true);
    }
    selectedPorts = newSelected;

    // Initialize custom rules
    localCustomRules = [...customRules];

    // Allocate ports initially
    allocatePorts();
  });

  async function allocatePorts() {
    const selectedVars = new Set(
      Array.from(selectedPorts.entries())
        .filter(([, selected]) => selected)
        .map(([name]) => name)
    );

    if (selectedVars.size === 0) {
      assignments = [];
      nextAllocatedPort = startPort;
      return;
    }

    const uniquePorts = portIsolationService.getUniqueEnvPorts(detected);
    const portsToAllocate = uniquePorts.filter((p) => selectedVars.has(p.variable_name));

    isAllocating = true;
    try {
      const result = await portIsolationService.allocatePorts(portsToAllocate, startPort);
      assignments = result.assignments;
      nextAllocatedPort = result.next_port;
    } catch (e) {
      console.error('Failed to allocate ports:', e);
    } finally {
      isAllocating = false;
    }
  }

  function togglePort(variableName: string) {
    const current = selectedPorts.get(variableName) ?? true;
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- intentionally using Map
    selectedPorts = new Map(selectedPorts).set(variableName, !current);
    allocatePorts();
  }

  function getAssignedValue(variableName: string): number | null {
    const assignment = assignments.find((a) => a.variable_name === variableName);
    return assignment?.assigned_value ?? null;
  }

  function handleApply() {
    onApply(assignments, nextAllocatedPort);
    onUpdateCustomRules(localCustomRules);
  }

  function addCustomRule() {
    const pattern = newRulePattern.trim();
    const search = newRuleSearch.trim();
    if (!pattern || !search) return;

    const newRule: CustomPortRule = {
      id: `rule-${Date.now()}`,
      file_pattern: pattern,
      search_pattern: search,
      enabled: true,
    };

    localCustomRules = [...localCustomRules, newRule];
    newRulePattern = '';
    newRuleSearch = '';
  }

  function removeCustomRule(id: string) {
    localCustomRules = localCustomRules.filter((r) => r.id !== id);
  }

  function toggleCustomRule(id: string) {
    localCustomRules = localCustomRules.map((r) =>
      r.id === id ? { ...r, enabled: !r.enabled } : r
    );
  }

  // Get unique ports for display
  const uniqueEnvPorts = $derived(() => portIsolationService.getUniqueEnvPorts(detected));

  // Count of selected ports
  const selectedCount = $derived(() => Array.from(selectedPorts.values()).filter(Boolean).length);

  // Get source files for a variable
  function getSourceFiles(variableName: string): string[] {
    return detected.env_ports
      .filter((p) => p.variable_name === variableName)
      .map((p) => {
        const filename = p.file_path.split('/').pop() ?? p.file_path;
        return `${filename}:${p.line_number}`;
      });
  }
</script>

<div class="port-config-panel">
  <div class="panel-header">
    <div class="header-icon">
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
      >
        <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
        <line x1="8" y1="21" x2="16" y2="21"></line>
        <line x1="12" y1="17" x2="12" y2="21"></line>
      </svg>
    </div>
    <div class="header-text">
      <h3 class="panel-title">Port Configuration</h3>
      <p class="panel-subtitle">
        {uniqueEnvPorts().length} port variable{uniqueEnvPorts().length !== 1 ? 's' : ''} detected
      </p>
    </div>
  </div>

  <div class="panel-body">
    <!-- Auto-detected Variables -->
    <div class="section">
      <div class="section-header">
        <span class="section-title">Auto-detected Variables</span>
        {#if isAllocating}
          <Spinner size="sm" />
        {/if}
      </div>

      {#if uniqueEnvPorts().length === 0}
        <div class="empty-state">No port variables detected</div>
      {:else}
        <div class="port-table">
          <div class="table-header">
            <div class="col-check"></div>
            <div class="col-var">Variable</div>
            <div class="col-before">Before</div>
            <div class="col-after">After</div>
            <div class="col-source">Source</div>
          </div>
          {#each uniqueEnvPorts() as port (port.variable_name)}
            {@const isSelected = selectedPorts.get(port.variable_name) ?? true}
            {@const assigned = getAssignedValue(port.variable_name)}
            {@const sources = getSourceFiles(port.variable_name)}
            <div class="table-row" class:disabled={!isSelected}>
              <div class="col-check">
                <input
                  type="checkbox"
                  class="port-checkbox"
                  checked={isSelected}
                  onchange={() => togglePort(port.variable_name)}
                />
              </div>
              <div class="col-var">
                <code class="var-name">{port.variable_name}</code>
              </div>
              <div class="col-before">
                <span class="port-value original">{port.port_value}</span>
              </div>
              <div class="col-after">
                {#if isSelected && assigned !== null}
                  <span class="port-value new">{assigned}</span>
                {:else}
                  <span class="port-value unchanged">-</span>
                {/if}
              </div>
              <div class="col-source">
                <span class="source-files" title={sources.join(', ')}>
                  {sources[0]}{sources.length > 1 ? ` +${sources.length - 1}` : ''}
                </span>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Reference: Dockerfile & docker-compose (read-only) -->
    {#if detected.dockerfile_ports.length > 0 || detected.compose_ports.length > 0}
      <div class="section">
        <div class="section-header">
          <span class="section-title">Reference (not transformed)</span>
        </div>
        <div class="reference-list">
          {#each detected.dockerfile_ports as port, i (`dockerfile-${i}`)}
            <div class="reference-item">
              <span class="ref-label">Dockerfile</span>
              <code class="ref-value">EXPOSE {port.port_value}</code>
            </div>
          {/each}
          {#each detected.compose_ports as port, i (`compose-${i}`)}
            <div class="reference-item">
              <span class="ref-label">compose</span>
              <code class="ref-value">ports: {port.port_value}</code>
            </div>
          {/each}
        </div>
        <p class="reference-note">
          Dockerfile and docker-compose ports are shown for reference but not automatically
          transformed. Use custom rules to handle these files.
        </p>
      </div>
    {/if}

    <!-- Custom Rules (Collapsible) -->
    <div class="section">
      <button
        type="button"
        class="section-toggle"
        onclick={() => (showCustomRules = !showCustomRules)}
      >
        <svg
          class="toggle-icon"
          class:expanded={showCustomRules}
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <polyline points="9 18 15 12 9 6"></polyline>
        </svg>
        <span class="section-title">Custom Rules</span>
        {#if localCustomRules.length > 0}
          <span class="rule-count">{localCustomRules.length}</span>
        {/if}
      </button>

      {#if showCustomRules}
        <div class="custom-rules-content">
          {#if localCustomRules.length > 0}
            <div class="rules-list">
              {#each localCustomRules as rule (rule.id)}
                <div class="rule-item" class:disabled={!rule.enabled}>
                  <input
                    type="checkbox"
                    class="rule-checkbox"
                    checked={rule.enabled}
                    onchange={() => toggleCustomRule(rule.id)}
                  />
                  <div class="rule-info">
                    <code class="rule-pattern">{rule.file_pattern}</code>
                    <code class="rule-search">{rule.search_pattern}</code>
                  </div>
                  <button
                    type="button"
                    class="rule-remove"
                    onclick={() => removeCustomRule(rule.id)}
                    title="Remove rule"
                  >
                    <svg
                      width="12"
                      height="12"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                    >
                      <line x1="18" y1="6" x2="6" y2="18"></line>
                      <line x1="6" y1="6" x2="18" y2="18"></line>
                    </svg>
                  </button>
                </div>
              {/each}
            </div>
          {/if}

          <div class="add-rule-form">
            <input
              type="text"
              class="rule-input"
              bind:value={newRulePattern}
              placeholder="File pattern (e.g., config/*.json)"
              spellcheck="false"
              autocomplete="off"
              autocorrect="off"
              autocapitalize="off"
            />
            <input
              type="text"
              class="rule-input"
              bind:value={newRuleSearch}
              placeholder="Search pattern (e.g., "port": (\d+))"
              spellcheck="false"
              autocomplete="off"
              autocorrect="off"
              autocapitalize="off"
              onkeydown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault();
                  addCustomRule();
                }
              }}
            />
            <button
              type="button"
              class="add-rule-btn"
              onclick={() => addCustomRule()}
              disabled={!newRulePattern.trim() || !newRuleSearch.trim()}
            >
              Add
            </button>
          </div>
        </div>
      {/if}
    </div>
  </div>

  <div class="panel-footer">
    <div class="footer-info">
      {#if selectedCount() > 0}
        <span class="selected-count">{selectedCount()} selected</span>
      {/if}
    </div>
    <div class="footer-actions">
      <button type="button" class="btn btn-ghost" onclick={() => onSkip()}> Skip </button>
      <button
        type="button"
        class="btn btn-primary"
        onclick={() => handleApply()}
        disabled={isAllocating}
      >
        Apply to Worktree
      </button>
    </div>
  </div>
</div>

<style>
  .port-config-panel {
    display: flex;
    flex-direction: column;
    background: var(--bg-glass);
    border: 1px solid var(--border-glow);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: rgba(0, 0, 0, 0.2);
    border-bottom: 1px solid var(--border-color);
  }

  .header-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background: rgba(125, 211, 252, 0.1);
    border-radius: var(--radius-sm);
    color: var(--accent-color);
  }

  .header-text {
    flex: 1;
  }

  .panel-title {
    margin: 0;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .panel-subtitle {
    margin: 0;
    font-size: 11px;
    color: var(--text-muted);
  }

  .panel-body {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .section-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .section-toggle {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) 0;
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--text-secondary);
    transition: color var(--transition-fast);
  }

  .section-toggle:hover {
    color: var(--text-primary);
  }

  .toggle-icon {
    transition: transform var(--transition-fast);
  }

  .toggle-icon.expanded {
    transform: rotate(90deg);
  }

  .rule-count {
    font-size: 10px;
    font-weight: 500;
    padding: 2px 6px;
    background: rgba(125, 211, 252, 0.1);
    border-radius: 10px;
    color: var(--accent-color);
  }

  /* Port Table */
  .port-table {
    display: flex;
    flex-direction: column;
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .table-header {
    display: flex;
    padding: var(--space-2) var(--space-3);
    background: rgba(0, 0, 0, 0.15);
    font-size: 10px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .table-row {
    display: flex;
    padding: var(--space-2) var(--space-3);
    border-top: 1px solid var(--border-subtle);
    font-size: 12px;
    transition: opacity var(--transition-fast);
  }

  .table-row.disabled {
    opacity: 0.4;
  }

  .col-check {
    width: 24px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  .col-var {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .col-before,
  .col-after {
    width: 60px;
    flex-shrink: 0;
    text-align: right;
  }

  .col-source {
    width: 100px;
    flex-shrink: 0;
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .port-checkbox,
  .rule-checkbox {
    width: 14px;
    height: 14px;
    accent-color: var(--accent-color);
    cursor: pointer;
  }

  .var-name {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-primary);
  }

  .port-value {
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .port-value.original {
    color: var(--text-muted);
  }

  .port-value.new {
    color: var(--accent-color);
    font-weight: 500;
  }

  .port-value.unchanged {
    color: var(--text-muted);
    opacity: 0.5;
  }

  .source-files {
    font-size: 10px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  /* Reference Section */
  .reference-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .reference-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    font-size: 11px;
  }

  .ref-label {
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    padding: 2px 6px;
    background: rgba(125, 211, 252, 0.1);
    border-radius: 3px;
    color: var(--text-muted);
  }

  .ref-value {
    font-family: var(--font-mono);
    color: var(--text-secondary);
  }

  .reference-note {
    margin: var(--space-1) 0 0;
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.4;
  }

  /* Custom Rules */
  .custom-rules-content {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-top: var(--space-2);
  }

  .rules-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .rule-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    transition: opacity var(--transition-fast);
  }

  .rule-item.disabled {
    opacity: 0.4;
  }

  .rule-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .rule-pattern {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-primary);
  }

  .rule-search {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rule-remove {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .rule-remove:hover {
    background: rgba(248, 113, 113, 0.15);
    color: var(--git-deleted);
  }

  .add-rule-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-top: var(--space-1);
  }

  .rule-input {
    padding: var(--space-2) var(--space-3);
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-primary);
    outline: none;
    transition: border-color var(--transition-fast);
  }

  .rule-input:focus {
    border-color: var(--accent-color);
  }

  .rule-input::placeholder {
    color: var(--text-muted);
  }

  .add-rule-btn {
    align-self: flex-end;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-2) var(--space-4);
    background: linear-gradient(
      135deg,
      rgba(125, 211, 252, 0.08) 0%,
      rgba(196, 181, 253, 0.05) 100%
    );
    border: 1px solid rgba(125, 211, 252, 0.25);
    border-radius: var(--radius-sm);
    color: var(--accent-color);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .add-rule-btn:hover:not(:disabled) {
    border-color: rgba(125, 211, 252, 0.5);
    box-shadow: 0 0 12px rgba(125, 211, 252, 0.15);
  }

  .add-rule-btn:disabled {
    opacity: 0.25;
    cursor: not-allowed;
  }

  /* Footer */
  .panel-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    background: rgba(0, 0, 0, 0.2);
    border-top: 1px solid var(--border-color);
  }

  .footer-info {
    font-size: 11px;
    color: var(--text-muted);
  }

  .selected-count {
    padding: 2px 8px;
    background: rgba(125, 211, 252, 0.1);
    border-radius: 10px;
    color: var(--accent-color);
  }

  .footer-actions {
    display: flex;
    gap: var(--space-2);
  }

  /* Buttons */
  .btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: all var(--transition-fast);
    background: var(--bg-elevated);
    color: var(--text-secondary);
  }

  .btn:hover {
    background: var(--bg-glass-hover);
    color: var(--text-primary);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-primary {
    background: var(--accent-subtle);
    border-color: var(--accent-color);
    color: var(--accent-color);
  }

  .btn-primary:hover {
    background: var(--accent-muted);
  }

  .btn-ghost {
    background: transparent;
    border-color: transparent;
  }

  .btn-ghost:hover {
    background: rgba(125, 211, 252, 0.05);
  }

  /* Empty State */
  .empty-state {
    padding: var(--space-4);
    text-align: center;
    color: var(--text-muted);
    font-size: 12px;
  }

  /* Scrollbar */
  .panel-body::-webkit-scrollbar {
    width: 6px;
  }

  .panel-body::-webkit-scrollbar-track {
    background: transparent;
  }

  .panel-body::-webkit-scrollbar-thumb {
    background: var(--border-color);
    border-radius: 3px;
  }

  .panel-body:hover::-webkit-scrollbar-thumb {
    background: rgba(125, 211, 252, 0.3);
  }
</style>
