import { removeProject, renameProject, setProjectArmed, DEMO_PROJECT } from "@/lib/config";
import { resetStore } from "@/lib/store";
import { ok, fail } from "@/lib/api";
import { z } from "zod";

export const dynamic = "force-dynamic";

type Ctx = { params: Promise<{ id: string }> };

export async function DELETE(_req: Request, { params }: Ctx) {
  try {
    const { id } = await params;
    removeProject(id);
    resetStore(id);
    return ok({ removed: id });
  } catch (e) {
    return fail(e);
  }
}

const patchSchema = z
  .object({
    name: z.string().min(1).optional(),
    armed: z.boolean().optional(),
  })
  .refine((p) => p.name !== undefined || p.armed !== undefined, {
    message: "name or armed is required",
  });

export async function PATCH(req: Request, { params }: Ctx) {
  try {
    const { id } = await params;
    const patch = patchSchema.parse(await req.json());
    if (id === DEMO_PROJECT.id && patch.armed !== undefined) {
      return ok({ error: "demo cannot be armed" }, 400);
    }
    if (patch.name !== undefined) {
      const entry = renameProject(id, patch.name);
      if (!entry) return ok({ error: "not found" }, 404);
      if (patch.armed === undefined) return ok(entry);
    }
    if (patch.armed !== undefined) {
      const entry = setProjectArmed(id, patch.armed);
      if (!entry) return ok({ error: "not found" }, 404);
      return ok(entry);
    }
    return ok({ error: "not found" }, 404);
  } catch (e) {
    return fail(e);
  }
}
