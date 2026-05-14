import { useEffect, useMemo, useState } from 'react'
import { taskApi } from '../api/client'
import { useLanguage } from '../lib/language'

type TaskHistoryItem = Record<string, unknown>

function getTaskText(item: TaskHistoryItem): string {
  const candidates = [
    item.text,
    item.task,
    item.task_text,
    item.request,
    item.request_text,
    item.input,
    item.prompt,
  ]

  const match = candidates.find((value) => typeof value === 'string' && value.trim())
  return typeof match === 'string' ? match : 'Untitled task'
}

function getRouteEntries(item: TaskHistoryItem): string[] {
  const routeFields = [
    item.routes,
    item.route_list,
    item.route_history,
    item.route_trace,
    item.route,
  ]

  for (const field of routeFields) {
    if (Array.isArray(field)) {
      const values = field
        .map((entry) => {
          if (typeof entry === 'string') {
            return entry
          }
          if (entry && typeof entry === 'object') {
            const value = (entry as Record<string, unknown>).route ?? (entry as Record<string, unknown>).name ?? (entry as Record<string, unknown>).label
            return typeof value === 'string' ? value : null
          }
          return null
        })
        .filter((value): value is string => Boolean(value))

      if (values.length > 0) {
        return values
      }
    }

    if (typeof field === 'string' && field.trim()) {
      return [field]
    }
  }

  return []
}

function getTimestamp(item: TaskHistoryItem): string | null {
  const candidates = [item.created_at, item.createdAt, item.started_at, item.startedAt]
  const match = candidates.find((value) => typeof value === 'string' && value.trim())
  return typeof match === 'string' ? match : null
}

export default function TaskHistoryPage() {
  const { text } = useLanguage()
  const [items, setItems] = useState<TaskHistoryItem[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let active = true

    async function load() {
      try {
        const history = await taskApi.history()
        if (!active) {
          return
        }
        setItems(history)
        setError(null)
      } catch (err: any) {
        if (!active) {
          return
        }
        setError(err?.response?.data?.error || err?.message || text({ zh: '加载执行历史失败', en: 'Failed to load execution history' }))
      } finally {
        if (active) {
          setLoading(false)
        }
      }
    }

    load()

    return () => {
      active = false
    }
  }, [text])

  const historyCards = useMemo(
    () =>
      items.map((item, index) => ({
        id: String(item.id ?? item.task_id ?? index),
        taskText: getTaskText(item),
        routes: getRouteEntries(item),
        timestamp: getTimestamp(item),
      })),
    [items],
  )

  return (
    <div className="mx-auto flex h-full w-full max-w-6xl flex-col gap-6 px-6 py-6">
      <section className="rounded-[28px] border border-white/10 bg-slate-950/70 p-6 shadow-[0_24px_60px_rgba(2,6,23,0.35)]">
        <div className="text-xs uppercase tracking-[0.24em] text-cyan-300">
          {text({ zh: '执行历史', en: 'Execution History' })}
        </div>
        <h1 className="mt-2 text-3xl font-semibold tracking-tight text-white">
          {text({ zh: '任务请求与路由轨迹', en: 'Task requests and routing traces' })}
        </h1>
        <p className="mt-3 max-w-3xl text-sm leading-6 text-slate-400">
          {text({
            zh: '这里显示自然语言任务历史，以及每次请求经过的路由路径，便于确认是命中工作流、走代理，还是要求澄清。',
            en: 'This page shows natural-language task history together with the routing path each request followed.',
          })}
        </p>
      </section>

      <section className="rounded-[28px] border border-white/10 bg-slate-950/60 p-6">
        {loading ? (
          <div className="py-16 text-center text-slate-500">{text({ zh: '正在加载历史…', en: 'Loading history…' })}</div>
        ) : error ? (
          <div className="rounded-2xl border border-rose-500/30 bg-rose-500/10 px-4 py-3 text-sm text-rose-200">{error}</div>
        ) : historyCards.length === 0 ? (
          <div className="py-16 text-center text-slate-500">{text({ zh: '还没有任务执行历史。', en: 'No task history yet.' })}</div>
        ) : (
          <div className="space-y-4">
            {historyCards.map((item) => (
              <article key={item.id} className="rounded-3xl border border-white/8 bg-white/[0.03] px-5 py-4">
                <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                  <div className="min-w-0">
                    <h2 className="text-base font-semibold text-white">{item.taskText}</h2>
                    {item.timestamp ? (
                      <p className="mt-2 text-xs text-slate-500">{item.timestamp}</p>
                    ) : null}
                  </div>
                  <div className="rounded-full border border-white/10 bg-white/[0.04] px-3 py-1 text-xs text-slate-300">
                    {text({
                      zh: `${item.routes.length || 0} 个路由节点`,
                      en: `${item.routes.length || 0} route entries`,
                    })}
                  </div>
                </div>

                <div className="mt-4">
                  <div className="mb-2 text-xs uppercase tracking-[0.2em] text-slate-500">
                    {text({ zh: '路由列表', en: 'Route List' })}
                  </div>
                  {item.routes.length === 0 ? (
                    <p className="text-sm text-slate-500">{text({ zh: '没有可显示的路由信息。', en: 'No route details available.' })}</p>
                  ) : (
                    <div className="flex flex-wrap gap-2">
                      {item.routes.map((route, index) => (
                        <span
                          key={`${item.id}-${route}-${index}`}
                          className="rounded-full border border-cyan-400/20 bg-cyan-400/10 px-3 py-1 text-xs text-cyan-100"
                        >
                          {route}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              </article>
            ))}
          </div>
        )}
      </section>
    </div>
  )
}
