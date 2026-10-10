# Dispatcher

The Dispatcher picks up Implementation tickets tagged `ready-for-agent`, runs them through implement → (optional) fix → merge Stages in git worktrees, and turns failures over to humans via `ready-for-human`.

Scan switches live in Settings' sibling board UI (memory only; every serve start is off). Shared Stage policy lives in the repo; machine capacity lives in the user config directory.

## Settings

| Setting | Location | Default |
| --- | --- | --- |
| Push after merge | `.markplane/config.yaml` → `dispatcher.push_after_merge` | `true` |
| Stage model / timeout | `dispatcher.stages.<implement\|fix\|merge>` | `cursor-grok-4.5-high` / `60` minutes |
| Global concurrency | `serve.json` → `dispatcher.concurrency` (next to `serve.pid`) | `20` |
| Scan switch | serve memory only | off at startup |

Unset fields are omitted on write-back so future default changes apply. Edit via **Settings → Dispatcher**, or hand-edit the files; changes hot-reload without restarting serve.

## Security

Agents run **as the same OS user as `markplane serve`**, with **no sandbox**:

- Full network access and arbitrary command execution
- Read/write any file that user can access
- MCP tools auto-approved
- Full environment variables of the serve process (including secrets)

Ticket titles, bodies, and Comments are part of the agent prompt. **Only turn scanning on for repositories you trust.**

Recommended hardening:

- Run serve under a dedicated OS user, VM, or container
- Use Cursor `permissions.deny` for sensitive paths/commands where available
- Do not put API keys or tokens in the serve environment if an agent should not see them
- Set a spend limit on the Cursor account
- Agents are instructed not to push and not to change ticket status/assignee/tags — that is convention, not enforcement

## One repository, one scanner

A given Markplane Project (repo + `.markplane/` path) should have scanning enabled in **only one** serve process at a time. Two scanners on the same Project race on pickups, worktrees, and ticket writes. The Dispatcher lock enforces a single writer; a second serve that cannot take the lock refuses to open Scan / Resume / continue / Clean up.

Do not turn Scan on in every clone. Prefer one machine (or one dedicated Dispatcher host) per repository.
