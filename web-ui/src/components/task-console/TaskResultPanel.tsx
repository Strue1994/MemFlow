import type { TaskExecutionResponse } from '../../api/client'

export default function TaskResultPanel({ result }: { result: TaskExecutionResponse | null }) {
  if (!result) {
    return (
      <section className="rounded-[28px] border border-dashed border-white/12 bg-slate-950/60 p-6">
        <div className="text-xs uppercase tracking-[0.22em] text-slate-500">Result</div>
        <p className="mt-3 text-sm leading-6 text-slate-400">
          Result details, raw JSON, and follow-up guidance will appear here after the first run.
        </p>
      </section>
    )
  }

  return (
    <section className="rounded-[28px] border border-white/10 bg-slate-950/70 p-6">
      <div className="text-xs uppercase tracking-[0.22em] text-cyan-300">Result</div>
      <pre className="mt-4 overflow-x-auto rounded-3xl border border-white/8 bg-slate-900/95 p-4 text-xs leading-6 text-slate-200">
        {JSON.stringify(result, null, 2)}
      </pre>
    </section>
  )
}
