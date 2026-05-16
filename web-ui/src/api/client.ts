const API = "/api";

async function req<T>(path: string, init?: RequestInit): Promise<T> {
  const key = localStorage.getItem("apiKey");
  const headers: Record<string, string> = { "Content-Type": "application/json" };
  if (key) headers["Authorization"] = `Bearer ${key}`;
  const r = await fetch(`${API}${path}`, { ...init, headers });
  if (!r.ok) {
    const body = await r.text();
    let msg: string;
    try { msg = JSON.parse(body).error || body; } catch { msg = body; }
    throw new Error(msg || `HTTP ${r.status}`);
  }
  const ct = r.headers.get("content-type") || "";
  if (ct.includes("application/json")) return r.json();
  return r.text() as T;
}

export const api = {
  health: () => req<{ status: string; uptime_s: number }>("/health"),

  execute: (text: string) =>
    req<{ success: boolean; output?: string; error?: string }>("/agent/execute", {
      method: "POST",
      body: JSON.stringify({ text, stream: false }),
    }),

  skills: () => req<{ skills: any[] }>("/skills"),

  providers: () => req<{ providers: any[] }>("/providers"),

  saveProvider: (p: { id: string; apiKey: string; model: string; enabled: boolean }) =>
    req<{ success: boolean }>("/providers", {
      method: "POST",
      body: JSON.stringify(p),
    }),

  channels: () => req<{ channels: any[] }>("/channels"),

  metrics: () => req<string>("/metrics"),

  setupStatus: () => req<{ configured: boolean; providerCount: number }>("/setup/status"),

  goals: () =>
    req<{ goals: Array<{ id: string; description: string; progress: number; status: string }> }>("/goals"),

  curatorRun: () =>
    req<{ success: boolean; skillsCreated: number; skillsPruned: number }>("/curator/run", {
      method: "POST",
    }),
};
