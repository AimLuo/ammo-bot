/** Structured API failure with optional Markplane error code / path. */
export class ApiError extends Error {
  readonly status: number;
  readonly code: string | null;
  readonly path: string | null;

  constructor(
    message: string,
    opts: { status: number; code?: string | null; path?: string | null },
  ) {
    super(message);
    this.name = "ApiError";
    this.status = opts.status;
    this.code = opts.code ?? null;
    this.path = opts.path ?? null;
  }

  get isProjectNotFound(): boolean {
    return this.status === 404 && this.code === "project_not_found";
  }

  get isProjectUnavailable(): boolean {
    return this.status === 409 && this.code === "project_unavailable";
  }
}

export async function throwIfNotOk(res: Response): Promise<void> {
  if (res.ok) return;
  const body = await res.json().catch(() => null);
  const detail = body?.error;
  throw new ApiError(detail?.message || `API error: ${res.status}`, {
    status: res.status,
    code: detail?.code ?? null,
    path: detail?.path ?? null,
  });
}
