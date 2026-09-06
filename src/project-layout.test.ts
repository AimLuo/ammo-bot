import { describe, expect, it } from "vitest";
import {
  mergeSyncFrom,
  originUrl,
  repoPrefixesFromDiff,
  resolveLayoutKind,
  subtreeRemoteUrl,
} from "./project-layout.ts";

describe("resolveLayoutKind", () => {
  it("treats Domain docs multi-context as multi", () => {
    const md = `### Domain docs\n\nmulti-context。见 \`docs/agents/domain.md\`。\n`;
    expect(resolveLayoutKind(md, false)).toBe("multi");
  });

  it("treats Domain docs single-context as single even when repos/ exists", () => {
    const md = `### Domain docs\n\nsingle-context。见 \`docs/agents/domain.md\`。\n`;
    expect(resolveLayoutKind(md, true)).toBe("single");
  });

  it("treats 多仓（multi-context） as multi", () => {
    const md = `## Setup\n\n布局：多仓（multi-context）\n`;
    expect(resolveLayoutKind(md, false)).toBe("multi");
  });

  it("falls back to Git subtree line when Domain docs is missing", () => {
    const md = `## Git\n\n父仓 commit 后 push\nrepos/ 下有 git 地址的用 subtree，各自 push；本空间代码跟父仓一起提交\n`;
    expect(resolveLayoutKind(md, false)).toBe("multi");
  });

  it("prefers Domain docs over the Git subtree line", () => {
    const md = `## Git\n\nrepos/ 下有 git 地址的用 subtree，各自 push\n\n### Domain docs\n\nsingle-context\n`;
    expect(resolveLayoutKind(md, true)).toBe("single");
  });

  it("uses repos/ when AGENTS.md is missing", () => {
    expect(resolveLayoutKind(null, true)).toBe("multi");
    expect(resolveLayoutKind(null, false)).toBe("single");
  });

  it("treats a section that says both single and multi as missing", () => {
    const md = `### Domain docs\n\n单仓 multi-context\n`;
    expect(resolveLayoutKind(md, true)).toBe("multi");
    expect(resolveLayoutKind(md, false)).toBe("single");
  });
});

describe("subtreeRemoteUrl", () => {
  const remotes = [
    { name: "origin", url: "git@github.com:org/parent.git" },
    { name: "ammo-bot", url: "https://github.com/org/ammo-bot.git" },
    { name: "repos/sandcastle", url: "https://github.com/org/sandcastle.git" },
  ];

  it("matches a remote named after the prefix", () => {
    expect(subtreeRemoteUrl("ammo-bot", remotes)).toBe(
      "https://github.com/org/ammo-bot.git",
    );
  });

  it("matches a remote named repos/<prefix>", () => {
    expect(subtreeRemoteUrl("sandcastle", remotes)).toBe(
      "https://github.com/org/sandcastle.git",
    );
  });

  it("matches a remote URL basename when the name differs", () => {
    expect(
      subtreeRemoteUrl("scotty", [
        { name: "origin", url: "git@github.com:org/parent.git" },
        { name: "vendor", url: "git@github.com:org/scotty.git" },
      ]),
    ).toBe("git@github.com:org/scotty.git");
  });

  it("does not use origin as a subtree remote", () => {
    expect(
      subtreeRemoteUrl("parent", [
        { name: "origin", url: "git@github.com:org/parent.git" },
      ]),
    ).toBeNull();
  });
});

describe("originUrl", () => {
  it("returns the origin remote URL", () => {
    expect(
      originUrl([
        { name: "origin", url: "git@github.com:org/parent.git" },
        { name: "ammo-bot", url: "https://github.com/org/ammo-bot.git" },
      ]),
    ).toBe("git@github.com:org/parent.git");
  });

  it("returns null when origin is absent", () => {
    expect(originUrl([{ name: "ammo-bot", url: "https://x/y.git" }])).toBeNull();
  });
});

describe("repoPrefixesFromDiff", () => {
  it("collects unique repos/<name> prefixes", () => {
    expect(
      repoPrefixesFromDiff([
        "repos/ammo-bot/src/pipeline.ts",
        "repos/ammo-bot/CONTEXT.md",
        "AGENTS.md",
        "repos/sandcastle/src/index.ts",
      ]),
    ).toEqual(["ammo-bot", "sandcastle"]);
  });
});

describe("mergeSyncFrom", () => {
  it("omits subtree prefixes for a single-context project", () => {
    expect(
      mergeSyncFrom({
        agentsMd: "### Domain docs\n\nsingle-context\n",
        hasReposDir: true,
        remotes: [
          { name: "origin", url: "git@github.com:org/parent.git" },
          { name: "ammo-bot", url: "https://github.com/org/ammo-bot.git" },
        ],
        diffPaths: ["repos/ammo-bot/src/pipeline.ts"],
      }),
    ).toEqual({
      kind: "single",
      originUrl: "git@github.com:org/parent.git",
      subtrees: [],
    });
  });

  it("maps changed multi-context prefixes to remotes", () => {
    expect(
      mergeSyncFrom({
        agentsMd: "### Domain docs\n\nmulti-context\n",
        hasReposDir: true,
        remotes: [
          { name: "origin", url: "git@github.com:org/parent.git" },
          { name: "ammo-bot", url: "https://github.com/org/ammo-bot.git" },
        ],
        diffPaths: ["repos/ammo-bot/src/pipeline.ts", "repos/missing/a.ts"],
      }),
    ).toEqual({
      kind: "multi",
      originUrl: "git@github.com:org/parent.git",
      subtrees: [
        { name: "ammo-bot", url: "https://github.com/org/ammo-bot.git" },
        { name: "missing", url: null },
      ],
    });
  });
});

