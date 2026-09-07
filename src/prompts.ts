import { COMPLETION_SIGNAL, type BeadComment } from "./types.ts";
import type { MergeSync } from "./project-layout.ts";

function commentsSection(comments: BeadComment[] | undefined): string {
  if (!comments?.length) return "";
  const body = comments
    .map((c) => `- ${c.author || "unknown"}: ${c.text}`)
    .join("\n");
  return `
# COMMENTS

These comments are on the bead. After a human response, follow their instructions.

${body}
`;
}

function needsYouSection(beadId: string): string {
  return `
# NEEDS YOU

If you cannot proceed without a human decision:

1. Comment the question: \`bd comment ${beadId} "..."\`
2. Add the human flag: \`bd update ${beadId} --add-label human\`
3. Stop immediately. Do not output ${COMPLETION_SIGNAL}. Do not keep working.

The pipeline stays running and resumes after the human flag is removed.
`;
}

export function implementPrompt(input: {
  beadId: string;
  title: string;
  branch: string;
  comments?: BeadComment[];
}): string {
  return `# TASK

Implement bead ${input.beadId}: ${input.title}

This prompt is the implementation. Pull in the bead with \`bd show ${input.beadId}\`. If it has a parent, pull that in too.

Only work on the bead specified.

Work on branch ${input.branch}. Make at most one commit this iteration and run tests.
${commentsSection(input.comments)}
# EXECUTION

If applicable, use RGR to complete the task.

1. RED: write one test
2. GREEN: write the implementation to pass that test
3. REPEAT until done

# FEEDBACK LOOPS

Before committing, run typecheck and tests.

# COMMIT

Make at most one git commit this iteration. Keep the message concise.

# THE BEAD

If the task is not complete, leave a comment on the bead with what was done.

Do not close the bead — this will be done later. Do not run \`bd close\` or \`bd update --claim\`.
${needsYouSection(input.beadId)}
Once complete, output ${COMPLETION_SIGNAL}.

# FINAL RULES

ONLY WORK ON A SINGLE BEAD.
`;
}

export function reviewPrompt(input: {
  beadId: string;
  branch: string;
  targetBranch: string;
  comments?: BeadComment[];
}): string {
  return `# TASK

Improve the existing commits on branch \`${input.branch}\` for clarity, consistency, and maintainability, preserving exact functionality.

This prompt is the improve pass. Read bead \`${input.beadId}\` with \`bd show ${input.beadId}\` if the diff needs spec context.

# CONTEXT

Run:

\`git diff ${input.targetBranch}...${input.branch}\`
\`git log ${input.targetBranch}..${input.branch} --oneline\`
${commentsSection(input.comments)}
# PROCESS

1. Understand the change from the diff and commits.
2. Improve clarity and consistency without changing behaviour.
3. If the code is already clean, do nothing.

# EXECUTION

If you find improvements, make them on this branch, run typecheck and tests, and commit.

Do not close the bead. Do not run \`bd close\` or \`bd update --claim\`.
${needsYouSection(input.beadId)}
Once complete, output ${COMPLETION_SIGNAL}.
`;
}

export function mergePrompt(input: {
  beadId: string;
  branch: string;
  targetBranch: string;
  comments?: BeadComment[];
  sync: MergeSync;
}): string {
  const layoutLabel =
    input.sync.kind === "multi" ? "multi-context" : "single-context";
  const missing = input.sync.subtrees.filter((s) => !s.url);
  const ready = input.sync.subtrees.filter((s) => s.url);
  const remoteLines: string[] = [];
  let n = 5;
  for (const subtree of ready) {
    remoteLines.push(
      `${n}. Run \`git subtree push --prefix=repos/${subtree.name} ${subtree.url} ${input.targetBranch}\`. If it is not a fast-forward, add the human flag and stop.`,
    );
    n += 1;
  }
  if (missing.length) {
    remoteLines.push(
      `${n}. A changed prefix has no git remote URL: ${missing
        .map((s) => `repos/${s.name}`)
        .join(
          ", ",
        )}. Comment which prefix is missing, add the human flag (\`bd update ${input.beadId} --add-label human\`), and stop. Do not output ${COMPLETION_SIGNAL}.`,
    );
    n += 1;
  }
  if (input.sync.originUrl) {
    remoteLines.push(
      `${n}. Run \`git push\` (or \`git push -u origin HEAD\` if there is no upstream). If it is not a fast-forward, add the human flag and stop.`,
    );
  } else {
    remoteLines.push(
      `${n}. There is no origin remote. Skip pushing the parent.`,
    );
  }

  return `# TASK

Merge \`${input.branch}\` into the current branch (\`${input.targetBranch}\`).

This project is ${layoutLabel} according to its AGENTS.md. If that disagrees with AGENTS.md in your context, add the human flag and stop.

1. If \`git merge-base --is-ancestor ${input.branch} HEAD\` succeeds, the branch is already merged. Skip merge and typecheck; only finish remote sync.
2. Otherwise run \`git merge ${input.branch} --no-edit\`
3. If there are merge conflicts, resolve them by reading both sides and choosing the correct resolution
4. After merging, run typecheck and tests. If they fail, fix them in this same iteration.
${remoteLines.join("\n")}

Do not close the bead. Do not run \`bd close\` or \`bd update --claim\`. Never use --force or --force-with-lease.
${commentsSection(input.comments)}${needsYouSection(input.beadId)}
Once you have merged and synced remotes, output ${COMPLETION_SIGNAL}.
`;
}

