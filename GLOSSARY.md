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
