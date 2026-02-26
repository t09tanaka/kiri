# Docker Compose Fixed Ports Design

## Problem

When port isolation transforms ports for worktrees, docker-compose port mappings like `5432:5432` have **both** host and container ports transformed to `5532:5532`. Container-internal ports must remain fixed because services (PostgreSQL, MySQL, MariaDB, etc.) listen on their standard ports inside the container.

## Solution

Introduce a dedicated `transform_compose_content()` function that understands docker-compose `host:container` port mapping syntax and only transforms the host side.

### Current Flow

```
.env*       → transform_env_content()
everything  → transform_generic_content()  ← compose included, both sides replaced
```

### New Flow

```
.env*                → transform_env_content()
docker-compose*.yml  → transform_compose_content()  ← NEW: host-side only
everything else      → transform_generic_content()
```

## Transformation Example

```yaml
# Original
services:
  db:
    ports:
      - "5432:5432"
      - "3000:3000"

# After (worktree offset +100)
services:
  db:
    ports:
      - "5532:5432"    # host transformed, container fixed
      - "3100:3000"    # host transformed, container fixed
```

## Implementation

### `transform_compose_content(content, assignments) -> String`

Process lines one by one:
1. If a line matches the compose port pattern `- "host:container"`, replace only the host portion if it matches an assignment
2. All other lines pass through unchanged (no generic replacement)

### `is_compose_file(path) -> bool`

Returns true for files matching docker-compose naming:
- `docker-compose.yml`, `docker-compose.yaml`
- `docker-compose.*.yml`, `docker-compose.*.yaml`
- `compose.yml`, `compose.yaml`

### `copy_files_with_port_transformation` change

Add compose file detection before choosing transform function:

```
if is_env_file(path)     → transform_env_content()
elif is_compose_file(path) → transform_compose_content()
else                       → transform_generic_content()
```

## Data Structure Changes

None. Existing `PortAssignment`, `PortConfig`, and UI remain unchanged.

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/commands/port_isolation.rs` | Add `transform_compose_content()`, `is_compose_file()` |
| `src-tauri/src/commands/port_isolation.rs` | Update `copy_files_with_port_transformation()` routing |
| `src-tauri/src/commands/port_isolation.rs` | Add tests, update existing compose test |

## Edge Cases

- Quoted vs unquoted ports: `"5432:5432"`, `'5432:5432'`, `5432:5432`
- Port with protocol: `"5432:5432/tcp"`
- Port ranges: `"8000-8010:8000-8010"` (not currently detected, ignore)
- Non-port-mapping lines in compose files should NOT be transformed
