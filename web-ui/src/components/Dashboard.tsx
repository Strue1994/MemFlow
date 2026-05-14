import { Link } from 'react-router-dom'
import { useLanguage } from '../lib/language'

export default function Dashboard() {
  const { text } = useLanguage()

  return (
    <div className="mx-auto flex h-full w-full max-w-6xl flex-col gap-6 px-6 py-6">
      <section className="rounded-[28px] border border-white/10 bg-slate-950/70 p-6 shadow-[0_24px_60px_rgba(2,6,23,0.35)]">
        <div className="text-xs uppercase tracking-[0.24em] text-cyan-300">
          {text({ zh: '高级区域', en: 'Advanced' })}
        </div>
        <h1 className="mt-2 text-3xl font-semibold tracking-tight text-white">
          {text({ zh: '高级能力暂存区', en: 'Temporary holding area for advanced tools' })}
        </h1>
        <p className="mt-3 max-w-3xl text-sm leading-6 text-slate-400">
          {text({
            zh: '这里不再承担主首页角色。当前它作为高级功能入口占位，承接编辑器、生成器和实验性能力，避免与任务控制台竞争主入口。',
            en: 'This page no longer serves as the main home. It is a lightweight advanced area for editor, builder, and experimental surfaces.',
          })}
        </p>
      </section>

      <section className="grid gap-4 md:grid-cols-3">
        <Link to="/editor" className="rounded-[24px] border border-white/10 bg-white/[0.03] p-5 transition hover:bg-white/[0.05]">
          <div className="text-sm font-semibold text-white">{text({ zh: '工作流编辑器', en: 'Workflow Editor' })}</div>
          <p className="mt-2 text-sm leading-6 text-slate-400">
            {text({ zh: '手动构建、调试和运行流程。', en: 'Manually build, debug, and run flows.' })}
          </p>
        </Link>
        <Link to="/create" className="rounded-[24px] border border-white/10 bg-white/[0.03] p-5 transition hover:bg-white/[0.05]">
          <div className="text-sm font-semibold text-white">{text({ zh: '工作流构建辅助', en: 'Workflow Builder Support' })}</div>
          <p className="mt-2 text-sm leading-6 text-slate-400">
            {text({ zh: '从自然语言生成草稿，再送入编辑器深化。', en: 'Generate a draft from natural language, then refine it in the editor.' })}
          </p>
        </Link>
        <Link to="/computer" className="rounded-[24px] border border-white/10 bg-white/[0.03] p-5 transition hover:bg-white/[0.05]">
          <div className="text-sm font-semibold text-white">{text({ zh: '电脑代理', en: 'Computer Agent' })}</div>
          <p className="mt-2 text-sm leading-6 text-slate-400">
            {text({ zh: '文件、终端与浏览器操作能力。', en: 'File, terminal, and browser operations.' })}
          </p>
        </Link>
      </section>
    </div>
  )
}
