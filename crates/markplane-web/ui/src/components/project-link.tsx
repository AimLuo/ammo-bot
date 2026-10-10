"use client";

import Link from "next/link";
import type { ComponentProps } from "react";
import { useWithProject } from "@/lib/project-prefix";

type ProjectLinkProps = ComponentProps<typeof Link>;

/**
 * Internal Link that prefixes href with `/p/<id>` after hydration.
 * Pass project-relative paths (e.g. `/backlog?task=…`); already-prefixed
 * `/p/…` paths are left unchanged.
 */
export function ProjectLink({ href, ...props }: ProjectLinkProps) {
  const withProject = useWithProject();
  const resolved =
    typeof href === "string"
      ? withProject(href)
      : href.pathname != null
        ? { ...href, pathname: withProject(href.pathname) }
        : href;
  return <Link href={resolved} {...props} />;
}
