# Skills workflow on Markplane

This fork is the tracker backend for [mattpocock/skills](https://github.com/mattpocock/skills). The end-to-end path is:

**wayfinder → my-to-spec → to-tickets → implement-spec → code-review**

All tracker operations use the `markplane` CLI (see the [Markplane tracker template](../my-setup-matt-pocock-skills/issue-tracker-markplane.md) written into a repo as `docs/agents/issue-tracker.md` by `my-setup-matt-pocock-skills`).

Terminology (Map, Destination, Decision ticket, Frontier, Spec, Implementation ticket) lives in root [`GLOSSARY.md`](../GLOSSARY.md).

## One-time setup

1. Install this fork from source and the repo `my-*` skills (`./scripts/link-skills.sh`). See [README Installation](../README.md#installation).
2. Install the upstream mattpocock skills you need (`wayfinder`, `to-tickets`, `implement-spec`, `code-review`, …).
3. In each project repo, run `/my-setup-matt-pocock-skills`. When `.markplane/` exists it recommends Markplane and writes `docs/agents/issue-tracker.md` from the Markplane template (CLI-only conventions).

## 1. wayfinder — chart a Map

Wayfinder explores uncertainty as a **Map** of **Decision tickets**, not implementation slices.

| Step | Command |
|------|---------|
| Create map | `markplane map "<effort>" --body-file <path>` |
| Child ticket | `markplane add "<question>" --map <MAP> --type <research\|prototype\|grilling\|task> --body-file <path>` |
| Blocking edges | `markplane link <blocker> <blocked> -r blocks` |
| Frontier | `markplane ls --map <MAP> --frontier` |
| Claim | `markplane start <TASK> --user <name>` |
| Resolve | `markplane resolve <TASK> --answer-file <path> --gist "<one line>"` |
| Out of scope | `markplane status <TASK> cancelled` (then edit the Map's Out of scope section) |
| Map done | After resolve hints nothing is open, **ask the human**, then `markplane status <MAP> done` |

Never tag Decision tickets `ready-for-agent`. Research artefacts still land on a `research/<name>` branch.

## 2. my-to-spec — publish a Spec

`my-to-spec` synthesizes the conversation into a Spec **without interviewing**. It asks once whether to publish as an **EPIC** (user-facing outcome) or a **PLAN** (technical scheme), with a recommendation.

```bash
# EPIC
markplane epic "<title>" --body-file <path> --tags ready-for-agent
markplane status <EPIC> next

# PLAN (standalone)
markplane plan --title "<title>" --body-file <path> --tags ready-for-agent
markplane status <PLAN> approved
```

If the conversation came from a Map, link after publish and leave the Map status alone:

```bash
markplane link <MAP> <SPEC> -r spec
```

Body files are sections only (no leading `#` title). EPIC and PLAN share the same spec section shape.

## 3. to-tickets — Implementation tickets under a Spec

Create tracer-bullet tasks under the published Spec:

```bash
markplane add "<title>" --epic <EPIC> --body-file <path> --tags ready-for-agent
# or
markplane add "<title>" --plan <PLAN> --body-file <path> --tags ready-for-agent

markplane link <blocker> <blocked> -r blocks
```

Attaching with `--plan` maintains `Plan.implements` automatically; that bookkeeping is not a modification of the parent Spec body or status.

## 4. implement-spec — dispatch the frontier

When starting a Spec:

| Spec | Start | Finish |
|------|-------|--------|
| EPIC | `markplane status <EPIC> now` | `markplane status <EPIC> done` |
| PLAN | `markplane status <PLAN> in-progress` | `markplane status <PLAN> done` |

List work (optionally only the frontier):

```bash
markplane ls --epic <EPIC> --frontier
markplane ls --plan <PLAN> --frontier
markplane start <TASK> --user <name>
markplane done <TASK>
```

Comments and discussion:

```bash
markplane comment <ID> --body-file <path>
markplane show <ID>
```

## 5. code-review — recover ticket and Spec from commits

Every implementation commit carries a trailer:

```
Refs: TASK-xxxxx
```

Review recovers context with:

```bash
markplane show TASK-xxxxx
# follow epic / plan / parent links as needed
```

No PR is required to close work — Specs and tickets move through status only (see the tracker doc's "How this tracker closes work").

## How the pieces connect

```
wayfinder          Map + Decision tickets → decisions recorded on the Map
       ↓
my-to-spec         Map (optional) → EPIC or PLAN Spec (+ link -r spec)
       ↓
to-tickets         Spec → Implementation tickets + blocks graph
       ↓
implement-spec     Spec status → now/in-progress; frontier → done
       ↓
code-review        Refs: TASK-xxxxx → show ticket + Spec
```

For command details see [CLI Reference](cli-reference.md). For MCP parity see [MCP Setup](mcp-setup.md). For Web UI Maps pages see [Web UI Guide](web-ui-guide.md#maps-maps).
