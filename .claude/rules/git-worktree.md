# Git Worktree Specification

## Maintenance Rule

**When modifying worktree-related code, always update this specification file.**

This includes changes to:
- Naming conventions or path structures
- Branch validation logic
- Window lifecycle behavior
- API signatures or data structures
- UI components or keyboard shortcuts

## Overview

Git worktree allows working on multiple branches simultaneously by creating separate working directories that share the same `.git` directory.

## Repository Root Requirement

**Worktree functionality is only available when the project is opened from the repository root.**

When a user opens a subdirectory of a git repository (e.g., in a monorepo setup like `/repo/packages/my-package`), worktree management is disabled because:
- Worktrees operate at the repository level, not subdirectory level
- Creating worktrees from a subdirectory would cause path confusion

### Detection Logic

The `isSubdirectoryOfRepo` derived store compares:
- `currentPath`: The path the user opened as a project
- `main_repo_path` from `WorktreeContext`: The actual git repository root

If they differ, the user is in a subdirectory and worktree functionality is disabled.

### User Feedback

When `Cmd+G` is pressed from a subdirectory:
- A toast warning is displayed: "Worktrees can only be managed from the repository root"
- The WorktreePanel does not open

## Directory Naming

### Worktree Path Structure

```
{parent_dir}/{repo_name}-{worktree_name}/
```

Example:
- Repository: `/Users/user/projects/kiri`
- Worktree name: `feature-auth`
- Worktree path: `/Users/user/projects/kiri-feature-auth`

### Branch Name to Worktree Name Conversion

Branch names can contain `/` (e.g., `features/admin`), but directory names cannot.

| Branch Name | Worktree Name | Directory |
|-------------|---------------|-----------|
| `fix-bug` | `fix-bug` | `kiri-fix-bug` |
| `features/admin` | `features-admin` | `kiri-features-admin` |
| `a/b/c` | `a-b-c` | `kiri-a-b-c` |

Use `branchToWorktreeName()` from `@/lib/utils/gitWorktree` for conversion.

## Branch Constraints

### Cannot Create Worktree For

1. **Current branch** - The branch currently checked out in the main worktree
2. **In-use branch** - A branch already checked out in another worktree

### Branch Creation Modes

| Mode | Behavior |
|------|----------|
| Branch exists | Uses the existing branch for the worktree |
| Branch doesn't exist | Creates a new branch from the default branch, then creates worktree |

Note: The `newBranch` parameter is kept for API compatibility but the behavior is now unified - if a branch doesn't exist, it will be created automatically from the default branch regardless of this flag.

### Default Branch Detection

The default branch is determined in the following order:
1. `origin/HEAD` reference (set by `git clone` or `git remote set-head`)
2. Fallback to `main` if it exists locally
3. Fallback to `master` if it exists locally

## Window Lifecycle

### Create Flow

1. User enters branch name in WorktreePanel
2. Worktree is created via `worktreeService.create()`
3. Files are copied based on copy patterns (`**/.env*`, user-configured patterns). The `**/` prefix ensures `.env*` files in subdirectories are also copied.
4. Initialization commands are executed (e.g., `npm install`)
5. New window opens with the worktree path
6. WorktreePanel form resets

### Open Flow (Existing Worktree)

1. User clicks on an existing worktree in the list
2. Progress view shows with "Running initialization commands"
3. Initialization commands are executed (same as Create Flow step 4)
4. New window opens with the worktree path
5. Progress view resets

### Close Flow

1. User closes worktree window
2. Confirmation dialog appears: "Delete this worktree?"
3. If confirmed, worktree is removed via `worktreeService.remove()`
4. Event `worktree-removed` is emitted to notify other windows

## API Reference

### Frontend Service (`worktreeService`)

