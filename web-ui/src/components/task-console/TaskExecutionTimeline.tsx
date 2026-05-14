import type { TaskExecutionResponse } from '../../api/client'

function buildSteps(result: TaskExecutionResponse | null) {
  if (!result) {
    return [
      { title: 'Task received', detail: 'The console is waiting for your request.' },
      { title: 'Routing decision', detail: 'MemFlow will evaluate the best execution path.' },
      { title: 'Execution result', detail: 'Results and follow-up guidance will appear here.' },
    ]
  }

  return [
    { title: 'Task received', detail: 'Request accepted by the task console.' },
    { title: 'Routing decision', detail: `Selected route: ${result.route}.` },
    {
      title: result.success ? 'Execution completed' : 'Execution needs attention',
      detail: result.success
        ? result.workflow
          ? `Workflow ${result.workflow.workflowId} handled the request.`
          : 'The agent path returned a response.'
        : result.clarificationQuestion || result.failureCategory || 'The task did not complete successfully.',
    },
  ]
}

export default function TaskExecutionTimeline({ result }: { result: TaskExecutionResponse | null }) {
  const steps = buildSteps(result)

  return (
    <section className="rounded-[28px] border border-white/10 bg-slate-950/70 p-6">
      <div className="text-xs uppercase tracking-[0.22em] text-cyan-300">Execution timeline</div>
      <ol className="mt-5 space-y-4">
        {steps.map((step, index) => (
          <li key={step.title} className="flex gap-4">
            <div className="flex flex-col items-center">
              <div className="flex h-8 w-8 items-center justify-center rounded-full bg-cyan-400/15 text-sm font-semibold text-cyan-200">
                {index + 1}
              </div>
              {index < steps.length - 1 ? <div className="mt-2 h-full w-px bg-white/10" /> : null}
            </div>
            <div className="pb-4">
              <div className="text-sm font-medium text-white">{step.title}</div>
              <div className="mt-1 text-sm leading-6 text-slate-400">{step.detail}</div>
            </div>
          </li>
        ))}
      </ol>
    </section>
  )
}
