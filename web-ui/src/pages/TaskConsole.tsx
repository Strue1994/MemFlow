import { useState, useRef, useEffect } from "react";
import { api } from "../api/client";

export default function TaskConsole() {
  const [text, setText] = useState("");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [history, setHistory] = useState<Array<{ text: string; result: string; error?: string; time: number }>>([]);
  const preRef = useRef<HTMLPreElement>(null);

  useEffect(() => {
    if (preRef.current) {
      preRef.current.scrollTop = preRef.current.scrollHeight;
    }
  }, [result, error]);

  async function handleSubmit() {
    const t = text.trim();
    if (!t || loading) return;
    setLoading(true);
    setResult(null);
    setError(null);
    try {
      const r = await api.execute(t);
      const output = r.output || JSON.stringify(r, null, 2);
      setResult(output);
      setHistory((prev) => [{ text: t, result: output, time: Date.now() }, ...prev].slice(0, 20));
    } catch (e: any) {
      const msg = e.message || String(e);
      setError(msg);
      setHistory((prev) => [{ text: t, result: msg, error: msg, time: Date.now() }, ...prev].slice(0, 20));
    } finally {
      setLoading(false);
      setText("");
    }
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }

  return (
    <div className="animate-in space-y-6">
      <header>
        <h1 className="text-3xl font-light tracking-tight text-white">Task Console</h1>
        <p className="mt-1.5 text-sm text-slate-500">Describe the job, MemFlow decides the route.</p>
      </header>

      {/* Input */}
      <div className="group relative">
        <div className="absolute -inset-0.5 rounded-2xl bg-gradient-to-r from-cyan-400/10 to-blue-400/5 opacity-0 transition-opacity duration-300 group-focus-within:opacity-100" />
        <div className="relative flex gap-3 rounded-2xl border border-white/[0.08] bg-slate-950/80 p-1.5 backdrop-blur-xl">
          <input
            autoFocus
            value={text}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder='e.g. "What is the weather in Shanghai?"'
            className="flex-1 rounded-xl border-0 bg-transparent px-4 py-3 text-sm text-white placeholder-slate-600 outline-none"
            disabled={loading}
          />
          <button
            onClick={handleSubmit}
            disabled={loading || !text.trim()}
            className="flex items-center gap-2 rounded-xl bg-cyan-400/10 px-5 py-3 text-sm font-medium text-cyan-200 transition-all hover:bg-cyan-400/20 disabled:opacity-30 disabled:cursor-not-allowed"
          >
            {loading ? (
              <span className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-cyan-400/30 border-t-cyan-400" />
            ) : (
              "Run"
            )}
          </button>
        </div>
      </div>

      {/* Result */}
      {(result || error) && (
        <div className="rounded-2xl border border-white/[0.06] bg-white/[0.02] overflow-hidden">
          <div className="flex items-center justify-between border-b border-white/[0.04] px-5 py-3">
            <span className="text-xs font-medium tracking-wider text-slate-500 uppercase">
              {error ? "Error" : "Result"}
            </span>
            <button
              onClick={() => { setResult(null); setError(null); }}
              className="text-xs text-slate-600 hover:text-slate-400 transition-colors"
            >
              Clear
            </button>
          </div>
          <pre
            ref={preRef}
            className={`max-h-80 overflow-auto p-5 text-sm leading-relaxed ${
              error ? "text-red-300/80" : "text-slate-300"
            }`}
          >
            {error || result}
          </pre>
        </div>
      )}

      {/* History */}
      {history.length > 0 && (
        <div>
          <h3 className="mb-3 text-xs font-medium tracking-wider text-slate-600 uppercase">Recent</h3>
          <div className="space-y-2">
            {history.map((item, i) => (
              <button
                key={i}
                onClick={() => {
                  setText(item.text);
                  setResult(item.result);
                  setError(item.error || null);
                }}
                className="w-full rounded-xl border border-white/[0.04] bg-white/[0.01] px-4 py-3 text-left transition-all hover:bg-white/[0.04] text-sm"
              >
                <div className="flex items-center justify-between">
                  <span className="truncate text-slate-400">{item.text}</span>
                  <span className="ml-3 shrink-0 text-[10px] text-slate-600">
                    {new Date(item.time).toLocaleTimeString()}
                  </span>
                </div>
                {item.error ? (
                  <span className="mt-1 block truncate text-xs text-red-400/60">{item.error}</span>
                ) : (
                  <span className="mt-1 block truncate text-xs text-slate-600">
                    {item.result.slice(0, 80)}
                    {item.result.length > 80 ? "..." : ""}
                  </span>
                )}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