```typescript
// List all worktrees
list(repoPath: string): Promise<WorktreeInfo[]>

// Create a new worktree
create(repoPath: string, name: string, branch: string | null, newBranch: boolean): Promise<WorktreeInfo>

// Remove a worktree
remove(repoPath: string, name: string): Promise<void>

// Get worktree context
getContext(repoPath: string): Promise<WorktreeContext>

// List available branches
listBranches(repoPath: string): Promise<BranchInfo[]>

// Copy files matching patterns from source to target
copyFiles(sourcePath: string, targetPath: string, patterns: string[]): Promise<CopyResult>

// Detect package manager from lock files
detectPackageManager(projectPath: string): Promise<PackageManager | null>

// Run initialization command in worktree directory
runInitCommand(cwd: string, command: string): Promise<CommandOutput>
```

### Backend Commands (Rust)

Located in `src-tauri/src/commands/git_worktree.rs`:

- `list_worktrees` - Returns main worktree + linked worktrees
- `create_worktree` - Creates worktree with branch validation
- `remove_worktree` - Prunes worktree and removes directory
- `get_worktree_context` - Determines if current path is a worktree
- `list_branches` - Lists local branches (HEAD first, then alphabetical)
- `copy_files_to_worktree` - Copies files matching glob patterns
- `detect_package_manager` - Detects npm/yarn/pnpm/bun from lock files
- `run_init_command` - Executes shell command in specified directory

## Data Structures

### WorktreeInfo

```typescript
interface WorktreeInfo {
  name: string;        // Worktree name (e.g., "features-admin")
  path: string;        // Full path to worktree directory
  branch: string | null; // Branch name (e.g., "features/admin")
  is_locked: boolean;  // Whether worktree is locked
  is_main: boolean;    // Whether this is the main working tree
  is_valid: boolean;   // Whether worktree is valid
}
```

### WorktreeContext

```typescript
interface WorktreeContext {
  is_worktree: boolean;       // True if current path is a linked worktree
  main_repo_path: string | null; // Path to main repository
  worktree_name: string | null;  // Name of current worktree (if any)
}
```

### Derived Stores (`worktreeStore.ts`)

| Store | Type | Description |
|-------|------|-------------|
| `worktreeCount` | `number` | Count of linked worktrees (excludes main) |
| `isWorktree` | `boolean` | True if current path is a linked worktree |
| `isSubdirectoryOfRepo` | `boolean` | True if project path differs from repo root |

### PackageManager

```typescript
interface PackageManager {
  name: string;      // "npm", "yarn", "pnpm", "bun"
  lock_file: string; // "package-lock.json", etc.
  command: string;   // "npm install", etc.
}
```

### WorktreeInitCommand

```typescript
interface WorktreeInitCommand {
  name: string;      // Display name (e.g., "Install dependencies")
  command: string;   // Shell command (e.g., "npm install")
  enabled: boolean;  // Whether to run this command
  auto: boolean;     // True if auto-detected, false if user-added
}
```

## Initialization Commands

### Package Manager Detection

The system automatically detects the package manager from lock files:

| Lock File | Package Manager | Command |
|-----------|-----------------|---------|
| pnpm-lock.yaml | pnpm | `pnpm install` |
| yarn.lock | yarn | `yarn install` |
| bun.lockb | bun | `bun install` |
| package-lock.json | npm | `npm install` |

If multiple lock files exist, priority is: pnpm > yarn > bun > npm.

If only `package.json` exists (no lock file), defaults to `npm install`.

### Custom Commands

Users can add custom initialization commands via Settings modal:
- Name: Display name for the command
- Command: Shell command to execute
- Can be enabled/disabled individually

Commands run in order after worktree creation and file copying.

## UI Components

### WorktreePanel (`src/lib/components/git/WorktreePanel.svelte`)

