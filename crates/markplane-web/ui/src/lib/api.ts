import { apiPath } from "@/lib/project-prefix";
import { throwIfNotOk } from "@/lib/api-error";

const API_BASE = typeof window !== "undefined" ? "" : "http://localhost:4200";

export async function fetcher<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${apiPath(path)}`);
  await throwIfNotOk(res);
  const json = await res.json();
  return json.data;
}

export async function fetchList<T>(path: string): Promise<{ data: T[]; total: number }> {
  const res = await fetch(`${API_BASE}${apiPath(path)}`);
  await throwIfNotOk(res);
  const json = await res.json();
  return { data: json.data, total: json.meta.total };
}

export async function postAction<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${apiPath(path)}`, { method: "POST" });
  await throwIfNotOk(res);
  const json = await res.json();
  return json.data;
}

export async function postJson<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(`${API_BASE}${apiPath(path)}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  await throwIfNotOk(res);
  const json = await res.json();
  return json.data;
}

export async function patchJson<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(`${API_BASE}${apiPath(path)}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  await throwIfNotOk(res);
  const json = await res.json();
  return json.data;
}

export async function deleteAction<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${apiPath(path)}`, { method: "DELETE" });
  await throwIfNotOk(res);
  const json = await res.json();
  return json.data;
}
