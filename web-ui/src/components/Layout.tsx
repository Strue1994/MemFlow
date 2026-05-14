import { type ReactNode, useEffect, useMemo, useState } from 'react'
import { NavLink, useLocation, useNavigate } from 'react-router-dom'
import {
  Bot,
  BrainCircuit,
  ClipboardList,
  FolderKanban,
  Languages,
  Settings as SettingsIcon,
  SlidersHorizontal,
  History,
} from 'lucide-react'
import { getStoredApiKey, subscribeToApiKey } from '../lib/apiKey'
import { useLanguage } from '../lib/language'

interface NavItemProps {
  to: string
  icon: React.ReactNode
  title: string
  subtitle: string
  matchPaths?: string[]
}

function matchesNavPath(pathname: string, to: string, matchPaths: string[] = []) {
  if (pathname === to) {
    return true
  }

  return matchPaths.some((path) => pathname === path || pathname.startsWith(`${path}/`))
}

function NavItem({ to, icon, title, subtitle, matchPaths }: NavItemProps) {
  const { pathname } = useLocation()
  const isActive = matchesNavPath(pathname, to, matchPaths)

  return (
    <NavLink
      to={to}
      className={() =>
        `group flex items-center gap-3 rounded-2xl border px-3 py-3 transition ${
          isActive
            ? 'border-cyan-400/25 bg-cyan-400/10 text-white shadow-[0_0_0_1px_rgba(34,211,238,0.12)]'
            : 'border-white/6 bg-white/[0.03] text-slate-300 hover:border-white/12 hover:bg-white/[0.06]'
        }`
      }
    >
      <div className="flex h-10 w-10 items-center justify-center rounded-2xl bg-slate-900/80 text-cyan-200 ring-1 ring-white/8">
        {icon}
      </div>
      <div className="min-w-0">
        <div className="text-sm font-medium">{title}</div>
        <div className="text-xs leading-5 text-slate-500">{subtitle}</div>
      </div>
    </NavLink>
  )
}

function MobileNavPill({ to, title, matchPaths }: { to: string; title: string; matchPaths?: string[] }) {
  const { pathname } = useLocation()
  const isActive = matchesNavPath(pathname, to, matchPaths)

  return (
    <NavLink
      to={to}
      className={() =>
        `whitespace-nowrap rounded-full border px-3 py-2 text-xs font-medium transition ${
          isActive
            ? 'border-cyan-400/30 bg-cyan-400/12 text-cyan-100'
            : 'border-white/10 bg-white/[0.03] text-slate-300'
        }`
      }
    >
      {title}
    </NavLink>
  )
}

function LanguageSwitch() {
  const { language, setLanguage } = useLanguage()

  return (
    <div className="inline-flex items-center rounded-2xl border border-white/10 bg-slate-950/75 p-1">
      <button
        type="button"
        onClick={() => setLanguage('zh')}
        className={`rounded-xl px-3 py-2 text-xs font-medium transition ${
          language === 'zh' ? 'bg-cyan-400 text-slate-950' : 'text-slate-300'
        }`}
      >
        中文
      </button>
      <button
        type="button"
        onClick={() => setLanguage('en')}
        className={`rounded-xl px-3 py-2 text-xs font-medium transition ${
          language === 'en' ? 'bg-cyan-400 text-slate-950' : 'text-slate-300'
        }`}
      >
        EN
      </button>
    </div>
  )
}

