---
name: my-to-spec
description: "Turn the current conversation into a spec and publish it to the project issue tracker: no interview, just synthesis of what you've already discussed. Based on mattpocock/skills v1.3.1"
disable-model-invocation: true
---

This skill takes the current conversation context and codebase understanding and produces a spec. Do NOT interview the user; just synthesize what you already know.

The issue tracker and triage label vocabulary should have been provided to you. If not, tell the user to run `/setup-matt-pocock-skills`.

## Process

1. Explore the repo to understand the current state of the codebase, if you haven't already. Use the project's domain glossary vocabulary throughout the spec, and respect any ADRs in the area you're touching.

2. Sketch out the seams at which you're going to test the feature. Existing seams should be preferred to new ones. Use the highest seam possible. If new seams are needed, propose them at the highest point you can. The fewer seams across the codebase, the better - the ideal number is one.

Check with the user that these seams match their expectations.

3. Ask once whether to publish as an **EPIC** or a **PLAN**, and include a recommendation grounded in the conversation:

- **EPIC** — user-facing feature or outcome (roadmap-shaped, broader scope).
- **PLAN** — technical scheme (refactor, migration, internal mechanism); may sit under an existing EPIC.

Wait for the user's choice before publishing.

4. Write the spec using the template below into a body file (sections only — no leading `#` title; the CLI adds it), then publish with `--body-file` and `--tags ready-for-agent`:

- **EPIC**: `markplane epic "<title>" --body-file <path> --tags ready-for-agent`, then `markplane status <EPIC-id> next`
- **PLAN**: `markplane plan --title "<title>" --body-file <path> --tags ready-for-agent`, then `markplane status <PLAN-id> approved`

5. If this conversation came from a Map, after publish run `markplane link <MAP-id> <SPEC-id> -r spec`. Leave the Map's status unchanged.

<spec-template>

## Problem Statement

The problem that the user is facing, from the user's perspective.

## Solution

The solution to the problem, from the user's perspective.

## User Stories

A LONG, numbered list of user stories. Each user story should be in the format of:

1. As an <actor>, I want a <feature>, so that <benefit>

<user-story-example>
1. As a mobile bank customer, I want to see balance on my accounts, so that I can make better informed decisions about my spending
</user-story-example>

This list of user stories should be extremely extensive and cover all aspects of the feature.

## Implementation Decisions

A list of implementation decisions that were made. This can include:

- The modules that will be built/modified
- The interfaces of those modules that will be modified
- Technical clarifications from the developer
- Architectural decisions
- Schema changes
- API contracts
- Specific interactions

Do NOT include specific file paths or code snippets. They may end up being outdated very quickly.

Exception: if a prototype produced a snippet that encodes a decision more precisely than prose can (state machine, reducer, schema, type shape), inline it within the relevant decision and note briefly that it came from a prototype. Trim to the decision-rich parts, not a working demo, just the important bits.

## Testing Decisions

A list of testing decisions that were made. Include:

- A description of what makes a good test (only test external behavior, not implementation details)
- Which modules will be tested
- Prior art for the tests (i.e. similar types of tests in the codebase)

## Out of Scope

A description of the things that are out of scope for this spec.

## Further Notes

Any further notes about the feature.

</spec-template>
