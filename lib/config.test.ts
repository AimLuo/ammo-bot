import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("server-only", () => ({}));

import { listArmedProjects } from "./config.ts";

const PROJECT = {
  id: "meicheng",
  name: "meicheng",
  path: "/tmp/meicheng",
  addedAt: "2026-09-07T14:55:31.716Z",
  lastOpened: "2026-09-07T14:55:36.080Z",
  armed: false,
};

function configJson(armed: boolean) {
  return JSON.stringify(
    {
      humanActor: "s2",
      humanAllowlist: ["s2"],
      pollIntervalMs: 30000,
      projects: [{ ...PROJECT, armed }],
      orders: {},
      gamification: false,
    },
    null,
    2,
  );
}

describe("listArmedProjects", () => {
  let tmp: string;
  let prevXdg: string | undefined;

  beforeEach(() => {
    tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ammo-config-"));
    prevXdg = process.env.XDG_CONFIG_HOME;
    process.env.XDG_CONFIG_HOME = tmp;
    fs.mkdirSync(path.join(tmp, "ammo-bot"), { recursive: true });
    fs.writeFileSync(path.join(tmp, "ammo-bot", "config.json"), configJson(false));
  });

  afterEach(() => {
    if (prevXdg === undefined) delete process.env.XDG_CONFIG_HOME;
    else process.env.XDG_CONFIG_HOME = prevXdg;
    fs.rmSync(tmp, { recursive: true, force: true });
  });

  it("sees Armed after another writer persists it to disk", () => {
    expect(listArmedProjects()).toEqual([]);

    fs.writeFileSync(path.join(tmp, "ammo-bot", "config.json"), configJson(true));

    expect(listArmedProjects().map((p) => p.id)).toEqual(["meicheng"]);

    fs.writeFileSync(path.join(tmp, "ammo-bot", "config.json"), configJson(false));

    expect(listArmedProjects()).toEqual([]);
  });
});
