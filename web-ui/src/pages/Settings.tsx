import { useState, useEffect } from "react";
import { api } from "../api/client";

export default function Settings() {
  const [apiKey, setApiKey] = useState(() => localStorage.getItem("apiKey") || "");
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [providers, setProviders] = useState<any[]>([]);
  const [channels, setChannels] = useState<any[]>([]);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");

  useEffect(() => {
    api.providers().then((r) => setProviders(r.providers || [])).catch(() => {});
    api.channels().then((r) => setChannels(r.channels || [])).catch(() => {});
  }, []);

  function saveApiKey() {
    const trimmed = apiKeyInput.trim();
    if (trimmed) {
      localStorage.setItem("apiKey", trimmed);
      setApiKey(trimmed);
      setApiKeyInput("");
      setMessage("API key saved");
    } else {
      localStorage.removeItem("apiKey");
      setApiKey("");
      setMessage("API key removed");
    }
    setTimeout(() => setMessage(""), 2000);
  }

  return (
    <div className="animate-in space-y-8">
      <header>
        <h1 className="text-3xl font-light tracking-tight text-white">Settings</h1>
        <p className="mt-1.5 text-sm text-slate-500">Configure API keys, providers, and channels</p>
      </header>

      {/* Message */}
      {message && (
        <div className="rounded-xl border border-emerald-400/20 bg-emerald-400/5 px-4 py-3 text-sm text-emerald-300">
          {message}
        </div>
      )}

      {/* API Key */}
      <section className="rounded-2xl border border-white/[0.06] bg-white/[0.02] p-5">
        <h2 className="text-sm font-medium text-slate-300">API Key</h2>
        <p className="mt-1 text-xs text-slate-600">
          Used for authenticated requests to the agent service.
        </p>
        <div className="mt-4 flex gap-3">
          <input
            value={apiKeyInput}
            onChange={(e) => setApiKeyInput(e.target.value)}
            placeholder={apiKey ? "Replace existing key..." : "Enter API key..."}
            className="flex-1 rounded-xl border border-white/[0.08] bg-slate-950/60 px-4 py-2.5 text-sm text-white placeholder-slate-600 outline-none focus:border-cyan-400/30"
            onKeyDown={(e) => e.key === "Enter" && saveApiKey()}
          />
          <button
            onClick={saveApiKey}
            className="rounded-xl bg-cyan-400/10 px-5 py-2.5 text-sm font-medium text-cyan-200 transition-all hover:bg-cyan-400/20"
          >
            {apiKeyInput.trim() ? "Save" : "Clear"}
          </button>
        </div>
        {apiKey && (
          <p className="mt-2 text-xs text-emerald-400/60">
            ✓ Key configured ({apiKey.slice(0, 8)}...)
          </p>
        )}
      </section>

      {/* LLM Providers */}
      <section className="rounded-2xl border border-white/[0.06] bg-white/[0.02] p-5">
        <h2 className="text-sm font-medium text-slate-300">LLM Providers</h2>
        {providers.length === 0 ? (
          <p className="mt-3 text-sm text-slate-600">
            No providers configured. Run{" "}
            <code className="rounded bg-white/[0.04] px-2 py-0.5 text-xs text-cyan-400/60">npm run setup</code>{" "}
            to add one.
          </p>
        ) : (
          <ul className="mt-3 space-y-2">
            {providers.map((p: any) => (
              <li key={p.id} className="flex items-center justify-between rounded-lg border border-white/[0.04] bg-white/[0.01] px-4 py-2.5">
                <div>
                  <span className="text-sm font-medium text-slate-200">{p.id}</span>
                  {p.model && <span className="ml-2 text-xs text-slate-600">{p.model}</span>}
                </div>
                <span className={`text-xs ${p.enabled ? "text-emerald-400" : "text-slate-600"}`}>
                  {p.enabled ? "Enabled" : "Disabled"}
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>

      {/* Channels */}
      <section className="rounded-2xl border border-white/[0.06] bg-white/[0.02] p-5">
        <h2 className="text-sm font-medium text-slate-300">Messaging Channels</h2>
        {channels.length === 0 ? (
          <p className="mt-3 text-sm text-slate-600">
            No channels configured. Run{" "}
            <code className="rounded bg-white/[0.04] px-2 py-0.5 text-xs text-cyan-400/60">npm run setup</code>{" "}
            to set up Telegram, Discord, etc.
          </p>
        ) : (
          <ul className="mt-3 space-y-2">
            {channels.map((c: any) => (
              <li key={c.id} className="flex items-center justify-between rounded-lg border border-white/[0.04] bg-white/[0.01] px-4 py-2.5">
                <span className="text-sm text-slate-200">{c.label}</span>
                <span className={`text-xs ${c.enabled ? "text-emerald-400" : "text-slate-600"}`}>
                  {c.enabled ? "Active" : "Inactive"}
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
