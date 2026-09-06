export type LayoutKind = "single" | "multi";

export type GitRemote = {
  name: string;
  url: string;
};

export type MergeSync = {
  kind: LayoutKind;
  originUrl: string | null;
  subtrees: { name: string; url: string | null }[];
};

export function mergeSyncFrom(input: {
  agentsMd: string | null;
  hasReposDir: boolean;
  remotes: GitRemote[];
  diffPaths: string[];
}): MergeSync {
  const kind = resolveLayoutKind(input.agentsMd, input.hasReposDir);
  const prefixes =
    kind === "multi" ? repoPrefixesFromDiff(input.diffPaths) : [];
  return {
    kind,
    originUrl: originUrl(input.remotes),
    subtrees: prefixes.map((name) => ({
      name,
      url: subtreeRemoteUrl(name, input.remotes),
    })),
  };
}

export function resolveLayoutKind(
  agentsMd: string | null,
  hasReposDir: boolean,
): LayoutKind {
  const fromDocs = layoutKindFromDomainDocs(agentsMd);
  if (fromDocs) return fromDocs;
  const fromGit = layoutKindFromGitSection(agentsMd);
  if (fromGit) return fromGit;
  const fromFile = agentsMd ? kindFromText(agentsMd) : null;
  if (fromFile) return fromFile;
  return hasReposDir ? "multi" : "single";
}

export function originUrl(remotes: GitRemote[]): string | null {
  return remotes.find((r) => r.name === "origin")?.url ?? null;
}

export function subtreeRemoteUrl(
  prefix: string,
  remotes: GitRemote[],
): string | null {
  const named = remotes.find(
    (r) =>
      r.name !== "origin" &&
      (r.name === prefix || r.name === `repos/${prefix}`),
  );
  if (named) return named.url;
  const byBasename = remotes.find(
    (r) => r.name !== "origin" && urlBasename(r.url) === prefix,
  );
  return byBasename?.url ?? null;
}

export function repoPrefixesFromDiff(paths: string[]): string[] {
  const seen = new Set<string>();
  const prefixes: string[] = [];
  for (const path of paths) {
    const match = /^repos\/([^/]+)/.exec(path);
    const name = match?.[1];
    if (!name || seen.has(name)) continue;
    seen.add(name);
    prefixes.push(name);
  }
  return prefixes;
}

function layoutKindFromDomainDocs(agentsMd: string | null): LayoutKind | null {
  if (!agentsMd) return null;
  const section = sectionBody(agentsMd, "Domain docs");
  if (section === null) return null;
  return kindFromText(section);
}

function layoutKindFromGitSection(agentsMd: string | null): LayoutKind | null {
  if (!agentsMd) return null;
  const section = sectionBody(agentsMd, "Git");
  if (section === null) return null;
  if (/各自 push|subtree/.test(section)) return "multi";
  return null;
}

function kindFromText(text: string): LayoutKind | null {
  const single = /单仓|single-context/.test(text);
  const multi = /多仓|multi-context/.test(text);
  if (single && multi) return null;
  if (multi) return "multi";
  if (single) return "single";
  return null;
}

function sectionBody(md: string, heading: string): string | null {
  const re = new RegExp(`^#{2,3}\\s+${heading}\\s*$`, "im");
  const match = re.exec(md);
  if (!match || match.index === undefined) return null;
  const start = match.index + match[0].length;
  const rest = md.slice(start);
  const next = /^#{2,3}\s+/m.exec(rest);
  return next ? rest.slice(0, next.index) : rest;
}

function urlBasename(url: string): string {
  const trimmed = url.replace(/\/+$/, "").replace(/\.git$/, "");
  const slash = trimmed.lastIndexOf("/");
  const colon = trimmed.lastIndexOf(":");
  const cut = Math.max(slash, colon);
  return cut >= 0 ? trimmed.slice(cut + 1) : trimmed;
}
