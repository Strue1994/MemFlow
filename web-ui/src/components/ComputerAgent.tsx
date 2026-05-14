import { type ReactNode, useCallback, useEffect, useState } from 'react'
import { toast } from 'sonner'
import {
  autonomyApi,
  computerApi,
  type AutonomyStatus,
  type CommandResult,
  type ComputerCapabilities,
  type ComputerDirectoryListing,
  type ComputerSearchResults,
} from '../api/client'
import { getStoredApiKey, setStoredApiKey, subscribeToApiKey } from '../lib/apiKey'
import { useLanguage } from '../lib/language'
import {
  Bot,
  BrainCircuit,
  Eye,
  EyeOff,
  FileSearch,
  FolderTree,
  Globe,
  KeyRound,
  Monitor,
  Play,
  RefreshCcw,
  Save,
  Search,
  Square,
  Terminal,
  WandSparkles,
} from 'lucide-react'

function CapabilityPill({ label, active }: { label: string; active: boolean }) {
  return (
    <span
      className={`inline-flex items-center rounded-full px-3 py-1 text-xs font-medium ${
        active
          ? 'bg-emerald-500/15 text-emerald-200 ring-1 ring-emerald-500/25'
          : 'bg-white/6 text-slate-400 ring-1 ring-white/10'
      }`}
    >
      {label}
    </span>
  )
}

function SectionCard({
  title,
  subtitle,
  icon,
  children,
}: {
  title: string
  subtitle: string
  icon: ReactNode
  children: ReactNode
}) {
  return (
    <section className="rounded-[28px] border border-white/10 bg-slate-950/70 p-5 shadow-[0_24px_60px_-30px_rgba(15,23,42,0.95)] backdrop-blur">
      <div className="mb-4 flex items-start justify-between gap-4">
        <div>
          <div className="mb-1 flex items-center gap-2 text-sm font-semibold text-slate-100">
            {icon}
            {title}
          </div>
          <p className="max-w-3xl text-sm leading-6 text-slate-400">{subtitle}</p>
        </div>
      </div>
      {children}
    </section>
  )
}

