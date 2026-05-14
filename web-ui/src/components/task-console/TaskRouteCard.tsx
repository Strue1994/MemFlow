import type { TaskExecutionResponse } from '../../api/client'

const routeLabels: Record<TaskExecutionResponse['route'], string> = {
  workflow: 'Existing workflow',
  generated_workflow: 'Generated workflow',
  agent: 'Agent route',
  clarification: 'Clarification',
}

export default function TaskRouteCard({ result }: { result: TaskExecutionResponse | null }) {
  return (
    <section className="rounded-[28px] border border-white/10 bg-slate-950/70 p-6">
      <div className="text-xs uppercase tracking-[0.22em] text-cyan-300">Routing decision</div>
      <div className="mt-3 text-xl font-semibold text-white">
        {result ? routeLabels[result.route] : 'Awaiting route selection'}
      </div>
      <p className="mt-3 text-sm leading-6 text-slate-300">
        {result ? result.reason : 'The route explanation will appear here after execution.'}
      </p>

      <dl className="mt-5 grid gap-3 sm:grid-cols-2">
        <div className="rounded-2xl border border-white/8 bg-white/[0.03] p-4">
          <dt className="text-xs uppercase tracking-[0.18em] text-slate-500">Confidence</dt>
          <dd className="mt-2 text-sm font-medium text-white">{result?.confidence || 'pending'}</dd>
        </div>
        <div className="rounded-2xl border border-white/8 bg-white/[0.03] p-4">
          <dt className="text-xs uppercase tracking-[0.18em] text-slate-500">Repeatable</dt>
          <dd className="mt-2 text-sm font-medium text-white">{result ? (result.repeatable ? 'yes' : 'no') : 'pending'}</dd>
        </div>
      </dl>
    </section>
  )
}
