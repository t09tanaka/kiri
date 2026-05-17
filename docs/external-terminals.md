# Driving kiri from an external terminal

The `kiri` CLI is auto-installed at `~/.kiri/bin/kiri` and is on PATH
inside every kiri-spawned shell. Outside a kiri terminal it can still
work — `kiri term split` and friends will reach the right kiri window
as long as it can find a live UDS to talk to.

This document covers the handshake for the three external terminals
people most often pair with kiri: WezTerm, Zed, and Cursor.

## How discovery works

```
1. Start with $KIRI_SOCKET.
2. If it points at a live socket, use it.
3. Otherwise scan ~/.kiri/instances/*.sock for live sockets.
4. If the current working directory is inside a project, prefer the
   socket whose window has that project open.
5. If exactly one socket survives, use it. Otherwise error out.
```

You don't need to memorise this. `kiri env` (added in v0.6) prints
exactly what the CLI sees, including which socket it would target and
why. Run it first when something is off:

```
$ kiri env --pretty
KIRI_TERMINAL  (unset)
KIRI_SOCKET    (unset)
  in kiri terminal:   no
  socket alive:       no
  cwd project:        /Users/me/code/kiri
  instances dir:      /Users/me/.kiri/instances
  resolved socket:    /Users/me/.kiri/instances/abc123.sock
  resolution:         resolved
  discovered windows:
    /Users/me/.kiri/instances/abc123.sock — /Users/me/code/kiri
```

`kiri env` makes no IPC calls, so it works from anywhere. It is the
debugging starting point for every "why can't kiri find my window?"
question.

## WezTerm

WezTerm inherits the shell's PATH, so `~/.kiri/bin/kiri` is reachable
as soon as you have launched kiri at least once. No env wiring is
needed — discovery handles it.

If you want to skip discovery (faster startup, more deterministic),
point `KIRI_SOCKET` at the open kiri window for the current project:

```lua
-- wezterm.lua
local wezterm = require 'wezterm'

return {
  set_environment_variables = {
    -- Optional: set KIRI_SOCKET so the CLI skips discovery. Replace
    -- <window-id> with the value from `kiri env --pretty`.
    -- KIRI_SOCKET = '/Users/me/.kiri/instances/<window-id>.sock',
  },
}
```

Most users do not need to set this. Discovery uses the project root of
the cwd, which gives the right answer when you have one kiri window
per project.

## Zed

Zed's integrated terminal inherits the shell environment from the
parent process. `~/.kiri/bin/kiri` should be on PATH the moment you
open a Zed terminal inside a project that has a kiri window open.

If `kiri term ls` returns an error from Zed:

1. Run `kiri env --pretty` from the same terminal.
2. Check `resolved socket` — empty means discovery failed.
3. Open the project in kiri if you have not already. Discovery only
   matches windows that have the same project open.

You do not need to configure Zed for this; the behaviour is the same
as any other terminal.

## Cursor

Cursor's integrated terminal also inherits the shell environment, with
one caveat: if you launched Cursor from a desktop shortcut rather than
from a shell, the PATH the terminal inherits may not include
`~/.kiri/bin/`. Fix by either:

- Launching Cursor from a shell session that has the right PATH, **or**
- Adding `~/.kiri/bin/` to your shell rc (`.zshrc`, `.bashrc`, etc.)
  so login shells already include it.

After that, `kiri env --pretty` should report the right socket and
`kiri term ls` should return a window.

## Scripting the handshake

Other terminals (Alacritty, kitty, iTerm2 profiles, …) follow the same
pattern: the CLI finds kiri by itself; you only need to wire env vars
if you want to skip discovery.

A robust handshake script:

```bash
# Bail out if kiri is not installed.
command -v kiri >/dev/null || exit 0

# Refresh KIRI_SOCKET from discovery. `kiri env --pretty` writes a
# stable "resolved socket: …" line on a successful resolution.
target=$(kiri env --pretty 2>/dev/null \
  | awk '/^  resolved socket: / { print $3 }')
if [ -n "$target" ] && [ "$target" != "(none)" ]; then
  export KIRI_SOCKET="$target"
fi
```

Drop that into the relevant rc file and external terminals will pin
`KIRI_SOCKET` to the right window without you thinking about it.
