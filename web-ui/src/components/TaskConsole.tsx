import { useState } from 'react'
import { toast } from 'sonner'
import { taskApi, type TaskExecutionResponse } from '../api/client'
import TaskRouteCard from './task-console/TaskRouteCard'
import TaskExecutionTimeline from './task-console/TaskExecutionTimeline'
import TaskResultPanel from './task-console/TaskResultPanel'

export default function TaskConsole() {
  const [taskText, setTaskText] = useState('')
  const [loading, setLoading] = useState(false)
  const [result, setResult] = useState<TaskExecutionResponse | null>(null)
  const [error, setError] = useState<string | null>(null)

  async function submitTask() {
    const nextTask = taskText.trim()
    if (!nextTask) {
      return
    }

    setLoading(true)
    setError(null)
    setResult(null)

    try {
      const response = await taskApi.execute(nextTask)
      setResult(response)
    } catch (err: any) {
      const message = err?.response?.data?.error || err?.message || 'Task execution failed.'
      setResult(null)
      setError(message)
      toast.error(message)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="mx-auto flex h-full w-full max-w-6xl flex-col gap-6 px-6 py-6">
      <section className="rounded-[28px] border border-white/10 bg-slate-950/70 p-6 shadow-[0_24px_60px_rgba(2,6,23,0.35)]">
        <div className="flex flex-col gap-5 lg:flex-row lg:items-end lg:justify-between">
          <div className="max-w-2xl">
            <div className="text-xs uppercase tracking-[0.24em] text-cyan-300">Task Console</div>
            <h1 className="mt-2 text-3xl font-semibold tracking-tight text-white">Describe the job, not the route.</h1>
            <p className="mt-3 text-sm leading-6 text-slate-400">
              MemFlow will decide whether this request should run through an existing workflow, generate a reusable one, ask for clarification, or hand off to the agent path.
            </p>
          </div>
          <button
            type="button"
            onClick={submitTask}
            disabled={loading || !taskText.trim()}
            className="inline-flex min-w-[140px] items-center justify-center rounded-2xl bg-cyan-400 px-4 py-3 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {loading ? 'Running...' : 'Run task'}
          </button>
        </div>

        <label htmlFor="task-console-input" className="mt-6 block text-sm font-medium text-slate-200">
          Task request
        </label>
        <textarea
          id="task-console-input"
          value={taskText}
          onChange={(event) => setTaskText(event.target.value)}
          placeholder="Describe a task or automation request"
          className="mt-3 min-h-44 w-full rounded-3xl border border-white/10 bg-slate-900/90 px-4 py-4 text-sm text-white outline-none transition placeholder:text-slate-500 focus:border-cyan-400/40 focus:ring-2 focus:ring-cyan-400/20"
        />

        {error ? (
          <div className="mt-4 rounded-2xl border border-rose-500/30 bg-rose-500/10 px-4 py-3 text-sm text-rose-200">
            {error}
          </div>
        ) : null}
      </section>

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.2fr)_minmax(320px,0.8fr)]">
        <TaskExecutionTimeline result={result} />
        <TaskRouteCard result={result} />
      </div>

      <TaskResultPanel result={result} />
    </div>
  )
}
