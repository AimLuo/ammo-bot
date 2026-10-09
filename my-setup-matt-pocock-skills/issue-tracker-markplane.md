# Issue tracker: Markplane

Issues and specs for this repo live in Markplane. Use the `markplane` CLI for all operations; the data is at the repo root in `.markplane/`.

## Conventions

- **Create**: `markplane add "<title>"` (task), `markplane epic "<title>"`, `markplane plan --title "<title>"` (standalone), `markplane map "<title>"`, `markplane note "<title>"`. Pass `--body-file <path|->` on create commands to write the full body in one step (placed after the title heading).
- **Read**: `markplane show <ID>` (includes comments when present).
- **List / query**: `markplane ls` with filters (`--epic`, `--plan`, `--map`, `--tags`, `--status`, `--assignee`, `--type`, `--frontier`, `--archived`). Also `markplane ls epics|plans|notes|maps`.
- **Attach parent**: on create use `--epic <EPIC>`, `--plan <PLAN>`, or `--map <MAP>`; or later `markplane link <TASK> <PARENT> -r epic|plan|map`.
- **Comment**: `markplane comment <ID> --body-file <path|->` (appends under `## Comments`).
- **Tags**: `markplane update <ID> --add-tag <tags>` / `--remove-tag <tags>`; create commands accept `--tags`.
- **Close**: `markplane done <ID>` or `markplane status <ID> done` (tasks); for specs see [How this tracker closes work](#how-this-tracker-closes-work).
- **Edit**: `markplane edit <ID>` opens `$EDITOR`; for sectional body edits, edit the markdown file directly.
- **Delete**: there is no delete command — `markplane status <ID> cancelled`, then `markplane archive <ID>`.

## How this tracker closes work

No pull requests. Specs move through status only:

| Timing | EPIC | PLAN |
| --- | --- | --- |
| Published (`/to-spec` / my-to-spec) | `markplane status <EPIC> next` | `markplane status <PLAN> approved` |
| Implementation started (`/implement-spec`) | `markplane status <EPIC> now` | `markplane status <PLAN> in-progress` |
| Implementation finished | `markplane status <EPIC> done` | `markplane status <PLAN> done` |

Implementation tickets: `markplane done <TASK>` (or `markplane status <TASK> done`). Out of scope: `markplane status <TASK> cancelled`.

## Commit references

Every implementation commit carries a trailer so `/code-review` can recover the ticket:

```
Refs: TASK-xxxxx
```

Fetch with `markplane show TASK-xxxxx` (and follow its `epic` / `plan` / parent links as needed).

## Triage

**PRs as a request surface: no.** Markplane has no PR surface.

Map the five canonical triage roles to **same-named tags** on EPICs, PLANs, and Implementation tickets (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). Apply with `--tags` on create or `markplane update <ID> --add-tag …`.

**Decision tickets never carry `ready-for-agent`.** They are claimed via wayfinding, not the unattended implementation queue.

## When a skill says "publish to the issue tracker"

### Specs (`/to-spec` / my-to-spec)

Write the spec body to a file (sections only — no leading `#` title), then:

- **EPIC**: `markplane epic "<title>" --body-file <path> --tags ready-for-agent`, then `markplane status <EPIC-id> next`
- **PLAN**: `markplane plan --title "<title>" --body-file <path> --tags ready-for-agent`, then `markplane status <PLAN-id> approved`

If the conversation came from a Map, after publish: `markplane link <MAP-id> <SPEC-id> -r spec`. Leave the Map's status unchanged.

### Tickets (`/to-tickets`)

Identify the parent EPIC or PLAN first. Create each Implementation ticket with its approved body:

- Under an EPIC: `markplane add "<title>" --epic <EPIC-id> --body-file <path> --tags ready-for-agent`
- Under a PLAN: `markplane add "<title>" --plan <PLAN-id> --body-file <path> --tags ready-for-agent`

Wire blocking edges with `markplane link <blocker> <blocked> -r blocks` (or `markplane link <blocked> <blocker> -r depends-on`).

Attaching a task with `--plan` (or `link … -r plan`) automatically maintains `Plan.implements`. That reciprocal bookkeeping is **not** a modification of the parent issue for `/to-tickets` purposes — leave the PLAN body and status alone.

## When a skill says "fetch the relevant ticket"

Run `markplane show <ID>`. The reference may be an ID (`TASK-xxxxx`, `EPIC-xxxxx`, `PLAN-xxxxx`, `MAP-xxxxx`) rather than a URL.

## Wayfinding operations

Used by `/wayfinder`. A **Map** holds **Decision tickets** (tasks with a `map` field). Wayfinding resolves decisions, not implementation specs; once the map is clear, proceed to `/to-spec`.

- **Map**: `markplane map "<effort>" --body-file <path>` (body holds Destination / Notes / Decisions so far / Not yet specified / Out of scope).
- **Child ticket**: `markplane add "<question>" --map <MAP-id> --type <research|prototype|grilling|task> --body-file <path>`. Never add `ready-for-agent`.
- **Blocking**: `markplane link <blocker> <blocked> -r blocks`.
- **Frontier**: `markplane ls --map <MAP-id> --frontier` (open, every `depends_on` closed or cancelled, unassigned; sorted by `position`). `--frontier` also combines with `--epic` / `--plan`.
- **Claim**: `markplane start <ID> --user <name>` (sets assignee; first write of the session).
- **Resolve**: `markplane resolve <ID> --answer-file <path> --gist "<one line>"` — writes `## Answer`, marks the ticket done, and appends `- <title> [[ID]]: <gist>` to the Map's Decisions so far. If the Map has no open tickets left, the command prints a hint.
- **Out of scope**: `markplane status <ID> cancelled`, then edit the Map's Out of scope section directly.
- **Map done**: after resolve hints that nothing remains open, **ask the user**, then `markplane status <MAP-id> done`.
- **Refer by name**: when citing a ticket or map in prose, write `标题 [[ID]]`.

## Research output

Research Decision tickets still land their artefacts on a `research/<name>` branch (not inside `.markplane/`).