- Branch name input with existing branch selector
- Path preview showing converted directory name
- Validation for current/in-use branches
- Tree visualization of main repo and worktrees
- Progress view during creation (steps: worktree → copy → init → done)
- Settings modal for copy patterns and initialization commands

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+G` | Toggle Worktrees panel (only at repo root) |
| `Escape` | Close panel |

## Incremental Replace

Incremental Replace automatically transforms numeric values in `.env` files to prevent conflicts when running multiple worktrees simultaneously. While the primary use case is port numbers, any numeric variable matching the detection pattern will be transformed.

### Port Detection

The system scans the repository (including subdirectories) for port variables:

| File Type | Pattern | Example | Action |
|-----------|---------|---------|--------|
| `**/.env*` | `^([A-Z_]*PORT[A-Z_]*)=(\d+)` | `PORT=3000`, `DB_PORT=3306` | **Transformed** |
| `**/Dockerfile` | `^EXPOSE\s+(\d+)` | `EXPOSE 3000` | Reference only |
| `**/docker-compose.yml` | `ports:` section | `- "3000:3000"` | **Transformed** |
| `**/package.json` | `-p`, `--port` flags in scripts | `"dev": "next dev -p 3000"` | **Transformed** |

### Port Allocation

- **Port Range**: 20000-39999
- Unique ports are allocated sequentially for each worktree
- Same variable names across multiple `.env*` files get the same assigned port
- Port assignments are stored per-project in `kiri-settings.json`

### Port Transformation Flow

1. WorktreePanel opens → Ports are automatically detected
2. Port configuration is displayed in Settings panel:
   - Toggle to enable/disable incremental replace
   - Table showing detected variables with Before/After values
   - Checkboxes to include/exclude individual variables
3. User clicks "Create" on a branch
4. Files are copied with port transformation applied:
   - `.env*` files: variable-aware transformation (preserves variable names)
   - `docker-compose*.yml` / `compose.yml`: host-only transformation (container ports stay fixed)
   - Other files: generic port number replacement
   - `nextPort` is updated for future worktrees

### Data Structures

#### PortConfig (Stored in ProjectSettings)

```typescript
interface PortConfig {
  enabled: boolean;           // Enable incremental replace
  portRangeStart: number;     // Default: 20000
  portRangeEnd: number;       // Default: 39999
  nextPort: number;           // Next port to allocate
  customRules: CustomPortRule[];
}
```

#### PortAssignment

```typescript
interface PortAssignment {
  variable_name: string;      // "PORT", "DB_PORT"
  original_value: number;     // 3000
  assigned_value: number;     // 20001
}
```

#### DetectedPorts

```typescript
interface DetectedPorts {
  env_ports: PortSource[];        // From .env* files (transformed)
  dockerfile_ports: PortSource[]; // From Dockerfile (reference)
  compose_ports: PortSource[];    // From docker-compose (transformed)
  script_ports: PortSource[];     // From package.json scripts (transformed)
}

interface PortSource {
  file_path: string;
  variable_name: string;
  port_value: number;
  line_number: number;
}
```

#### CustomPortRule

```typescript
interface CustomPortRule {
  id: string;
  file_pattern: string;      // Glob: "config/*.json"
  search_pattern: string;    // Regex: '"port":\s*(\d+)'
  enabled: boolean;
}
```

### Frontend Service (`portIsolationService`)

```typescript
// Detect ports in a directory
detectPorts(dirPath: string): Promise<DetectedPorts>

// Allocate unique ports
allocatePorts(ports: PortSource[], startPort: number): Promise<PortAllocationResult>

// Copy files with port transformation
copyFilesWithPorts(
  sourcePath: string,
  targetPath: string,
  patterns: string[],
  assignments: PortAssignment[]
): Promise<CopyResult>

// Apply custom rules
applyCustomRules(
  sourcePath: string,
  targetPath: string,
  rules: CustomPortRule[],
  portOffset: number
): Promise<CustomRuleReplacement[]>
```

### Backend Commands (Rust)

Located in `src-tauri/src/commands/port_isolation.rs`:

- `detect_ports` - Scans directory for port variables (.env, Dockerfile, docker-compose, package.json)
- `allocate_worktree_ports` - Allocates unique ports from range
- `copy_files_with_ports` - Copies files with `.env` transformation
- `apply_port_custom_rules` - Applies custom regex rules to files

### UI Components

Incremental Replace settings are integrated into the WorktreePanel Settings modal:

- Toggle switch to enable/disable incremental replace
- Table of auto-detected variables with Before/After values
- Checkboxes to include/exclude individual variables
- Source file references
- Reference section for Dockerfile/docker-compose ports (not transformed)
