# Multiple Tabs Design

## Overview

Design for supporting multiple tabs for both terminal and editor.

## Requirements

1. Multiple editor tabs (open multiple files)
2. Multiple terminal tabs
3. Close tabs with X button
4. Switch between tabs by clicking
5. Show modified indicator on unsaved files

## Data Structure

### Tab Types

```typescript
interface EditorTab {
  id: string;
  type: 'editor';
  filePath: string;
  modified: boolean;
}

interface TerminalTab {
  id: string;
  type: 'terminal';
  title: string;
  terminalId: number;
}

type Tab = EditorTab | TerminalTab;
```

### State

```typescript
interface TabState {
  tabs: Tab[];
  activeTabId: string | null;
}
```

## UI Design

```
┌─────────────────────────────────────────────────────────┐
│  [Terminal] [file1.ts ●] [file2.rs] [+ New]             │
├─────────────────────────────────────────────────────────┤
│                    Content Area                         │
└─────────────────────────────────────────────────────────┘
```

- Tabs scrollable if too many
- Active tab highlighted
- Modified indicator (●) for unsaved files
- X button on hover to close
- - button to add new terminal

## Implementation

### tabStore.ts

New store to manage tabs:

```typescript
const tabStore = {
  tabs: Tab[];
  activeTabId: string;

  addEditorTab(filePath: string);
  addTerminalTab();
  closeTab(id: string);
  setActiveTab(id: string);
  setModified(id: string, modified: boolean);
}
```

### Updates Required

1. Create tabStore
2. Update MainContent to use tabs
3. Create TabBar component
4. Update Editor to report modified state
5. Update Terminal to support multiple instances

## Implementation Steps

1. Create tabStore with tab management
2. Create TabBar component
3. Update MainContent to render based on active tab
4. Support multiple editor instances
5. Support multiple terminal instances
6. Add close functionality
7. Add new terminal button

## Acceptance Criteria

- [ ] Can open multiple files in tabs
- [ ] Can have multiple terminal tabs
- [ ] Can switch between tabs
- [ ] Can close tabs
- [ ] Modified files show indicator
- [ ] Can add new terminal with + button
