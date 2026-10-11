//! Stage prompt templates (from `prototype/stage-prompts`).

use markplane_core::Stage;

/// Variables for rendering implement / fix / merge prompts.
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
    /// Latest `**Review**` body by `agent-implement` (fix Stage); empty if none.
    pub review_comment: String,
    /// When set, prepended as a prior-attempt note (fresh session after interrupt).
    pub prior_attempt: String,
    /// Human comment body for continue-with-comment / prior-continue (empty otherwise).
    pub human_comment: String,
    /// Last failure reason for continue prompts.
    pub failure_reason: String,
    /// Human comment author (serve default author).
    pub human: String,
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
            .replace("{{review_comment}}", &self.review_comment)
            .replace("{{prior_attempt}}", &self.prior_attempt)
            .replace("{{human_comment}}", &self.human_comment)
            .replace("{{failure_reason}}", &self.failure_reason)
            .replace("{{human}}", &self.human)
            .replace("{{inspect_hint}}", &self.inspect_hint())
    }

    fn inspect_hint(&self) -> String {
        format!(
            "run `git status` and `git log --oneline {}..HEAD` in `{}`",
            self.base_short(),
            self.worktree
        )
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

1. Leave the worktree clean: commit or discard every change so that `git status --porcelain` prints nothing (including untracked scratch files and non-ignored build output).
2. Post the `**Fix report**` comment.
3. Make the very last line of your final message exactly one of these, on its own line, with no formatting and nothing after it:

```text
FIX: done
FIX: blocked
```

- `FIX: done`: every must-fix item is either fixed or declined as a false positive with a reason, the full test suite passes, and everything is committed.
- `FIX: blocked`: at least one must-fix item couldn't be fixed, or the tests don't pass. Commit whatever you did fix and explain the rest in the report. The Dispatcher hands the ticket to a human."#;

const PRIOR_HALFDONE: &str = r#"## Previous attempt (interrupted)

An earlier session for this stage was interrupted (the Markplane server stopped) and couldn't be resumed, so this is a fresh session with no memory of it. Its work may still be here: {{inspect_hint}}. Look before you change anything. Keep what is correct and already committed, finish or discard half-done edits, and don't redo finished steps. It may also have posted comments as `{{author}}`; check `markplane show {{task_id}}` and don't post duplicates.

"#;

const PRIOR_CONTINUE: &str = r#"## Previous attempt (failed, human replied)

An earlier session for this stage failed: {{failure_reason}}. Its session couldn't be resumed, so this is a fresh session with no memory of it. A human then commented on the ticket (as `{{human}}`):

<human-comment>
{{human_comment}}
</human-comment>

Act on that comment. Where it conflicts with the instructions below it wins, except for "Boundaries", "Writing on the ticket", and "Finishing", which always apply. If it asks a question, answer it in a ticket comment and keep going. If it tells you to stop, end with the failing final line.

The earlier session's work may still be here: {{inspect_hint}}. Look before you change anything. Keep what is correct, and don't post duplicate comments.

"#;

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
    let prior = if ctx.prior_attempt.is_empty() {
        String::new()
    } else {
        ctx.fill(PRIOR_HALFDONE)
    };
    let body = stitch(&[
        "/implement {{task_id}}\n\n",
        prior.as_str(),
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

/// Render the fix Stage prompt (prototype `templates/fix.md`).
///
/// Inlines the latest `agent-implement` `**Review**` comment when the engine
/// provides it; when the `<review>` block is empty the agent must `markplane show`.
pub fn render_fix_prompt(ctx: &PromptContext) -> String {
    let mut ctx = ctx.clone();
    ctx.author = "agent-fix".into();
    let prior = if ctx.prior_attempt.is_empty() {
        String::new()
    } else {
        ctx.fill(PRIOR_HALFDONE)
    };
    let body = stitch(&[
        prior.as_str(),
        "You are the **fix** stage for ticket {{task_id}} \"{{title}}\". An earlier session implemented the ticket on branch `{{branch}}`, and its code review found must-fix items. Fix them, report on the ticket, and finish with the final line described under \"Finishing\". This is a new session, so you don't have the earlier conversation.\n\n",
        "## This run\n\n",
        "- Ticket: `{{task_id}}`, issue tracker Markplane. Read it with `markplane show {{task_id}}` (it is the spec), including linked plans or epics.\n",
        "- Worktree: `{{worktree}}` on branch `{{branch}}`, cut from `{{base_branch}}` at `{{base_sha}}`. To see what was done so far, run `git log --oneline {{base_short}}..HEAD` and `git diff {{base_sha}}...HEAD`.\n",
        "- How to build and test: see `AGENTS.md` / `CLAUDE.md` / `CONTRIBUTING.md` in this worktree.\n\n",
        "## What to fix\n\n",
        "The Dispatcher copied the latest `**Review**` comment by `agent-implement` from the ticket:\n\n",
        "<review>\n",
        "{{review_comment}}\n",
        "</review>\n\n",
        "If the block above is empty, find the last comment by `agent-implement` whose first line starts with `**Review**` in `markplane show {{task_id}}`. If there is none, post a `**Fix report**` saying so and end with `FIX: blocked`.\n\n",
        "- Fix every item under **Must-fix**. Fix an **Optional** item only if it is trivial and clearly inside the ticket's scope; otherwise leave it alone.\n",
        "- Stay inside the ticket's scope. No refactors beyond what an item needs.\n",
        "- You may decline a must-fix item only if you are confident it's a false positive, meaning the code is already right according to the ticket or the repo's documented standards. Explain why in the report. Difficulty is not a reason to decline; mark the item blocked instead.\n",
        "- Run the typecheck and the affected tests as you go, and the full suite once at the end.\n",
        "- Commit with the `Refs: {{task_id}}` trailer.\n\n",
        HEADLESS,
        "\n\n",
        BOUNDARIES_WORKTREE,
        "\n\n",
        TICKET,
        "\n\n",
        "## Report\n\n",
        "Post exactly one comment:\n\n",
        "```text\n**Fix report** · <a> fixed · <b> declined · <c> blocked\n1. <must-fix item 1, short> — fixed in `<short sha>` | declined: <why> | blocked: <why>\n…\nOptional items done: <list or none>\nTests: <command> → <pass/fail>\n```\n\n",
        CONTRACT_FIX,
        "\n",
    ]);
    ctx.fill(&body)
}

/// Short prompt when `--resume` continues an interrupted session.
pub fn render_resume_interrupted_prompt(ctx: &PromptContext, stage: Stage) -> String {
    let mut ctx = ctx.clone();
    ctx.author = match stage {
        Stage::Implement => "agent-implement".into(),
        Stage::Fix => "agent-fix".into(),
        Stage::Merge => "agent-merge".into(),
    };
    let contract = match stage {
        Stage::Implement => CONTRACT_IMPLEMENT,
        Stage::Fix => CONTRACT_FIX,
        Stage::Merge => CONTRACT_MERGE,
    };
    let stage_name = stage.as_str();
    let template = format!(
        "Your previous turn in this session was interrupted before it finished. The Markplane server stopped or restarted; this isn't counted as your failure. Continue the same **{stage_name}** task for {{{{task_id}}}} from where you left off.\n\n\
- The state is whatever you left behind: {{{{inspect_hint}}}}. Check it before doing anything. Don't redo committed work; finish or discard half-done edits.\n\
- Re-read the ticket with `markplane show {{{{task_id}}}}`. A human may have commented since your last turn.\n\
- Comments you already posted are still on the ticket, so don't post them again. If one is now out of date (for example, a review you've since redone), post a new comment with the same marker; the latest one counts.\n\
- Every rule from your original instructions still applies: unattended run, boundaries, writing on the ticket.\n\n\
{contract}\n"
    );
    ctx.fill(&template)
}

/// Short prompt when `--resume` continues after a human "Comment & continue".
///
/// From `prototype/stage-prompts/templates/continue-with-comment.md`.
pub fn render_continue_with_comment_prompt(ctx: &PromptContext, stage: Stage) -> String {
    let mut ctx = ctx.clone();
    ctx.author = match stage {
        Stage::Implement => "agent-implement".into(),
        Stage::Fix => "agent-fix".into(),
        Stage::Merge => "agent-merge".into(),
    };
    let contract = match stage {
        Stage::Implement => CONTRACT_IMPLEMENT,
        Stage::Fix => CONTRACT_FIX,
        Stage::Merge => CONTRACT_MERGE,
    };
    let stage_name = stage.as_str();
    let template = format!(
        "Your **{stage_name}** task for {{{{task_id}}}} ended without success: {{{{failure_reason}}}}. A human looked at it and commented on the ticket (as `{{{{human}}}}`):\n\n\
<human-comment>\n\
{{{{human_comment}}}}\n\
</human-comment>\n\n\
Continue the same task, acting on that comment.\n\n\
- Where the comment conflicts with your original instructions, the comment wins. The exceptions are the boundaries, the rules for writing on the ticket, and the finishing contract below; those always apply.\n\
- If the comment asks a question, answer it in a ticket comment and carry on. If it tells you to stop, end with the failing final line.\n\
- Check the current state before changing anything: {{{{inspect_hint}}}}. Re-read the ticket with `markplane show {{{{task_id}}}}` if you need its other comments.\n\
- Don't repeat comments you already posted. If one is out of date, post a new one with the same marker; the latest one counts.\n\n\
{contract}\n"
    );
    ctx.fill(&template)
}

/// Preamble when continue starts a fresh session (session could not be resumed).
pub fn render_prior_continue(ctx: &PromptContext) -> String {
    ctx.fill(PRIOR_CONTINUE)
}

/// Render the merge Stage prompt.
pub fn render_merge_prompt(ctx: &PromptContext) -> String {
    let mut ctx = ctx.clone();
    ctx.author = "agent-merge".into();
    let prior = if ctx.prior_attempt.is_empty() {
        String::new()
    } else {
        ctx.fill(PRIOR_HALFDONE)
    };
    let body = stitch(&[
        prior.as_str(),
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
        "   - `git rev-parse -q --verify MERGE_HEAD` prints the same SHA as `git rev-parse {{branch}}`: an earlier merge session for this ticket was interrupted halfway. Pick it up at step 3 (`git diff --name-only --diff-filter=U` lists what is still conflicted).\n",
        "   - `MERGE_HEAD` is something else, or a rebase, cherry-pick, revert, or bisect is in progress: someone else is mid-operation. Don't touch it. Post `**Merge failed**` and end with `MERGE: failed`.\n",
        "2. **Start the merge without committing:** `git merge --no-ff --no-commit {{branch}}`.\n",
        "   - If git refuses because local changes would be overwritten, leave those changes alone. Post `**Merge failed**` listing the files and end with `MERGE: failed`.\n",
        "3. **Resolve conflicts, if any.** Keep both sides' intent. Don't use `-X ours`/`-X theirs` or take one side of a whole file unless that is really right for that file. For files under `.markplane/`, keep `HEAD`'s version, because the main tree is the source of truth for tickets. `git add` each resolved path.\n",
        "4. **Verify.**\n",
        "   - No conflicts: run the project's build or typecheck only. The earlier stages already ran the full suite on this code.\n",
        "   - Conflicts resolved by hand: run the build and the full test suite.\n",
        "   - If verification fails because of the merge and the fix belongs to the conflict resolution, fix it and verify again. Otherwise give up (step 6).\n",
        "   - Afterwards, delete only untracked files that your commands created; compare against the step 1 output.\n",
        "5. **Commit:** `git commit -m \"Merge {{branch}}: {{title}}\" -m \"Refs: {{task_id}}\"`. Then confirm with `git merge-base --is-ancestor {{branch}} HEAD`. If you resolved conflicts by hand, post one comment:\n\n",
        "   ```text\n   **Merge notes** `<merge sha>`\n   - `<path>`: how you resolved it and why\n   Tests: <command> → pass\n   ```\n\n",
        "6. **Give up** when a conflict needs a product decision that neither the ticket nor the code settles, when the verification failure isn't a merge problem, or when git refuses to merge. Run `git merge --abort`. Confirm that `git rev-parse -q --verify MERGE_HEAD` prints nothing and that `git status --porcelain` matches step 1. Then post:\n\n",
        "   ```text\n   **Merge failed**\n   - Conflicted / blocking files: …\n   - What I tried: …\n   - What a human needs to decide: …\n   ```\n\n",
        "   End with `MERGE: failed`.\n\n",
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
            review_comment: String::new(),
            prior_attempt: String::new(),
            human_comment: String::new(),
            failure_reason: String::new(),
            human: String::new(),
        }
    }

    #[test]
    fn continue_with_comment_prompt_inlines_human_and_contract() {
        let mut c = ctx();
        c.human_comment = "fix the typo in foo.rs".into();
        c.failure_reason = "implement agent failed (exit 1, verdict=<none>)".into();
        c.human = "chuck".into();
        let p = render_continue_with_comment_prompt(&c, Stage::Implement);
        assert!(p.contains("<human-comment>"));
        assert!(p.contains("fix the typo in foo.rs"));
        assert!(p.contains("chuck"));
        assert!(p.contains("REVIEW: clean"));
        assert!(p.contains("implement agent failed"));
    }

    #[test]
    fn prior_continue_has_human_comment_block() {
        let mut c = ctx();
        c.human_comment = "try again".into();
        c.failure_reason = "no new commits".into();
        c.human = "alice".into();
        let p = render_prior_continue(&c);
        assert!(p.contains("failed, human replied"));
        assert!(p.contains("try again"));
        assert!(p.contains("alice"));
    }

    #[test]
    fn resume_interrupted_prompt_has_stage_and_contract() {
        let p = render_resume_interrupted_prompt(&ctx(), Stage::Fix);
        assert!(p.contains("**fix**"));
        assert!(p.contains("FIX: done"));
        assert!(p.contains("TASK-abc12"));
        assert!(p.contains("interrupted"));
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
    fn fix_prompt_inlines_review_and_allows_false_positive() {
        let mut c = ctx();
        c.review_comment = "**Review** base `deadbee` · 1 must-fix · 0 optional\n\n**Must-fix**\n1. rename foo\n".into();
        let p = render_fix_prompt(&c);
        assert!(p.contains("**fix** stage"));
        assert!(p.contains("<review>"));
        assert!(p.contains("rename foo"));
        assert!(p.contains("</review>"));
        assert!(p.contains("FIX: done"));
        assert!(p.contains("FIX: blocked"));
        assert!(p.contains("**Fix report**"));
        assert!(p.contains("false positive") || p.contains("declined"));
        assert!(p.contains("markplane show TASK-abc12"));
        assert!(p.contains("signs your comments as `agent-fix`"));
    }

    #[test]
    fn fix_prompt_empty_review_tells_agent_to_show() {
        let p = render_fix_prompt(&ctx());
        assert!(p.contains("<review>\n\n</review>") || p.contains("<review>\n</review>"));
        assert!(p.contains("If the block above is empty"));
        assert!(p.contains("markplane show TASK-abc12"));
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

    #[test]
    fn merge_prompt_clean_vs_conflict_verify_and_notes() {
        let p = render_merge_prompt(&ctx());
        assert!(p.contains("build or typecheck only"));
        assert!(p.contains("full test suite"));
        assert!(p.contains("**Merge notes**"));
        assert!(p.contains("git merge --abort"));
        assert!(p.contains("git add -A"));
        assert!(p.contains("git stash"));
        assert!(p.contains(".markplane/"));
        assert!(p.contains("keep `HEAD`'s version"));
    }
}
