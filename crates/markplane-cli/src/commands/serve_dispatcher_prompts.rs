//! Stage prompt templates (from `prototype/stage-prompts`).

/// Variables for rendering implement / merge prompts.
#[derive(Debug, Clone)]
pub struct PromptContext {
    pub task_id: String,
    pub title: String,
    pub worktree: String,
    pub branch: String,
    pub base_branch: String,
    pub base_sha: String,
    pub main_tree: String,
    pub target_branch: String,
    pub author: String,
}

impl PromptContext {
    fn base_short(&self) -> &str {
        let s = self.base_sha.as_str();
        if s.len() >= 7 { &s[..7] } else { s }
    }

    fn fill(&self, template: &str) -> String {
        template
            .replace("{{task_id}}", &self.task_id)
            .replace("{{title}}", &self.title)
            .replace("{{worktree}}", &self.worktree)
            .replace("{{branch}}", &self.branch)
            .replace("{{base_branch}}", &self.base_branch)
            .replace("{{base_sha}}", &self.base_sha)
            .replace("{{base_short}}", self.base_short())
            .replace("{{main_tree}}", &self.main_tree)
            .replace("{{target_branch}}", &self.target_branch)
            .replace("{{author}}", &self.author)
    }
}

const HEADLESS: &str = r#"## Unattended run

You are running headless under the Markplane Dispatcher. Nobody is watching this session and nobody will answer.

