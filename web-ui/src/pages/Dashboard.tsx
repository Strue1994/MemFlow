import { useEffect, useState } from "react";
import { api } from "../api/client";

export default function Dashboard() {
  const [health, setHealth] = useState<{ status: string; uptime_s: number } | null>(null);
  const [skills, setSkills] = useState<any[]>([]);
  const [providers, setProviders] = useState<any[]>([]);
  const [channels, setChannels] = useState<any[]>([]);
  const [goals, setGoals] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      api.health().catch(() => null),
      api.skills().then((r) => r.skills || []).catch(() => []),
      api.providers().then((r) => r.providers || []).catch(() => []),
      api.channels().then((r) => r.channels || []).catch(() => []),
      api.goals().then((r) => r.goals || []).catch(() => []),
    ]).then(([h, s, p, c, g]) => {
      setHealth(h);
      setSkills(s);
      setProviders(p);
      setChannels(c);
      setGoals(g);
    }).finally(() => setLoading(false));
  }, []);

  const uptime = health ? fmt(health.uptime_s) : "—";

  return (
    <div className="animate-in space-y-8">
      <header>
        <h1 className="text-3xl font-light tracking-tight text-white">Dashboard</h1>
        <p className="mt-1.5 text-sm text-slate-500">系统概览</p>
      </header>

      {loading ? (
        <div className="flex items-center justify-center py-20 text-sm text-slate-600">
          <span className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-cyan-400/30 border-t-cyan-400 mr-3" />
          Loading...
        </div>
      ) : (
        <>
          {/* Stats row */}
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <StatCard label="Status" value={health?.status || "unknown"} color="emerald" />
            <StatCard label="Uptime" value={uptime} color="cyan" />
            <StatCard label="Skills" value={String(skills.length)} color="blue" />
            <StatCard label="Providers" value={String(providers.length)} color="violet" />
          </div>

          {/* Details */}
          <div className="grid gap-6 lg:grid-cols-2">
            {/* Providers */}
            <Section title="LLM Providers" count={providers.length}>
              {providers.length === 0 ? (
                <EmptyState message="No providers configured" action="Go to Settings →" to="/settings" />
              ) : (
                <ul className="space-y-2">
                  {providers.map((p: any) => (
                    <li key={p.id} className="flex items-center justify-between rounded-lg border border-white/[0.06] bg-white/[0.02] px-4 py-2.5">
                      <span className="text-sm font-medium text-slate-200">{p.id}</span>
                      <span className="text-xs text-slate-500">{p.model || "—"}</span>
                    </li>
                  ))}
                </ul>
              )}
            </Section>

            {/* Channels */}
            <Section title="Channels" count={channels.length}>
              {channels.length === 0 ? (
                <EmptyState message="No channels configured" action="Configure via API" to="/settings" />
              ) : (
                <ul className="space-y-2">
                  {channels.map((c: any) => (
                    <li key={c.id} className="flex items-center justify-between rounded-lg border border-white/[0.06] bg-white/[0.02] px-4 py-2.5">
                      <span className="text-sm font-medium text-slate-200">{c.label}</span>
                      <span className={`text-xs ${c.enabled ? "text-emerald-400" : "text-slate-600"}`}>
                        {c.enabled ? "Active" : "Off"}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </Section>

            {/* Skills */}
            <Section title="Learned Skills" count={skills.length}>
              {skills.length === 0 ? (
                <p className="text-sm text-slate-600">No skills yet. Run a few tasks to grow skills.</p>
              ) : (
                <ul className="max-h-48 space-y-1.5 overflow-y-auto">
                  {skills.slice(0, 20).map((s: any) => (
                    <li key={s.name} className="flex items-center gap-2 rounded-md px-3 py-1.5 text-sm text-slate-400 hover:bg-white/[0.03]">
                      <span className="text-cyan-400/60">▸</span>
                      {s.name}
                    </li>
                  ))}
                  {skills.length > 20 && (
                    <li className="text-xs text-slate-600 px-3 pt-1">+{skills.length - 20} more</li>
                  )}
                </ul>
              )}
            </Section>

            {/* Goals */}
            <Section title="Active Goals" count={goals.length}>
              {goals.length === 0 ? (
                <p className="text-sm text-slate-600">No active goals. Set a goal via the API.</p>
              ) : (
                <ul className="space-y-2">
                  {goals.map((g: any) => (
                    <li key={g.id} className="rounded-lg border border-white/[0.06] bg-white/[0.02] px-4 py-2.5">
                      <div className="flex items-center justify-between">
                        <span className="text-sm text-slate-200">{g.description}</span>
                        <span className="text-xs text-slate-500">{g.status}</span>
                      </div>
                      <div className="mt-1.5 h-1 w-full overflow-hidden rounded-full bg-white/[0.06]">
                        <div className="h-full rounded-full bg-cyan-400/40 transition-all" style={{ width: `${g.progress || 0}%` }} />
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </Section>
          </div>
        </>
      )}
    </div>
  );
}

function fmt(s: number): string {
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  const parts: string[] = [];
  if (d > 0) parts.push(`${d}d`);
  if (h > 0) parts.push(`${h}h`);
  parts.push(`${m}m`);
  return parts.join(" ");
}

function StatCard({ label, value, color }: { label: string; value: string; color: string }) {
  const colors: Record<string, string> = {
    emerald: "bg-emerald-400/10 border-emerald-400/20 text-emerald-300",
    cyan: "bg-cyan-400/10 border-cyan-400/20 text-cyan-300",
    blue: "bg-blue-400/10 border-blue-400/20 text-blue-300",
    violet: "bg-violet-400/10 border-violet-400/20 text-violet-300",
  };
  return (
    <div className={`rounded-2xl border px-5 py-4 ${colors[color] || colors.cyan}`}>
      <div className="text-[11px] uppercase tracking-widest opacity-60">{label}</div>
      <div className="mt-1 text-2xl font-light">{value}</div>
    </div>
  );
}

function Section({ title, count, children }: { title: string; count: number; children: React.ReactNode }) {
  return (
    <div className="rounded-2xl border border-white/[0.06] bg-white/[0.02] p-5">
      <div className="mb-4 flex items-center justify-between">
        <h3 className="text-sm font-medium text-slate-300">{title}</h3>
        <span className="rounded-full bg-white/[0.04] px-2.5 py-0.5 text-[11px] text-slate-500">{count}</span>
      </div>
      {children}
    </div>
  );
}

function EmptyState({ message, action, to }: { message: string; action: string; to: string }) {
  return (
    <div className="py-6 text-center">
      <p className="text-sm text-slate-600">{message}</p>
      <a href={to} className="mt-2 inline-block text-xs text-cyan-400/60 hover:text-cyan-300 transition-colors">
        {action}
      </a>
    </div>
  );
}
