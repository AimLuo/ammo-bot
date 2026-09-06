import { COMPLETION_SIGNAL, type BeadComment } from "./types.ts";

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

Fix bead ${input.beadId}: ${input.title}

Pull in the bead using \`bd show ${input.beadId}\`. If it has a parent, pull that in too.

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

Review the code changes on branch \`${input.branch}\` and improve code clarity, consistency, and maintainability while preserving exact functionality.

# CONTEXT

Run:

\`git diff ${input.targetBranch}...${input.branch}\`
\`git log ${input.targetBranch}..${input.branch} --oneline\`
${commentsSection(input.comments)}
# REVIEW PROCESS

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
}): string {
  return `# TASK

Merge \`${input.branch}\` into the current branch (\`${input.targetBranch}\`).

1. Run \`git merge ${input.branch} --no-edit\`
2. If there are merge conflicts, resolve them by reading both sides and choosing the correct resolution
3. After merging, run typecheck and tests. If they fail, fix them in this same iteration.
4. Close the bead with \`bd close ${input.beadId}\`

Do not run \`bd update --claim\`.
${commentsSection(input.comments)}${needsYouSection(input.beadId)}
Once you have merged and closed what you can, output ${COMPLETION_SIGNAL}.
`;
}
