"use client";

import { useEffect, useState } from "react";
import { ProjectLink } from "@/components/project-link";

type ProjectItem = {
  id: string;
  name: string;
  path: string;
  path_valid: boolean;
  last_opened: string;
};

export default function Home() {
  const [projects, setProjects] = useState<ProjectItem[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetch("/api/projects")
      .then(async (res) => {
        if (!res.ok) {
          throw new Error(`Failed to load projects (${res.status})`);
        }
        const json = await res.json();
        setProjects(json.data ?? []);
      })
      .catch((e: Error) => setError(e.message));
  }, []);

  return (
    <div className="mx-auto max-w-2xl p-8">
      <h1 className="text-2xl font-semibold tracking-tight font-mono">Markplane</h1>
      <p className="mt-2 text-muted-foreground">Choose a Project</p>
      {error && <p className="mt-4 text-destructive">{error}</p>}
      {projects === null && !error && (
        <p className="mt-4 text-muted-foreground">Loading…</p>
      )}
      {projects && projects.length === 0 && (
        <p className="mt-4 text-muted-foreground">
          No projects registered. Run <code className="font-mono">markplane serve</code> inside a
          Project directory.
        </p>
      )}
      {projects && projects.length > 0 && (
        <ul className="mt-6 space-y-2">
          {projects.map((p) => (
            <li key={p.id}>
              <ProjectLink
                href={`/p/${encodeURIComponent(p.id)}/`}
                className="block rounded-md border px-4 py-3 hover:bg-accent"
              >
                <span className="font-medium">{p.name}</span>
                {!p.path_valid && (
                  <span className="ml-2 text-sm text-destructive">unavailable</span>
                )}
                <span className="mt-1 block text-sm text-muted-foreground font-mono">
                  {p.path}
                </span>
              </ProjectLink>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
