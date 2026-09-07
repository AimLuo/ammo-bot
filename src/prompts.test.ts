import { describe, expect, it } from "vitest";
import type { MergeSync } from "./project-layout.ts";
import { implementPrompt, mergePrompt, reviewPrompt } from "./prompts.ts";
import { COMPLETION_SIGNAL } from "./types.ts";

const base = {
  beadId: "bead-1",
  branch: "ammo/issue-bead-1",
  targetBranch: "main",
};

function promptFor(sync: MergeSync) {
  return mergePrompt({ ...base, sync });
}

describe("mergePrompt", () => {
  it("does not close the bead", () => {
    const prompt = promptFor({ kind: "single", originUrl: null, subtrees: [] });
    expect(prompt).toContain("Do not close the bead");
    expect(prompt).not.toMatch(/Close the bead with/);
  });

  it("does not ask the agent to read AGENTS.md", () => {
    const prompt = promptFor({
      kind: "single",
      originUrl: "git@github.com:org/parent.git",
      subtrees: [],
    });
    expect(prompt).not.toMatch(/read AGENTS\.md/i);
    expect(prompt).not.toContain("请读取");
  });

  it("asks for git push when origin exists on a single-context project", () => {
    const prompt = promptFor({
      kind: "single",
      originUrl: "git@github.com:org/parent.git",
      subtrees: [],
    });
    expect(prompt).toContain("single-context");
    expect(prompt).toContain("git push");
    expect(prompt).not.toContain("git subtree push");
    expect(prompt).not.toMatch(/git push --force/);
  });

  it("skips parent push when origin is absent", () => {
    const prompt = promptFor({ kind: "single", originUrl: null, subtrees: [] });
    expect(prompt).not.toContain("git push");
  });

  it("pushes changed subtrees before origin on a multi-context project", () => {
    const prompt = promptFor({
      kind: "multi",
      originUrl: "git@github.com:org/parent.git",
      subtrees: [
        { name: "ammo-bot", url: "https://github.com/org/ammo-bot.git" },
      ],
    });
    expect(prompt).toContain("multi-context");
    expect(prompt).toContain(
      "git subtree push --prefix=repos/ammo-bot https://github.com/org/ammo-bot.git main",
    );
    const subtreeAt = prompt.indexOf("git subtree push");
    const originAt = prompt.indexOf("git push");
    expect(subtreeAt).toBeGreaterThan(0);
    expect(originAt).toBeGreaterThan(subtreeAt);
  });

  it("requires a human flag when a changed subtree has no remote URL", () => {
    const prompt = promptFor({
      kind: "multi",
      originUrl: null,
      subtrees: [{ name: "ammo-bot", url: null }],
    });
    expect(prompt).toContain("repos/ammo-bot");
    expect(prompt).toContain("--add-label human");
    expect(prompt).not.toContain("git subtree push --prefix=repos/ammo-bot");
  });

  it("still lists subtree pushes that have URLs when another prefix is missing", () => {
    const prompt = promptFor({
      kind: "multi",
      originUrl: "git@github.com:org/parent.git",
      subtrees: [
        { name: "ammo-bot", url: "https://github.com/org/ammo-bot.git" },
        { name: "missing", url: null },
      ],
    });
    expect(prompt).toContain(
      "git subtree push --prefix=repos/ammo-bot https://github.com/org/ammo-bot.git main",
    );
    expect(prompt).toContain("repos/missing");
    expect(prompt).toContain("git push");
  });

  it("skips merge and typecheck when the issue branch is already on the target", () => {
    const prompt = promptFor({
      kind: "single",
      originUrl: "git@github.com:org/parent.git",
      subtrees: [],
    });
    expect(prompt).toContain("git merge-base --is-ancestor");
    expect(prompt).toContain("only finish remote sync");
  });
});

describe("implementPrompt", () => {
  it("asks the agent to follow this prompt and the bead", () => {
    const prompt = implementPrompt({
      beadId: "bead-1",
      title: "add login",
      branch: "ammo/issue-bead-1",
    });
    expect(prompt).toContain("This prompt is the implementation");
    expect(prompt).toContain("bd show bead-1");
    expect(prompt).toContain(COMPLETION_SIGNAL);
    expect(prompt).not.toContain("/code-review");
  });
});

describe("reviewPrompt", () => {
  it("asks the agent to improve existing commits from this prompt", () => {
    const prompt = reviewPrompt({
      beadId: "bead-1",
      branch: "ammo/issue-bead-1",
      targetBranch: "main",
    });
    expect(prompt).toContain("This prompt is the improve pass");
    expect(prompt).toContain("Improve the existing commits");
    expect(prompt).toContain(COMPLETION_SIGNAL);
    expect(prompt).not.toContain("/code-review");
    expect(prompt).not.toContain("Review the code changes");
  });
});
