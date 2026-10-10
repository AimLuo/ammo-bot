# Markplane

A markdown-first project management system kept inside the repo, serving as the issue tracker for Matt Pocock's agent skills.

## Language

**Project**:
A directory containing a `.markplane/` folder; the unit of work the web UI opens, one at a time per view.
_Avoid_: Workspace, repo

**Project registry**:
The user-level list of Projects the web UI can open and switch between; adding or removing an entry never touches the Project's files.
_Avoid_: Project list, recent projects

**Map**:
An item that charts the way from the current state to a Destination as a set of Decision tickets; the wayfinder skill's canonical artifact.
_Avoid_: Roadmap, wayfinding issue

**Destination**:
What reaching the end of a Map looks like: the spec, decision, or change the effort is finding its way to.
_Avoid_: Goal, objective

**Decision ticket**:
A Task that belongs to a Map, whose resolution is a decision (research, prototype, grilling, or task) rather than a slice of a build.
_Avoid_: Wayfinder ticket, question

**Frontier**:
The Decision tickets of a Map that are open, unblocked, and unclaimed, in Map order.
_Avoid_: Ready queue, next up

**Spec**:
The published description of what to build, produced as either an Epic or a Plan; a Map may point at the Specs it produced.
_Avoid_: PRD, requirements doc

**Implementation ticket**:
A Task attached to a Spec and tagged `ready-for-agent`: a tracer-bullet slice of the build an agent can pick up.
_Avoid_: Slice, story

**Dispatcher**:
The part of the web server that picks up unblocked Implementation tickets and carries each one through Runs in parallel, under one global concurrency limit.
_Avoid_: Scheduler, runner, worker pool

**Run**:
One Implementation ticket's journey through the Dispatcher, from its isolated worktree to a merged result or a hand-off to a human; a Resume or Continue carries the same Run on rather than starting a new one.
_Avoid_: Job, execution

**Stage**:
One step of a Run (implement, fix, or merge), carried out as its own agent session with its own model.
_Avoid_: Phase, step

**Slot**:
One unit of the Dispatcher's global concurrency limit; a Run holds one while the Dispatcher is carrying it, including while it waits in a Merge queue; an Interrupted Run or a Run handed to a human holds none.
_Avoid_: Worker, capacity, quota

**Merge queue**:
The line of Runs waiting to merge into the same working tree; Runs from every Project sharing that working tree wait in one line, and only the Run at its head merges.
_Avoid_: Merge slot, merge lane

**Interrupted Run**:
A Run whose web server stopped before the Run ended; it holds no Slot and stays where it stopped until a human resumes or continues it.
_Avoid_: Crashed run, paused run, orphaned run

**Resume**:
A human's request that an Interrupted Run carry on from the Stage where it stopped.
_Avoid_: Restart, retry

**Continue**:
A human's request, made with a Comment, that a Run handed off to a human or interrupted carry on from the Stage where it stopped, taking the Comment as its next instruction.
_Avoid_: Retry, rerun, comment and continue

**Dispatcher lock**:
A repository-wide claim that makes one web server the only Dispatcher for a Project, shared across every working copy of that repository on the same filesystem; other web servers can view and edit the Project but cannot pick up tickets or start Stages for it.
_Avoid_: Dispatcher mutex, run lock, scan lock

**Run record**:
The local, unshared account of one Implementation ticket's Runs: which Stage each Run reached, the agent sessions it used, and how each ended. It never decides the ticket's status; the ticket does.
_Avoid_: Run log, run history, job record

**Scan switch**:
A Project's on/off control for whether the Dispatcher picks up its Implementation tickets; it lasts only as long as the web server runs and starts off every time; the Dispatcher may also turn it off on its own when agents repeatedly fail to start.
_Avoid_: Auto mode, autopilot

**Comment**:
A dated, attributed entry appended to an item by a person, the Dispatcher, or an agent; once written it is never edited in place.
_Avoid_: Note, remark

**Needs You**:
The view of every Task tagged `ready-for-human` that is not yet completed or cancelled, whatever its other status.
_Avoid_: Need Your, inbox