export default function Layout({ children }: { children: ReactNode }) {
  const navigate = useNavigate()
  const { text } = useLanguage()
  const [apiKey, setApiKey] = useState(getStoredApiKey())

  useEffect(() => subscribeToApiKey(setApiKey), [])

  const navItems = useMemo(
    () => [
      {
        to: '/tasks',
        icon: <ClipboardList size={18} />,
        title: text({ zh: '任务控制台', en: 'Task Console' }),
        subtitle: text({ zh: '自然语言任务入口', en: 'Natural-language task entry' }),
      },
      {
        to: '/assets',
        icon: <FolderKanban size={18} />,
        title: text({ zh: '工作流资产', en: 'Workflow Assets' }),
        subtitle: text({ zh: '已登记可复用流程', en: 'Registered reusable flows' }),
      },
      {
        to: '/history',
        icon: <History size={18} />,
        title: text({ zh: '执行历史', en: 'Execution History' }),
        subtitle: text({ zh: '任务与路由轨迹', en: 'Task and routing traces' }),
      },
      {
        to: '/settings',
        icon: <SettingsIcon size={18} />,
        title: text({ zh: '设置', en: 'Settings' }),
        subtitle: text({ zh: '密钥、模型与系统', en: 'Keys, models, system' }),
      },
      {
        to: '/advanced',
        icon: <SlidersHorizontal size={18} />,
        title: text({ zh: '高级', en: 'Advanced' }),
        subtitle: text({ zh: '编辑器、生成器与实验能力', en: 'Editor, builder, experimental tools' }),
        matchPaths: ['/editor', '/create', '/computer', '/marketplace', '/timeline'],
      },
    ],
    [text],
  )

  return (
    <div className="min-h-screen bg-[radial-gradient(circle_at_top_left,_rgba(34,211,238,0.08),_transparent_28%),radial-gradient(circle_at_bottom_right,_rgba(14,165,233,0.1),_transparent_30%),linear-gradient(180deg,#020617,#0f172a)] text-slate-100">
      <div className="mx-auto flex min-h-screen max-w-[1800px]">
        <aside className="hidden w-[320px] flex-shrink-0 border-r border-white/8 bg-slate-950/75 px-5 py-6 backdrop-blur xl:flex xl:flex-col">
          <div className="mb-6 rounded-[28px] border border-cyan-400/15 bg-[linear-gradient(135deg,rgba(34,211,238,0.12),rgba(15,23,42,0.82))] p-5">
            <div className="mb-4 inline-flex items-center gap-2 rounded-full border border-cyan-400/20 bg-cyan-400/10 px-3 py-1 text-[11px] uppercase tracking-[0.24em] text-cyan-200">
              <BrainCircuit size={14} />
              MemFlow
            </div>
            <div className="max-w-[12ch] text-2xl font-semibold tracking-tight text-white">
              {text({ zh: '统一控制台', en: 'Unified Console' })}
            </div>
            <div className="mt-4 flex items-center gap-2 text-xs text-slate-300">
              <Languages size={14} className="text-cyan-200" />
              <LanguageSwitch />
            </div>
          </div>

          <div className="mb-4 rounded-[24px] border border-white/8 bg-white/[0.03] p-4">
            <div className="flex items-center justify-between gap-3">
              <div>
                <div className="text-[11px] uppercase tracking-[0.2em] text-slate-500">
                  {text({ zh: '请求密钥', en: 'Request key' })}
                </div>
                <div className="mt-2 text-sm font-medium text-white">
                  {apiKey ? text({ zh: '已配置', en: 'Configured' }) : text({ zh: '未配置', en: 'Missing' })}
                </div>
              </div>
              <div
                className={`rounded-full px-3 py-1 text-xs font-medium ${
                  apiKey ? 'bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/25' : 'bg-rose-500/10 text-rose-300 ring-1 ring-rose-500/25'
                }`}
              >
                {apiKey ? text({ zh: '可用', en: 'Ready' }) : text({ zh: '待配置', en: 'Action' })}
              </div>
            </div>
            <p className="mt-3 text-xs leading-6 text-slate-500">
              {text({
                zh: '控制台与电脑代理共用这把请求密钥。入口已经统一到设置页。',
                en: 'The console and computer agent share this request key. Manage it from Settings.',
              })}
            </p>
            <button
              onClick={() => navigate('/settings')}
              className="mt-4 inline-flex items-center gap-2 rounded-2xl border border-white/10 px-3 py-2 text-sm text-slate-200 transition hover:bg-white/[0.06]"
            >
              <SettingsIcon size={15} />
              {text({ zh: '进入设置', en: 'Open Settings' })}
            </button>
          </div>

          <div className="space-y-2">
            {navItems.map((item) => (
              <NavItem
                key={item.to}
                to={item.to}
                icon={item.icon}
                title={item.title}
                subtitle={item.subtitle}
                matchPaths={item.matchPaths}
              />
            ))}
          </div>

          <div className="mt-auto rounded-[24px] border border-white/8 bg-white/[0.03] p-4">
            <div className="mb-3 flex items-center gap-2 text-sm font-medium text-white">
              <Bot size={16} className="text-cyan-200" />
              {text({ zh: '当前覆盖', en: 'Coverage' })}
            </div>
            <div className="flex flex-wrap gap-2 text-xs">
              <span className="rounded-full bg-cyan-400/10 px-3 py-1 text-cyan-200 ring-1 ring-cyan-400/15">
                {text({ zh: '工作流', en: 'Workflow' })}
              </span>
              <span className="rounded-full bg-cyan-400/10 px-3 py-1 text-cyan-200 ring-1 ring-cyan-400/15">
                {text({ zh: '文件系统', en: 'Filesystem' })}
              </span>
              <span className="rounded-full bg-cyan-400/10 px-3 py-1 text-cyan-200 ring-1 ring-cyan-400/15">
                {text({ zh: '终端', en: 'Terminal' })}
              </span>
              <span className="rounded-full bg-white/6 px-3 py-1 text-slate-400 ring-1 ring-white/10">
                {text({ zh: '网页自动化待接通', en: 'Browser automation pending' })}
              </span>
            </div>
          </div>
        </aside>

        <div className="flex min-w-0 flex-1 flex-col">
          <header className="sticky top-0 z-20 border-b border-white/8 bg-slate-950/70 px-4 py-3 backdrop-blur md:px-6 xl:hidden">
            <div className="flex items-center justify-between gap-3">
              <div>
                <div className="text-xs uppercase tracking-[0.22em] text-cyan-300">MemFlow</div>
                <div className="text-sm font-medium text-white">{text({ zh: '控制台', en: 'Console' })}</div>
              </div>
              <div className="flex items-center gap-2">
                <LanguageSwitch />
                <button
                  onClick={() => navigate('/settings')}
                  className="rounded-2xl border border-white/10 px-3 py-2 text-sm text-slate-200"
                >
                  {text({ zh: '设置', en: 'Settings' })}
                </button>
              </div>
            </div>
            <div className="-mx-1 mt-3 flex gap-2 overflow-x-auto px-1 pb-1">
              {navItems.map((item) => (
                <MobileNavPill key={item.to} to={item.to} title={item.title} matchPaths={item.matchPaths} />
              ))}
            </div>
          </header>
          <main className="min-w-0 flex-1 overflow-auto">{children}</main>
        </div>
      </div>
    </div>
  )
}