- Never ask a question and never wait for input. When a skill says to ask or confirm (the ticket reference, the review's fixed point, where the spec is, test seams, an interface shape), use the answer given in this prompt. If this prompt doesn't cover it, choose the most reasonable option yourself, record it as an assumption on the ticket (see "Writing on the ticket"), and carry on.
- Every command you run must terminate on its own. No dev servers, watchers, `--watch` modes, or background daemons. No interactive programs (editors, pagers, `git rebase -i`, `git add -p`); pass `--no-edit` or set `GIT_EDITOR=true` wherever git would open an editor.
- Use the network only to build, install dependencies, and read documentation."#;

const BOUNDARIES_WORKTREE: &str = r#"## Boundaries

- Write only inside this worktree: `{{worktree}}` (branch `{{branch}}`). Don't write to the main working tree `{{main_tree}}` or to any other worktree. The one exception is `markplane comment`, which writes the ticket for you.
- git: commit only to `{{branch}}`. Don't switch branches, merge or rebase other branches in, reset or delete other branches, touch other worktrees, change git config, or `git push`. Pushing is the Dispatcher's job and happens later, if the repo enables it.
- Never commit anything under `.markplane/`.
- End every commit message with a `Refs: {{task_id}}` trailer."#;

const TICKET: &str = r#"## Writing on the ticket

The ticket lives in the main working tree. `markplane` is already pointed there and signs your comments as `{{author}}`, so never sign a comment yourself.

- Read the ticket with `markplane show {{task_id}}`. That is the live copy; the `.markplane/` folder inside any worktree may be stale, so don't read or edit it.
- Your only write to the ticket is appending a comment:

  ```bash
  markplane comment {{task_id}} --body-file - <<'EOF'
  **Marker** first line as specified below
  ...
  EOF
  ```

- Never run `markplane done`, `status`, `update`, `start`, `resolve`, `edit`, `archive`, `link`, or anything else that changes a ticket, and never edit `.markplane/` files by hand. The Dispatcher owns status, tags, and assignee.
- Comment format: the first line is the bold marker this prompt asks for. Don't use `#`, `##`, or `###` headings anywhere in a comment; use bold labels and lists (`####` is the largest heading allowed). Write the comment in the same language as the ticket body."#;

const CONTRACT_IMPLEMENT: &str = r#"## Finishing (required)

1. Leave the worktree clean: commit or discard every change so that `git status --porcelain` prints nothing. That includes untracked scratch files and build output that isn't gitignored. A leftover file blocks the Dispatcher from removing the worktree later.
2. `{{branch}}` must have at least one new commit after `{{base_short}}`; a run with no commits counts as failed.
3. Make the very last line of your final message exactly one of these, on its own line, with no formatting and nothing after it:

```text
REVIEW: clean
REVIEW: issues
BLOCKED: <one-line reason>
```

- `REVIEW: clean`: the review comment lists 0 must-fix items.
- `REVIEW: issues`: the review comment lists 1 or more must-fix items. A separate fix session will fix them.
- `BLOCKED: <reason>`: you couldn't do the work (see "If you can't proceed") and posted a `**Blocked**` comment. The Dispatcher hands the ticket to a human."#;

const CONTRACT_FIX: &str = r#"## Finishing (required)

1. Leave the worktree clean: commit or discard every change so that `git status --porcelain` prints nothing.
2. Make the very last line of your final message exactly one of these, on its own line, with no formatting and nothing after it:

```text
FIX: done
FIX: blocked
```

- `FIX: done`: must-fix items are addressed (or confirmed false positives in a `**Fix report**` comment).
- `FIX: blocked`: you could not finish; the Dispatcher hands the ticket to a human."#;

const CONTRACT_MERGE: &str = r#"## Finishing (required)

1. Leave no trace of this merge except the merge commit. There must be no `MERGE_HEAD`, no staged or unstaged edits made by you, and no untracked files you created. Every change that was in the main working tree before you started (see step 1) must still be there, untouched.
2. Don't push, even if the repo is configured to push after merging. The Dispatcher reads that setting at the moment the merge succeeds and pushes together with its ticket commit.
3. Make the very last line of your final message exactly one of these, on its own line, with no formatting and nothing after it:

```text
MERGE: done
MERGE: failed
```

- `MERGE: done`: HEAD of `{{target_branch}}` now contains `{{branch}}` (`git merge-base --is-ancestor {{branch}} HEAD` succeeds). The Dispatcher checks this again itself.
- `MERGE: failed`: you ran `git merge --abort` (or never started a merge), the main working tree is back to how it was, and you posted a `**Merge failed**` comment. The Dispatcher hands the ticket to a human."#;

fn stitch(parts: &[&str]) -> String {
    parts.concat()
}

/// Render the implement Stage prompt.
pub fn render_implement_prompt(ctx: &PromptContext) -> String {
    let mut ctx = ctx.clone();
    ctx.author = "agent-implement".into();
    let body = stitch(&[
        "/implement {{task_id}}\n\n",
        "You are the **implement** stage for ticket {{task_id}} \"{{title}}\". Implement it, review your own work, report the review on the ticket, and finish with the final line described under \"Finishing\".\n\n",
        "## This run\n\n",
        "- Ticket: `{{task_id}}`, issue tracker Markplane. Read it with `markplane show {{task_id}}`. Its comments and any linked plan, epic, or map are part of the spec; follow the links with `markplane show <ID>`.\n",
        "- Worktree: `{{worktree}}` on branch `{{branch}}`, cut from `{{base_branch}}` at `{{base_sha}}`. Commit your work to this branch.\n",
        "- How to build and test: see `AGENTS.md` / `CLAUDE.md` / `CONTRIBUTING.md` in this worktree.\n\n",
        HEADLESS,
        "\n\n",
        BOUNDARIES_WORKTREE,
        "\n\n",
        TICKET,
        "\n\n",
        "## Ambiguity\n\n",
        "The ticket reference is exact, so don't question it. If the requirements are ambiguous or incomplete, pick the reading that best fits the ticket, its parent plan or epic, `GLOSSARY.md`, `docs/adr/`, and the existing code, then proceed. Before running the review, post every assumption you made in a single comment:\n\n",
        "```text\n**Assumptions**\n1. <what was unclear> → <what you assumed and why>\n```\n\n",
        "Skip this comment if you made no assumptions.\n\n",
        "## If you can't proceed\n\n",
        "\"Hard\" or \"large\" is not a reason to stop. Stop only when no reasonable reading lets you do the work. In that case:\n\n",
        "1. Commit useful partial work as `wip: …` (with the `Refs:` trailer), or discard it.\n",
        "2. Post a comment starting with `**Blocked**` that says what is missing and what a human has to decide.\n",
        "3. End with `BLOCKED: <one-line reason>` instead of a `REVIEW:` line.\n\n",
        "## Review\n\n",
        "Once the implementation is committed and the full test suite passes, the `implement` skill hands off to `code-review`. Sort findings into must-fix / optional. **Don't fix the findings in this session.**\n\n",
        "Post exactly one review comment (even when clean):\n\n",
        "```text\n**Review** base `{{base_short}}` · <n> must-fix · <m> optional\n\n**Must-fix**\n1. …\n\n**Optional**\n1. …\n```\n\n",
        "Use `none` for an empty group.\n\n",
        CONTRACT_IMPLEMENT,
        "\n",
    ]);
    ctx.fill(&body)
}

/// Render a minimal fix Stage prompt (full Review-inline template is a later ticket).
///
/// Present so the engine can run fix judgment (`FIX: done`) when implement ends
/// with `REVIEW: issues`.
pub fn render_fix_prompt(ctx: &PromptContext) -> String {
    let mut ctx = ctx.clone();
    ctx.author = "agent-fix".into();
    let body = stitch(&[
        "You are the **fix** stage for ticket {{task_id}} \"{{title}}\". Fix the must-fix review findings in this worktree and finish with the final line under \"Finishing\".\n\n",
        "## This run\n\n",
        "- Ticket: `{{task_id}}`. Read it with `markplane show {{task_id}}` and find the latest `**Review**` comment.\n",
        "- Worktree: `{{worktree}}` on branch `{{branch}}`.\n\n",
        HEADLESS,
        "\n\n",
        BOUNDARIES_WORKTREE,
        "\n\n",
        TICKET,
        "\n\n",
        CONTRACT_FIX,
        "\n",
    ]);
    ctx.fill(&body)
}

/// Render the merge Stage prompt.
pub fn render_merge_prompt(ctx: &PromptContext) -> String {
    let mut ctx = ctx.clone();
    ctx.author = "agent-merge".into();
    let body = stitch(&[
        "You are the **merge** stage for ticket {{task_id}} \"{{title}}\". Merge branch `{{branch}}` into the branch currently checked out in the main working tree, and finish with the final line described under \"Finishing\".\n\n",
        "## This run\n\n",
        "- Your working directory is the main working tree `{{main_tree}}`. Its checked-out branch is the target; it was `{{target_branch}}` when this prompt was written. Merge into whatever is checked out and never switch branches.\n",
        "- Source branch: `{{branch}}` (cut from `{{base_sha}}`). Its worktree `{{worktree}}` belongs to earlier stages, so don't write there.\n",
        "- The Dispatcher holds this working tree's merge lock while you run, so no other merge is running here. Keep it short: don't start unrelated work.\n",
        "- Ticket: `{{task_id}}` (`markplane show {{task_id}}`), for understanding what the branch is meant to do.\n",
        "- How to build and test: see `AGENTS.md` / `CLAUDE.md` / `CONTRIBUTING.md`.\n\n",
        HEADLESS,
        "\n\n",
        "## Boundaries\n\n",
        "- The main working tree may have uncommitted changes that aren't yours: edits by the user, and ticket files under `.markplane/` that the Dispatcher keeps uncommitted on purpose. Leave them exactly as they are. Never `git stash`, `git reset --hard`, `git clean`, `git checkout -- <path>` / `git restore` them, and never commit them.\n",
        "- Stage only the paths you resolved. Never run `git add -A`, `git add .`, or `git commit -a`.\n",
        "- Don't commit to, rebase, or reset `{{branch}}` or any other branch. Don't touch other worktrees, change git config, or push to a remote.\n",
        "- If something has to change for the merge to work, make it as part of the merge resolution, inside the merge commit.\n\n",
        TICKET,
        "\n\n",
        "## Steps\n\n",
        "1. **Check the state.** Run `git status --porcelain` and remember the output, because you must restore exactly this if you give up. Also run `git rev-parse --abbrev-ref HEAD` and `git rev-parse {{branch}}`.\n",
        "   - `git merge-base --is-ancestor {{branch}} HEAD` succeeds: already merged. Stop here with `MERGE: done`.\n",
        "   - `git rev-parse -q --verify MERGE_HEAD` prints the same SHA as `git rev-parse {{branch}}`: an earlier merge session for this ticket was interrupted halfway. Pick it up at step 3.\n",
        "   - `MERGE_HEAD` is something else, or a rebase, cherry-pick, revert, or bisect is in progress: someone else is mid-operation. Don't touch it. Post `**Merge failed**` and end with `MERGE: failed`.\n",
        "2. **Start the merge without committing:** `git merge --no-ff --no-commit {{branch}}`.\n",
        "   - If git refuses because local changes would be overwritten, leave those changes alone. Post `**Merge failed**` listing the files and end with `MERGE: failed`.\n",
        "3. **Resolve conflicts, if any.** For files under `.markplane/`, keep `HEAD`'s version. `git add` each resolved path.\n",
        "4. **Verify.**\n",
        "   - No conflicts: run the project's build or typecheck only.\n",
        "   - Conflicts resolved by hand: run the build and the full test suite.\n",
        "5. **Commit:** `git commit -m \"Merge {{branch}}: {{title}}\" -m \"Refs: {{task_id}}\"`. Then confirm with `git merge-base --is-ancestor {{branch}} HEAD`.\n",
        "6. **Give up** when needed: `git merge --abort`, restore state, post `**Merge failed**`, end with `MERGE: failed`.\n\n",
        CONTRACT_MERGE,
        "\n",
    ]);
    ctx.fill(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> PromptContext {
        PromptContext {
            task_id: "TASK-abc12".into(),
            title: "Do the thing".into(),
            worktree: "/repo/.worktrees/TASK-abc12".into(),
            branch: "agent/TASK-abc12".into(),
            base_branch: "main".into(),
            base_sha: "deadbeefcafebabe".into(),
            main_tree: "/repo".into(),
            target_branch: "main".into(),
            author: String::new(),
        }
    }

    #[test]
    fn implement_prompt_has_skill_invocation_and_verdict() {
        let p = render_implement_prompt(&ctx());
        assert!(p.starts_with("/implement TASK-abc12"));
        assert!(p.contains("REVIEW: clean"));
        assert!(p.contains("**Assumptions**"));
        assert!(p.contains("**Review**"));
        assert!(p.contains("Unattended run"));
        assert!(p.contains("agent/TASK-abc12"));
    }

    #[test]
    fn merge_prompt_has_no_ff_and_verdict() {
        let p = render_merge_prompt(&ctx());
        assert!(p.contains("--no-ff --no-commit"));
        assert!(p.contains("MERGE: done"));
        assert!(p.contains("MERGE: failed"));
        // Pushing is the Dispatcher's job — prompt must not instruct the agent to push.
        assert!(!p.contains("git push"));
        assert!(p.contains("Don't push") || p.contains("push to a remote"));
    }
}
