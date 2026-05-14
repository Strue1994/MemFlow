import { useEffect, useState } from 'react'
import { workflowApi, type WorkflowInfo } from '../api/client'
import { useLanguage } from '../lib/language'

export default function WorkflowAssetsPage() {
  const { text } = useLanguage()
  const [workflows, setWorkflows] = useState<WorkflowInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let active = true

    async function load() {
      try {
        const items = await workflowApi.listWorkflows()
        if (!active) {
          return
        }
        setWorkflows(items)
        setError(null)
      } catch (err: any) {
        if (!active) {
          return
        }
        setError(err?.response?.data?.error || err?.message || text({ zh: '加载工作流资产失败', en: 'Failed to load workflow assets' }))
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

  return (
    <div className="mx-auto flex h-full w-full max-w-6xl flex-col gap-6 px-6 py-6">
      <section className="rounded-[28px] border border-white/10 bg-slate-950/70 p-6 shadow-[0_24px_60px_rgba(2,6,23,0.35)]">
        <div className="text-xs uppercase tracking-[0.24em] text-cyan-300">
          {text({ zh: '工作流资产', en: 'Workflow Assets' })}
        </div>
        <h1 className="mt-2 text-3xl font-semibold tracking-tight text-white">
          {text({ zh: '已登记的可复用流程', en: 'Reusable workflows already registered' })}
        </h1>
        <p className="mt-3 max-w-3xl text-sm leading-6 text-slate-400">
          {text({
            zh: '这里列出当前 executor 已知的工作流资产，方便确认哪些流程已经存在、可直接复用，而不是每次都重新生成。',
            en: 'This page lists the workflow assets known to the executor so you can see what can be reused instead of regenerated.',
          })}
        </p>
      </section>

      <section className="rounded-[28px] border border-white/10 bg-slate-950/60 p-6">
        {loading ? (
          <div className="py-16 text-center text-slate-500">{text({ zh: '正在加载资产…', en: 'Loading assets…' })}</div>
        ) : error ? (
          <div className="rounded-2xl border border-rose-500/30 bg-rose-500/10 px-4 py-3 text-sm text-rose-200">{error}</div>
        ) : workflows.length === 0 ? (
          <div className="py-16 text-center text-slate-500">{text({ zh: '还没有已登记的工作流资产。', en: 'No workflow assets are registered yet.' })}</div>
        ) : (
          <div className="space-y-3">
            {workflows.map((workflow) => (
              <article
                key={workflow.id}
                className="rounded-3xl border border-white/8 bg-white/[0.03] px-5 py-4 transition hover:border-white/14 hover:bg-white/[0.05]"
              >
                <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
                  <div>
                    <h2 className="text-base font-semibold text-white">{workflow.name || workflow.id}</h2>
                    <p className="mt-2 font-mono text-xs text-slate-500">{workflow.id}</p>
                  </div>
                  <div className="rounded-full border border-cyan-400/20 bg-cyan-400/10 px-3 py-1 text-xs text-cyan-100">
                    {text({ zh: `版本 ${workflow.version}`, en: `Version ${workflow.version}` })}
                  </div>
                </div>
              </article>
            ))}
          </div>
        )}
      </section>
    </div>
  )
}