export default function ComputerAgent() {
  const { text } = useLanguage()
  const [apiKey, setApiKeyState] = useState(getStoredApiKey())
  const [showApiKey, setShowApiKey] = useState(false)
  const [capabilities, setCapabilities] = useState<ComputerCapabilities | null>(null)
  const [autonomy, setAutonomy] = useState<AutonomyStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [command, setCommand] = useState('dir /a')
  const [commandCwd, setCommandCwd] = useState('.')
  const [commandResult, setCommandResult] = useState<CommandResult | null>(null)
  const [browserUrl, setBrowserUrl] = useState('https://example.com')
  const [browserPreview, setBrowserPreview] = useState('')
  const [fsPath, setFsPath] = useState('.')
  const [directory, setDirectory] = useState<ComputerDirectoryListing | null>(null)
  const [filePath, setFilePath] = useState('README.md')
  const [fileContent, setFileContent] = useState('')
  const [searchQuery, setSearchQuery] = useState('settings')
  const [searchResults, setSearchResults] = useState<ComputerSearchResults | null>(null)
  const [searchLoading, setSearchLoading] = useState(false)

  const refresh = useCallback(async () => {
    try {
      setLoading(true)
      const [capabilityData, autonomyData, dirData] = await Promise.all([
        computerApi.capabilities(),
        autonomyApi.status(),
        computerApi.listDirectory(fsPath),
      ])
      setCapabilities(capabilityData)
      setAutonomy(autonomyData)
      setDirectory(dirData)
    } catch (error: any) {
      toast.error(error?.response?.data?.error || error?.message || text({ zh: '加载电脑代理状态失败', en: 'Failed to load computer agent state' }))
    } finally {
      setLoading(false)
    }
  }, [fsPath])

  useEffect(() => {
    refresh()
  }, [refresh])

  useEffect(() => subscribeToApiKey(setApiKeyState), [])

  async function handleAutonomyStart() {
    try {
      const next = await autonomyApi.start('Cover workflow automation and general computer operations safely', 180)
      setAutonomy(next)
      toast.success(text({ zh: '自治循环已启动', en: 'Autonomy loop started' }))
    } catch (error: any) {
      toast.error(error?.response?.data?.error || error?.message || text({ zh: '启动自治循环失败', en: 'Failed to start autonomy loop' }))
    }
  }

  async function handleAutonomyStop() {
    try {
      const next = await autonomyApi.stop()
      setAutonomy(next)
      toast.success(text({ zh: '自治循环已停止', en: 'Autonomy loop stopped' }))
    } catch (error: any) {
      toast.error(error?.response?.data?.error || error?.message || text({ zh: '停止自治循环失败', en: 'Failed to stop autonomy loop' }))
    }
  }

  async function handleTick() {
    try {
      const next = await autonomyApi.tick()
      setAutonomy(next)
      toast.success(text({ zh: '单次自治执行完成', en: 'Autonomy tick completed' }))
    } catch (error: any) {
      toast.error(error?.response?.data?.error || error?.message || text({ zh: '执行单次自治失败', en: 'Failed to run autonomy tick' }))
    }
  }

  async function handleRunCommand(nextCommand = command, nextCwd = commandCwd) {
    try {
      const result = await computerApi.runCommand(nextCommand, nextCwd)
      setCommand(nextCommand)
      setCommandCwd(nextCwd)
      setCommandResult(result)
      if (result.allowed) {
        toast.success(text({ zh: '命令执行完成', en: 'Command finished' }))
      } else {
        toast.warning(text({ zh: '命令被安全模式拦截', en: 'Command blocked by safe mode' }))
      }
    } catch (error: any) {
      toast.error(error?.response?.data?.error || error?.message || text({ zh: '执行命令失败', en: 'Failed to run command' }))
    }
  }

  async function handleBrowseDirectory(nextPath = fsPath) {
    try {
      const result = await computerApi.listDirectory(nextPath)
      setDirectory(result)
      setFsPath(result.path)
    } catch (error: any) {
      toast.error(error?.response?.data?.error || error?.message || text({ zh: '目录读取失败', en: 'Failed to list directory' }))
    }
  }

  async function handleReadFile(nextPath = filePath) {
    try {
      const result = await computerApi.readFile(nextPath)
      setFilePath(result.path)
      setFileContent(result.content)
      toast.success(text({ zh: '文件已读取', en: 'File loaded' }))
    } catch (error: any) {
      toast.error(error?.response?.data?.error || error?.message || text({ zh: '读取文件失败', en: 'Failed to read file' }))
    }
  }

  async function handleWriteFile(append = false) {
    try {
      const result = await computerApi.writeFile(filePath, fileContent, append)
      toast.success(`${text({ zh: append ? '已追加' : '已保存', en: append ? 'Appended to' : 'Saved' })} ${result.path}`)
      await handleBrowseDirectory(pathParent(filePath))
    } catch (error: any) {
      toast.error(error?.response?.data?.error || error?.message || text({ zh: '写入文件失败', en: 'Failed to write file' }))
    }
  }

  async function handleSearch() {
    try {
      setSearchLoading(true)
      const result = await computerApi.searchFiles(searchQuery, fsPath)
      setSearchResults(result)
      if (result.items.length === 0) {
        toast.warning(text({ zh: '没有找到匹配文件', en: 'No matching files found' }))
      } else {
        toast.success(text({ zh: `找到 ${result.items.length} 个结果`, en: `Found ${result.items.length} matches` }))
      }
    } catch (error: any) {
      toast.error(error?.response?.data?.error || error?.message || text({ zh: '搜索文件失败', en: 'Failed to search files' }))
    } finally {
      setSearchLoading(false)
    }
  }

  async function handleBrowserFetch() {
    try {
      const result = await computerApi.fetchUrl(browserUrl)
      setBrowserPreview(`${result.title ? `# ${result.title}\n\n` : ''}${result.bodyPreview}`)
      toast.success(text({ zh: `抓取完成 ${result.status}`, en: `Fetched ${result.status}` }))
    } catch (error: any) {
      toast.error(error?.response?.data?.error || error?.message || text({ zh: '抓取页面失败', en: 'Failed to fetch page' }))
    }
  }

  async function handleBrowserOpen() {
    try {
      await computerApi.openUrl(browserUrl)
      toast.success(text({ zh: '已打开链接', en: 'Opened URL in browser' }))
    } catch (error: any) {
      toast.error(error?.response?.data?.error || error?.message || text({ zh: '打开链接失败', en: 'Failed to open URL' }))
    }
  }

  return (
    <div className="mx-auto max-w-7xl px-4 py-6 sm:px-6 sm:py-8">
      <div className="mb-6 overflow-hidden rounded-[32px] border border-cyan-400/20 bg-[radial-gradient(circle_at_top_left,_rgba(34,211,238,0.16),_transparent_35%),linear-gradient(135deg,rgba(15,23,42,0.98),rgba(2,6,23,0.92))] p-6 shadow-[0_40px_90px_-40px_rgba(34,211,238,0.55)] sm:p-8">
        <div className="flex flex-col gap-6 xl:flex-row xl:items-end xl:justify-between">
          <div className="max-w-3xl">
            <div className="mb-3 inline-flex items-center gap-2 rounded-full border border-cyan-400/25 bg-cyan-400/10 px-3 py-1 text-xs font-medium uppercase tracking-[0.22em] text-cyan-200">
              <Monitor size={14} />
              {text({ zh: '电脑代理', en: 'Computer Agent' })}
            </div>
            <h1 className="max-w-[18ch] text-3xl font-semibold tracking-tight text-white sm:text-4xl">
              {text({ zh: '从工作流自治，扩展到电脑操作台', en: 'Move from workflow autonomy to a computer console' })}
            </h1>
            <p className="mt-4 max-w-2xl text-sm leading-7 text-slate-300">
              {text({
                zh: '这里直接展示系统当前真正能做的事：文件读写、安全命令、页面抓取和自治循环。深度网页点击型自动化还未验证通过，所以会明确标注。',
                en: 'This page exposes the real current surface: file operations, safe terminal commands, page fetch, and autonomy controls. Deep browser-click automation is still unverified and is marked as such.',
              })}
            </p>
          </div>
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
            <MetricTile label={text({ zh: '请求密钥', en: 'API Key' })} value={apiKey ? text({ zh: '已配置', en: 'Configured' }) : text({ zh: '未配置', en: 'Missing' })} />
            <MetricTile label={text({ zh: '自治', en: 'Autonomy' })} value={autonomy?.running ? text({ zh: '运行中', en: 'Running' }) : text({ zh: '空闲', en: 'Idle' })} />
            <MetricTile label={text({ zh: '文件系统', en: 'Filesystem' })} value={capabilities?.filesystem.write ? text({ zh: '可写', en: 'Writable' }) : text({ zh: '不可用', en: 'Unavailable' })} />
            <MetricTile label={text({ zh: '终端', en: 'Terminal' })} value={capabilities?.terminal.safeMode ? text({ zh: '安全模式', en: 'Safe mode' }) : text({ zh: '直连', en: 'Direct' })} />
          </div>
        </div>
      </div>

      <div className="grid gap-4 xl:grid-cols-[1.2fr_0.8fr]">
        <SectionCard
          title={text({ zh: '能力覆盖', en: 'Capability Coverage' })}
          subtitle={text({
            zh: '把当前真正开放出来的电脑操作能力写清楚，避免把工作流自治误判成完整电脑代理。',
            en: 'Show the actual exposed computer capabilities so workflow autonomy is not mistaken for a full computer agent.',
          })}
          icon={<BrainCircuit size={18} className="text-cyan-300" />}
        >
          {loading ? (
            <p className="text-sm text-slate-400">{text({ zh: '加载中…', en: 'Loading capabilities…' })}</p>
          ) : (
            <div className="space-y-4">
              <div className="flex flex-wrap gap-2">
                <CapabilityPill label={text({ zh: '工作流自治', en: 'Workflow Autonomy' })} active />
                <CapabilityPill label={text({ zh: '目录浏览', en: 'Filesystem List' })} active={Boolean(capabilities?.filesystem.list)} />
                <CapabilityPill label={text({ zh: '文件搜索', en: 'Filesystem Search' })} active={Boolean(capabilities?.filesystem.search)} />
                <CapabilityPill label={text({ zh: '文件写入', en: 'Filesystem Write' })} active={Boolean(capabilities?.filesystem.write)} />
                <CapabilityPill label={text({ zh: '终端命令', en: 'Terminal Commands' })} active={Boolean(capabilities?.terminal.run)} />
                <CapabilityPill label={text({ zh: '打开链接', en: 'Browser Open URL' })} active={Boolean(capabilities?.browser.openUrl)} />
                <CapabilityPill label={text({ zh: '抓取页面', en: 'Browser Fetch' })} active={Boolean(capabilities?.browser.fetchPage)} />
                <CapabilityPill label={text({ zh: '网页自动化', en: 'Browser Automation' })} active={Boolean(capabilities?.browser.automation)} />
              </div>
              <div className="grid gap-3 md:grid-cols-3">
                <InfoBox label={text({ zh: '根目录', en: 'Sandbox root' })} value={capabilities?.filesystem.sandboxRoot || '—'} />
                <InfoBox label={text({ zh: '平台', en: 'Platform' })} value={capabilities?.platform || '—'} />
                <InfoBox label={text({ zh: '超时', en: 'Command timeout' })} value={capabilities ? `${capabilities.terminal.timeoutMs} ms` : '—'} />
              </div>
              <div className="rounded-2xl border border-white/10 bg-white/[0.04] px-4 py-3 text-sm leading-6 text-slate-300">
                {capabilities?.browser.automationNote || text({ zh: '暂无网页自动化说明。', en: 'Interactive browser automation note unavailable.' })}
              </div>
            </div>
          )}
        </SectionCard>

        <SectionCard
          title={text({ zh: '请求密钥', en: 'Request Key' })}
          subtitle={text({
            zh: '把请求密钥固定成显式主控件，避免再因为入口太隐蔽而误以为无法设置。',
            en: 'Pin the request key into a primary control so the setting is never hidden in the sidebar.',
          })}
          icon={<KeyRound size={18} className="text-amber-300" />}
        >
          <div className="space-y-3">
            <div className="rounded-2xl border border-white/10 bg-slate-900/70 p-3">
              <div className="mb-2 flex items-center justify-between gap-3">
                <div className="text-[11px] uppercase tracking-[0.18em] text-slate-500">{text({ zh: '当前鉴权', en: 'Current request authorization' })}</div>
                <div
                  className={`rounded-full px-3 py-1 text-[11px] font-medium ${
                    apiKey
                      ? 'bg-emerald-500/15 text-emerald-200 ring-1 ring-emerald-500/25'
                      : 'bg-amber-500/15 text-amber-200 ring-1 ring-amber-500/25'
                  }`}
                >
                  {apiKey ? text({ zh: '已配置', en: 'Configured' }) : text({ zh: '未配置', en: 'Missing' })}
                </div>
              </div>
              <div className="flex gap-2">
                <input
                  type={showApiKey ? 'text' : 'password'}
                  value={apiKey}
                  onChange={(event) => setApiKeyState(event.target.value)}
                  placeholder={text({ zh: '输入当前请求密钥', en: 'Paste active API key' })}
                  className="w-full rounded-2xl border border-white/10 bg-slate-950 px-4 py-3 text-sm text-white placeholder:text-slate-500 focus:border-cyan-400/50 focus:outline-none"
                />
                <button
                  type="button"
                  onClick={() => setShowApiKey((value) => !value)}
                  className="inline-flex items-center justify-center rounded-2xl border border-white/10 px-3 text-slate-300 transition hover:bg-white/[0.05]"
                  aria-label={showApiKey ? text({ zh: '隐藏密钥', en: 'Hide key' }) : text({ zh: '显示密钥', en: 'Show key' })}
                >
                  {showApiKey ? <EyeOff size={16} /> : <Eye size={16} />}
                </button>
              </div>
            </div>
            <div className="flex flex-wrap gap-3">
              <ActionButton
                onClick={() => {
                  setStoredApiKey(apiKey)
                  toast.success(text({ zh: '请求密钥已保存', en: 'Active API key saved' }))
                }}
                icon={<Save size={15} />}
                label={text({ zh: '保存', en: 'Save' })}
                tone="primary"
              />
              <ActionButton
                onClick={() => {
                  setApiKeyState('')
                  setStoredApiKey('')
                  toast.success(text({ zh: '请求密钥已清空', en: 'Active API key cleared' }))
                }}
                icon={<Square size={15} />}
                label={text({ zh: '清空', en: 'Clear' })}
              />
            </div>
            <p className="text-xs leading-6 text-slate-500">
              {text({
                zh: '保存后，这把密钥会自动附加到控制台请求头里，电脑代理和设置页会立刻共用它。',
                en: 'Once saved, this key is attached to console requests immediately and shared by the computer agent and settings pages.',
              })}
            </p>
          </div>
        </SectionCard>
      </div>

      <div className="mt-4 grid gap-4 xl:grid-cols-[0.95fr_1.05fr]">
        <div className="space-y-4">
        <SectionCard
          title={text({ zh: '自治控制', en: 'Autonomy Control' })}
            subtitle={text({
              zh: '观察自治监督器是否真的在看、在想、在行动，而不是只挂着一个状态。',
              en: 'Check whether the autonomy supervisor is actually observing, reflecting, and acting instead of just sitting idle.',
            })}
            icon={<Bot size={18} className="text-emerald-300" />}
          >
            <div className="mb-4 grid gap-3 md:grid-cols-2">
              <InfoBox label={text({ zh: '状态', en: 'Status' })} value={autonomy?.running ? text({ zh: '运行中', en: 'Running' }) : text({ zh: '已停止', en: 'Stopped' })} />
              <InfoBox label={text({ zh: '下次执行', en: 'Next tick' })} value={autonomy?.nextTickAt || '—'} />
              <InfoBox label={text({ zh: '最近动作', en: 'Last action' })} value={autonomy?.lastAction || '—'} />
              <InfoBox label={text({ zh: '最近错误', en: 'Last error' })} value={autonomy?.lastError || '—'} tone={autonomy?.lastError ? 'danger' : 'default'} />
            </div>
            <div className="flex flex-wrap gap-3">
              <ActionButton onClick={handleAutonomyStart} icon={<Play size={15} />} label={text({ zh: '启动', en: 'Start' })} tone="primary" />
              <ActionButton onClick={handleAutonomyStop} icon={<Square size={15} />} label={text({ zh: '停止', en: 'Stop' })} />
              <ActionButton onClick={handleTick} icon={<WandSparkles size={15} />} label={text({ zh: '单次', en: 'Single tick' })} />
              <ActionButton onClick={refresh} icon={<RefreshCcw size={15} />} label={text({ zh: '刷新', en: 'Refresh' })} />
            </div>
            <div className="mt-4 rounded-2xl border border-white/10 bg-slate-900/80 p-4">
              <p className="mb-2 text-xs uppercase tracking-[0.18em] text-slate-500">{text({ zh: '最近反思', en: 'Recent reflections' })}</p>
              <div className="space-y-2">
                {(autonomy?.recent || []).slice(0, 6).map((item) => (
                  <div key={`${item.at}-${item.message}`} className="rounded-2xl border border-white/8 bg-white/[0.03] px-3 py-2">
                    <div className="flex items-center justify-between gap-3">
                      <span className="text-xs font-medium uppercase tracking-[0.16em] text-cyan-200">{item.kind}</span>
                      <span className="text-[11px] text-slate-500">{new Date(item.at).toLocaleString()}</span>
                    </div>
                    <p className="mt-1 break-words text-sm leading-6 text-slate-200">{item.message}</p>
                  </div>
                ))}
                {(!autonomy || autonomy.recent.length === 0) && (
                  <p className="text-sm text-slate-500">{text({ zh: '还没有反思记录。', en: 'No reflections yet.' })}</p>
                )}
              </div>
            </div>
          </SectionCard>

          <SectionCard
            title={text({ zh: '网页动作', en: 'Browser Actions' })}
            subtitle={text({
              zh: '当前支持打开链接和抓取页面内容。深度点击型浏览器自动化还未验证通过，所以这里会明确标注。',
              en: 'This currently supports URL open and page fetch. Deep click-driven browser automation is not yet verified and is marked clearly.',
            })}
            icon={<Globe size={18} className="text-indigo-300" />}
          >
            <div className="space-y-3">
              <input
                value={browserUrl}
                onChange={(event) => setBrowserUrl(event.target.value)}
                placeholder="https://example.com"
                className="w-full rounded-2xl border border-white/10 bg-slate-900 px-4 py-3 text-sm text-white placeholder:text-slate-500 focus:border-cyan-400/50 focus:outline-none"
              />
              <div className="flex flex-wrap gap-3">
                <ActionButton onClick={handleBrowserOpen} icon={<Globe size={15} />} label={text({ zh: '打开', en: 'Open' })} tone="primary" />
                <ActionButton onClick={handleBrowserFetch} icon={<RefreshCcw size={15} />} label={text({ zh: '抓取', en: 'Fetch' })} />
              </div>
              <textarea
                readOnly
                value={browserPreview}
                placeholder={text({ zh: '抓取结果会显示在这里。', en: 'Fetched page preview will appear here.' })}
                className="min-h-[220px] w-full rounded-2xl border border-white/10 bg-slate-950 px-4 py-3 font-mono text-xs leading-6 text-slate-300 placeholder:text-slate-600"
              />
            </div>
          </SectionCard>
        </div>

        <div className="space-y-4">
          <SectionCard
            title={text({ zh: '终端', en: 'Terminal' })}
            subtitle={text({
              zh: '受限命令入口。默认只放行白名单，适合目录查看、构建验证和只读检查。',
              en: 'Restricted command surface. Safe-mode only allows the allowlist for inspection and build checks.',
            })}
            icon={<Terminal size={18} className="text-fuchsia-300" />}
          >
            <div className="mb-3 flex flex-wrap gap-2">
              <QuickChip label={text({ zh: '根目录', en: 'List root' })} onClick={() => handleRunCommand('dir /a', '.')} />
              <QuickChip label={text({ zh: 'Git 状态', en: 'Git status' })} onClick={() => handleRunCommand('git status', '.')} />
              <QuickChip label={text({ zh: '读 README', en: 'Read README' })} onClick={() => handleRunCommand('type README.md', '.')} />
              <QuickChip label={text({ zh: '构建前端', en: 'Build web-ui' })} onClick={() => handleRunCommand('npm run build', 'web-ui')} />
            </div>
            <div className="grid gap-3 sm:grid-cols-[1fr_180px]">
              <input
                value={command}
                onChange={(event) => setCommand(event.target.value)}
                placeholder="dir /a"
                className="w-full rounded-2xl border border-white/10 bg-slate-900 px-4 py-3 text-sm text-white placeholder:text-slate-500 focus:border-cyan-400/50 focus:outline-none"
              />
              <input
                value={commandCwd}
                onChange={(event) => setCommandCwd(event.target.value)}
                placeholder="."
                className="w-full rounded-2xl border border-white/10 bg-slate-900 px-4 py-3 text-sm text-white placeholder:text-slate-500 focus:border-cyan-400/50 focus:outline-none"
              />
            </div>
            <div className="mt-3 flex gap-3">
              <ActionButton onClick={() => handleRunCommand(command, commandCwd)} icon={<Terminal size={15} />} label={text({ zh: '执行', en: 'Run' })} tone="primary" />
            </div>
            <div className="mt-4 grid gap-3 xl:grid-cols-2">
              <TerminalPane title="stdout" tone="success" content={commandResult?.stdout || ''} />
              <TerminalPane title="stderr" tone={commandResult?.stderr ? 'danger' : 'muted'} content={commandResult?.stderr || text({ zh: '没有错误输出', en: 'No stderr' })} />
            </div>
          </SectionCard>

          <SectionCard
            title={text({ zh: '文件系统', en: 'Filesystem' })}
            subtitle={text({
              zh: '先搜索、再浏览、再读写文件，让系统逐步覆盖工作流之外的实际电脑操作。',
              en: 'Search, browse, read, and write files so the system covers real computer work beyond workflows.',
            })}
            icon={<FolderTree size={18} className="text-amber-300" />}
          >
            <div className="space-y-4">
              <div className="grid gap-3 lg:grid-cols-[1fr_auto]">
                <div className="grid gap-3 sm:grid-cols-[1fr_220px]">
                  <input
                    value={searchQuery}
                    onChange={(event) => setSearchQuery(event.target.value)}
                    placeholder={text({ zh: '搜索文件名，例如 settings', en: 'Search file names, e.g. settings' })}
                    className="w-full rounded-2xl border border-white/10 bg-slate-900 px-4 py-3 text-sm text-white placeholder:text-slate-500 focus:border-cyan-400/50 focus:outline-none"
                  />
                  <input
                    value={fsPath}
                    onChange={(event) => setFsPath(event.target.value)}
                    placeholder="."
                    className="w-full rounded-2xl border border-white/10 bg-slate-900 px-4 py-3 text-sm text-white placeholder:text-slate-500 focus:border-cyan-400/50 focus:outline-none"
                  />
                </div>
                <div className="flex gap-3">
                  <ActionButton onClick={handleSearch} icon={<Search size={15} />} label={searchLoading ? text({ zh: '搜索中…', en: 'Searching…' }) : text({ zh: '搜索', en: 'Search' })} tone="primary" />
                  <ActionButton onClick={() => handleBrowseDirectory(fsPath)} icon={<RefreshCcw size={15} />} label={text({ zh: '浏览', en: 'Browse' })} />
                </div>
              </div>

              <div className="grid gap-3 lg:grid-cols-[0.95fr_1.05fr]">
                <div className="rounded-2xl border border-white/10 bg-slate-950/85 p-3">
                  <div className="mb-3 flex items-center gap-2 text-xs uppercase tracking-[0.18em] text-slate-500">
                    <FileSearch size={14} />
                    {text({ zh: '搜索结果', en: 'Search results' })}
                  </div>
                  <div className="space-y-2">
                    {(searchResults?.items || []).map((item) => (
                      <button
                        key={`search-${item.path}`}
                        type="button"
                        onClick={() => {
                          if (item.kind === 'directory') {
                            void handleBrowseDirectory(item.path)
                            return
                          }
                          void handleReadFile(item.path)
                        }}
                        className="w-full rounded-2xl border border-white/8 bg-white/[0.03] px-3 py-3 text-left transition hover:bg-white/[0.06]"
                      >
                        <div className="flex items-center justify-between gap-3">
                          <span className="text-xs uppercase tracking-[0.16em] text-cyan-200">{item.kind}</span>
                          <span className="text-[11px] text-slate-500">{new Date(item.modifiedAt).toLocaleString()}</span>
                        </div>
                        <div className="mt-2 break-all font-mono text-xs text-slate-100">{item.path}</div>
                      </button>
                    ))}
                    {searchResults?.truncated && (
                      <div className="rounded-2xl border border-amber-400/15 bg-amber-400/10 px-3 py-2 text-xs text-amber-200">
                        {text({ zh: '结果过多，请缩小搜索范围。', en: 'Search result limit reached. Narrow the query.' })}
                      </div>
                    )}
                    {(!searchResults || searchResults.items.length === 0) && (
                      <div className="rounded-2xl border border-dashed border-white/10 bg-white/[0.02] px-4 py-8 text-center text-sm text-slate-500">
                        {text({ zh: '先搜索文件名，再点击结果打开或进入目录。', en: 'Search for a filename first, then click a result to open or browse it.' })}
                      </div>
                    )}
                  </div>
                </div>

                <div className="rounded-2xl border border-white/10 bg-slate-950/85">
                  <div className="border-b border-white/10 px-4 py-3 text-xs uppercase tracking-[0.18em] text-slate-500">
                    {text({ zh: '目录列表', en: 'Directory listing' })}
                  </div>
                  <div className="max-h-72 overflow-auto">
                    <table className="w-full text-sm">
                      <thead className="sticky top-0 bg-slate-950/95 text-left text-[11px] uppercase tracking-[0.18em] text-slate-500">
                        <tr>
                          <th className="px-4 py-3">{text({ zh: '名称', en: 'Name' })}</th>
                          <th className="px-4 py-3">{text({ zh: '类型', en: 'Type' })}</th>
                          <th className="px-4 py-3">{text({ zh: '修改时间', en: 'Modified' })}</th>
                        </tr>
                      </thead>
                      <tbody className="divide-y divide-white/5">
                        {(directory?.items || []).map((item) => (
                          <tr
                            key={item.path}
                            className="cursor-pointer hover:bg-white/[0.04]"
                            onClick={() => {
                              if (item.kind === 'directory') {
                                void handleBrowseDirectory(item.path)
                                return
                              }
                              void handleReadFile(item.path)
                            }}
                          >
                            <td className="px-4 py-3 font-mono text-xs text-slate-200">{item.name}</td>
                            <td className="px-4 py-3 text-xs text-slate-500">{item.kind}</td>
                            <td className="px-4 py-3 text-xs text-slate-500">{new Date(item.modifiedAt).toLocaleString()}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
              </div>

              <div className="grid gap-3">
                <input
                  value={filePath}
                  onChange={(event) => setFilePath(event.target.value)}
                  placeholder="README.md"
                  className="w-full rounded-2xl border border-white/10 bg-slate-900 px-4 py-3 text-sm text-white placeholder:text-slate-500 focus:border-cyan-400/50 focus:outline-none"
                />
                <textarea
                  value={fileContent}
                  onChange={(event) => setFileContent(event.target.value)}
                  placeholder={text({ zh: '文件内容', en: 'File content' })}
                  className="min-h-[220px] w-full rounded-2xl border border-white/10 bg-slate-950 px-4 py-3 font-mono text-xs leading-6 text-slate-200 placeholder:text-slate-600 focus:border-cyan-400/50 focus:outline-none"
                />
                <div className="flex flex-wrap gap-3">
                  <ActionButton onClick={() => handleReadFile(filePath)} icon={<FolderTree size={15} />} label={text({ zh: '读取', en: 'Read' })} />
                  <ActionButton onClick={() => handleWriteFile(false)} icon={<Save size={15} />} label={text({ zh: '覆盖', en: 'Overwrite' })} tone="primary" />
                  <ActionButton onClick={() => handleWriteFile(true)} icon={<Save size={15} />} label={text({ zh: '追加', en: 'Append' })} />
                </div>
              </div>
            </div>
          </SectionCard>
        </div>
      </div>
    </div>
  )
}

function pathParent(value: string): string {
  const parts = value.split(/[\\/]/).filter(Boolean)
  if (parts.length <= 1) {
    return '.'
  }
  return parts.slice(0, -1).join('/')
}

function MetricTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-2xl border border-white/10 bg-white/6 px-4 py-3">
      <div className="text-[11px] uppercase tracking-[0.18em] text-slate-400">{label}</div>
      <div className="mt-2 break-words text-sm font-medium text-white">{value}</div>
    </div>
  )
}

function InfoBox({
  label,
  value,
  tone = 'default',
}: {
  label: string
  value: string
  tone?: 'default' | 'danger'
}) {
  return (
    <div className="rounded-2xl border border-white/10 bg-white/[0.04] px-4 py-3">
      <div className="text-[11px] uppercase tracking-[0.18em] text-slate-500">{label}</div>
      <div
        className={`mt-2 break-words text-sm font-medium leading-6 ${
          tone === 'danger' ? 'text-rose-300' : 'text-slate-100'
        }`}
      >
        {value}
      </div>
    </div>
  )
}

function ActionButton({
  onClick,
  icon,
  label,
  tone = 'secondary',
}: {
  onClick: () => void
  icon: ReactNode
  label: string
  tone?: 'primary' | 'secondary'
}) {
  return (
    <button
      onClick={onClick}
      className={`inline-flex items-center gap-2 rounded-2xl px-4 py-2.5 text-sm font-medium transition ${
        tone === 'primary'
          ? 'bg-cyan-400 text-slate-950 hover:bg-cyan-300'
          : 'border border-white/10 bg-white/[0.04] text-slate-200 hover:bg-white/[0.08]'
      }`}
    >
      {icon}
      {label}
    </button>
  )
}

function QuickChip({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="rounded-full border border-white/10 bg-white/[0.04] px-3 py-2 text-xs text-slate-300 transition hover:bg-white/[0.08]"
    >
      {label}
    </button>
  )
}

function TerminalPane({
  title,
  content,
  tone,
}: {
  title: string
  content: string
  tone: 'success' | 'danger' | 'muted'
}) {
  const color =
    tone === 'success' ? 'text-emerald-200' : tone === 'danger' ? 'text-rose-200' : 'text-slate-300'
  return (
    <div className="rounded-2xl border border-white/10 bg-slate-950/90">
      <div className="border-b border-white/10 px-4 py-2 text-[11px] uppercase tracking-[0.18em] text-slate-500">{title}</div>
      <pre className={`max-h-72 overflow-auto whitespace-pre-wrap break-all px-4 py-3 font-mono text-xs leading-6 ${color}`}>
        {content || 'No output'}
      </pre>
    </div>
  )
}
