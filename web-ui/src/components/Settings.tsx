import { type ReactNode, useEffect, useMemo, useState } from 'react'
import { toast } from 'sonner'
import { llmSettingsApi, type LLMProviderPreset, type LLMSettings, workflowApi } from '../api/client'
import { getStoredApiKey, setStoredApiKey, subscribeToApiKey } from '../lib/apiKey'
import { useLanguage } from '../lib/language'
import { Copy, Eye, EyeOff, KeyRound, Languages, Plus, Save, ShieldCheck, Trash2, WandSparkles } from 'lucide-react'

interface ApiKeyEntry {
  name: string
  role: string
  created_at: string
}

function Card({
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
    <section className="rounded-3xl border border-white/10 bg-slate-950/70 p-5 shadow-[0_24px_60px_-30px_rgba(15,23,42,0.95)] backdrop-blur">
      <div className="mb-4 flex items-start gap-3">
        <div className="flex h-10 w-10 items-center justify-center rounded-2xl bg-cyan-400/10 text-cyan-200 ring-1 ring-cyan-400/20">
          {icon}
        </div>
        <div>
          <h2 className="text-base font-semibold text-white">{title}</h2>
          <p className="mt-1 text-sm leading-6 text-slate-400">{subtitle}</p>
        </div>
      </div>
      {children}
    </section>
  )
}

function LocaleSwitch() {
  const { language, setLanguage } = useLanguage()
  return (
    <div className="inline-flex rounded-2xl border border-white/10 bg-slate-950/75 p-1">
      <button
        type="button"
        onClick={() => setLanguage('zh')}
        className={`rounded-xl px-3 py-2 text-xs font-medium transition ${language === 'zh' ? 'bg-cyan-400 text-slate-950' : 'text-slate-300'}`}
      >
        中文
      </button>
      <button
        type="button"
        onClick={() => setLanguage('en')}
        className={`rounded-xl px-3 py-2 text-xs font-medium transition ${language === 'en' ? 'bg-cyan-400 text-slate-950' : 'text-slate-300'}`}
      >
        EN
      </button>
    </div>
  )
}

export default function Settings() {
  const { language, text } = useLanguage()
  const [keys, setKeys] = useState<ApiKeyEntry[]>([])
  const [loading, setLoading] = useState(false)
  const [showForm, setShowForm] = useState(false)
  const [newKeyName, setNewKeyName] = useState('')
  const [newKeyRole, setNewKeyRole] = useState('Viewer')
  const [newKeyLimit, setNewKeyLimit] = useState(60)
  const [createdKey, setCreatedKey] = useState<string | null>(null)
  const [showCreatedKey, setShowCreatedKey] = useState(false)
  const [storedApiKey, setStoredApiKeyState] = useState(getStoredApiKey())
  const [showStoredApiKey, setShowStoredApiKey] = useState(false)
  const [showLlmApiKey, setShowLlmApiKey] = useState(false)
  const [llmLoading, setLlmLoading] = useState(false)
  const [llmSaving, setLlmSaving] = useState(false)
  const [llmTesting, setLlmTesting] = useState(false)
  const [providerCatalog, setProviderCatalog] = useState<LLMProviderPreset[]>([])
  const [llmTestResult, setLlmTestResult] = useState<{ tone: 'healthy' | 'critical'; message: string } | null>(null)
  const [llmSettings, setLlmSettings] = useState<LLMSettings>({
    provider: 'openai',
    apiKey: '',
    baseUrl: '',
    model: 'gpt-4o-mini',
    updatedAt: null,
  })

  useEffect(() => {
    void fetchKeys()
    void fetchLLMSettings()
  }, [])

  useEffect(() => subscribeToApiKey(setStoredApiKeyState), [])

  const selectedPreset = useMemo(
    () => providerCatalog.find((provider) => provider.id === llmSettings.provider) || null,
    [providerCatalog, llmSettings.provider],
  )

  async function fetchKeys() {
    try {
      setLoading(true)
      const data = await workflowApi.listApiKeys()
      setKeys(data)
    } catch {
      toast.error(text({ zh: '加载服务端密钥失败', en: 'Failed to load service API keys' }))
    } finally {
      setLoading(false)
    }
  }

  async function fetchLLMSettings() {
    try {
      setLlmLoading(true)
      const [catalog, settings] = await Promise.all([llmSettingsApi.getCatalog(), llmSettingsApi.get()])
      setProviderCatalog(catalog.providers)
      setLlmSettings(settings)
    } catch {
      toast.error(text({ zh: '加载模型设置失败', en: 'Failed to load LLM settings' }))
    } finally {
      setLlmLoading(false)
    }
  }

  function applyPreset(providerId: LLMSettings['provider']) {
    const preset = providerCatalog.find((provider) => provider.id === providerId)
    if (!preset) {
      return
    }
    setLlmSettings((current) => ({
      ...current,
      provider: providerId,
      baseUrl: current.provider === providerId && current.baseUrl ? current.baseUrl : preset.defaultBaseUrl,
      model: current.provider === providerId && current.model ? current.model : preset.defaultModel,
    }))
  }

  async function handleCreate(event: React.FormEvent) {
    event.preventDefault()
    if (!newKeyName.trim()) {
      toast.warning(text({ zh: '请输入密钥名称', en: 'Key name is required' }))
      return
    }
    try {
      const res = await workflowApi.createApiKey(newKeyName.trim(), newKeyRole, newKeyLimit)
      setCreatedKey(res.key)
      setShowCreatedKey(true)
      setNewKeyName('')
      setShowForm(false)
      toast.success(text({ zh: '服务端密钥已创建', en: 'Service API key created' }))
      void fetchKeys()
    } catch {
      toast.error(text({ zh: '创建服务端密钥失败', en: 'Failed to create service API key' }))
    }
  }

  async function handleDelete(key: string) {
    if (!confirm(text({ zh: `确认删除密钥 “${key}”？`, en: `Delete key "${key}"?` }))) return
    try {
      await workflowApi.deleteApiKey(key)
      toast.success(text({ zh: '服务端密钥已删除', en: 'Service API key deleted' }))
      void fetchKeys()
    } catch {
      toast.error(text({ zh: '删除服务端密钥失败', en: 'Failed to delete service API key' }))
    }
  }

  function copyToClipboard(textValue: string) {
    navigator.clipboard.writeText(textValue).then(() => toast.success(text({ zh: '已复制', en: 'Copied' })))
  }

  function saveActiveApiKey() {
    setStoredApiKey(storedApiKey)
    toast.success(text({ zh: '请求密钥已保存', en: 'Request key saved' }))
  }

  async function saveLLMSettings() {
    try {
      setLlmSaving(true)
      const saved = await llmSettingsApi.save(llmSettings)
      setLlmSettings(saved)
      toast.success(text({ zh: '模型设置已保存', en: 'LLM settings saved' }))
    } catch {
      toast.error(text({ zh: '保存模型设置失败', en: 'Failed to save LLM settings' }))
    } finally {
      setLlmSaving(false)
    }
  }

  async function testLLMSettings() {
    try {
      setLlmTesting(true)
      const result = await llmSettingsApi.test({
        provider: llmSettings.provider,
        apiKey: llmSettings.apiKey,
        baseUrl: llmSettings.baseUrl,
        model: llmSettings.model,
      })
      setLlmTestResult({
        tone: 'healthy',
        message: `${result.provider} / ${result.model}: ${result.content}`,
      })
      toast.success(text({ zh: '模型连接测试成功', en: 'LLM connection test succeeded' }))
    } catch (error: any) {
      const message = error?.response?.data?.error || error?.message || text({ zh: '模型连接测试失败', en: 'LLM connection test failed' })
      setLlmTestResult({ tone: 'critical', message })
      toast.error(message)
    } finally {
      setLlmTesting(false)
    }
  }

  return (
    <div className="mx-auto max-w-7xl px-4 py-6 sm:px-6 sm:py-8">
      <div className="mb-6 rounded-[32px] border border-cyan-400/20 bg-[radial-gradient(circle_at_top_left,_rgba(34,211,238,0.16),_transparent_35%),linear-gradient(135deg,rgba(15,23,42,0.98),rgba(2,6,23,0.92))] p-6 shadow-[0_40px_90px_-40px_rgba(34,211,238,0.55)] sm:p-8">
        <div className="flex flex-col gap-5 xl:flex-row xl:items-end xl:justify-between">
          <div>
            <div className="mb-3 inline-flex items-center gap-2 rounded-full border border-cyan-400/20 bg-cyan-400/10 px-3 py-1 text-[11px] uppercase tracking-[0.22em] text-cyan-200">
              <ShieldCheck size={14} />
              {text({ zh: '设置', en: 'Settings' })}
            </div>
            <h1 className="text-3xl font-semibold tracking-tight text-white">
              {text({ zh: '把模型、密钥和界面控制放到一个地方', en: 'Keep providers, keys, and UI controls together' })}
            </h1>
            <p className="mt-3 max-w-3xl text-sm leading-7 text-slate-300">
              {text({
                zh: '这里同时管理请求密钥、LLM Provider、模型连接测试、语言偏好和服务端 API key，不再把关键配置拆散在多个角落。',
                en: 'Manage request keys, LLM providers, model connection tests, language preferences, and service API keys from one page.',
              })}
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <div className="rounded-2xl border border-white/10 bg-white/6 px-4 py-3">
              <div className="text-[11px] uppercase tracking-[0.18em] text-slate-400">
                {text({ zh: '当前语言', en: 'Language' })}
              </div>
              <div className="mt-2">
                <LocaleSwitch />
              </div>
            </div>
            <div className="rounded-2xl border border-white/10 bg-white/6 px-4 py-3">
              <div className="text-[11px] uppercase tracking-[0.18em] text-slate-400">
                {text({ zh: '当前 Provider', en: 'Provider' })}
              </div>
              <div className="mt-2 text-sm font-medium text-white">
                {selectedPreset ? (language === 'zh' ? selectedPreset.labelZh : selectedPreset.label) : '—'}
              </div>
            </div>
          </div>
        </div>
      </div>

      <div className="grid gap-4 xl:grid-cols-[0.95fr_1.05fr]">
        <div className="space-y-4">
          <Card
            title={text({ zh: '系统与请求', en: 'System and Request' })}
            subtitle={text({
              zh: '先把界面语言和控制台请求密钥固定好，其他页面都会跟着这套设置走。',
              en: 'Set the console language and request key first. The rest of the UI will follow this state.',
            })}
            icon={<Languages size={18} />}
          >
            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-2xl border border-white/10 bg-slate-900/70 p-4">
                <div className="mb-3 text-[11px] uppercase tracking-[0.18em] text-slate-500">
                  {text({ zh: '界面语言', en: 'Interface language' })}
                </div>
                <LocaleSwitch />
              </div>

              <div className="rounded-2xl border border-white/10 bg-slate-900/70 p-4">
                <div className="mb-2 flex items-center justify-between gap-3">
                  <div className="text-[11px] uppercase tracking-[0.18em] text-slate-500">
                    {text({ zh: '请求密钥', en: 'Request key' })}
                  </div>
                  <div
                    className={`rounded-full px-3 py-1 text-[11px] font-medium ${
                      storedApiKey
                        ? 'bg-emerald-500/15 text-emerald-200 ring-1 ring-emerald-500/25'
                        : 'bg-amber-500/15 text-amber-200 ring-1 ring-amber-500/25'
                    }`}
                  >
                    {storedApiKey ? text({ zh: '已配置', en: 'Configured' }) : text({ zh: '未配置', en: 'Missing' })}
                  </div>
                </div>
                <div className="flex gap-2">
                  <input
                    type={showStoredApiKey ? 'text' : 'password'}
                    className="w-full rounded-2xl border border-white/10 bg-slate-950 px-4 py-3 text-sm text-white placeholder:text-slate-500 focus:border-cyan-400/50 focus:outline-none"
                    placeholder={text({ zh: '输入控制台请求密钥', en: 'Paste console request key' })}
                    value={storedApiKey}
                    onChange={(event) => setStoredApiKeyState(event.target.value)}
                  />
                  <button
                    type="button"
                    onClick={() => setShowStoredApiKey((value) => !value)}
                    className="inline-flex items-center justify-center rounded-2xl border border-white/10 px-3 text-slate-300 transition hover:bg-white/[0.05]"
                  >
                    {showStoredApiKey ? <EyeOff size={16} /> : <Eye size={16} />}
                  </button>
                </div>
                <div className="mt-3 flex flex-wrap gap-3">
                  <button
                    onClick={saveActiveApiKey}
                    className="inline-flex items-center gap-2 rounded-2xl bg-cyan-400 px-4 py-2.5 text-sm font-medium text-slate-950 transition hover:bg-cyan-300"
                  >
                    <Save size={15} />
                    {text({ zh: '保存', en: 'Save' })}
                  </button>
                  <button
                    onClick={() => {
                      setStoredApiKeyState('')
                      setStoredApiKey('')
                      toast.success(text({ zh: '请求密钥已清空', en: 'Request key cleared' }))
                    }}
                    className="rounded-2xl border border-white/10 px-4 py-2.5 text-sm text-slate-300 transition hover:bg-white/[0.05]"
                  >
                    {text({ zh: '清空', en: 'Clear' })}
                  </button>
                </div>
              </div>
            </div>
          </Card>

          <Card
            title={text({ zh: '服务端 API 密钥', en: 'Service API Keys' })}
            subtitle={text({
              zh: '这部分是 executor 侧的服务端访问密钥。和上面的控制台请求密钥不同。',
              en: 'These are executor-side service API keys. They are different from the console request key above.',
            })}
            icon={<KeyRound size={18} />}
          >
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div className="text-sm text-slate-300">
                  {keys.length} {text({ zh: '个密钥', en: 'keys' })}
                </div>
                <button
                  onClick={() => setShowForm((value) => !value)}
                  className="inline-flex items-center gap-2 rounded-2xl border border-white/10 px-3 py-2 text-sm text-slate-200 transition hover:bg-white/[0.05]"
                >
                  <Plus size={15} />
                  {text({ zh: '新建', en: 'New' })}
                </button>
              </div>

              {showForm && (
                <form onSubmit={handleCreate} className="rounded-2xl border border-white/10 bg-slate-900/70 p-4">
                  <div className="grid gap-3 md:grid-cols-3">
                    <input
                      type="text"
                      className="rounded-2xl border border-white/10 bg-slate-950 px-4 py-3 text-sm text-white placeholder:text-slate-500 focus:border-cyan-400/50 focus:outline-none"
                      placeholder={text({ zh: '密钥名称', en: 'Key name' })}
                      value={newKeyName}
                      onChange={(event) => setNewKeyName(event.target.value)}
                      required
                    />
                    <select
                      className="rounded-2xl border border-white/10 bg-slate-950 px-4 py-3 text-sm text-white focus:border-cyan-400/50 focus:outline-none"
                      value={newKeyRole}
                      onChange={(event) => setNewKeyRole(event.target.value)}
                    >
                      <option value="Viewer">Viewer</option>
                      <option value="Editor">Editor</option>
                      <option value="Admin">Admin</option>
                    </select>
                    <input
                      type="number"
                      min={1}
                      max={10000}
                      className="rounded-2xl border border-white/10 bg-slate-950 px-4 py-3 text-sm text-white placeholder:text-slate-500 focus:border-cyan-400/50 focus:outline-none"
                      value={newKeyLimit}
                      onChange={(event) => setNewKeyLimit(Number(event.target.value))}
                    />
                  </div>
                  <div className="mt-3 flex justify-end gap-3">
                    <button
                      type="button"
                      onClick={() => setShowForm(false)}
                      className="rounded-2xl border border-white/10 px-4 py-2.5 text-sm text-slate-300 transition hover:bg-white/[0.05]"
                    >
                      {text({ zh: '取消', en: 'Cancel' })}
                    </button>
                    <button
                      type="submit"
                      className="inline-flex items-center gap-2 rounded-2xl bg-cyan-400 px-4 py-2.5 text-sm font-medium text-slate-950 transition hover:bg-cyan-300"
                    >
                      <Plus size={15} />
                      {text({ zh: '创建', en: 'Create' })}
                    </button>
                  </div>
                </form>
              )}

              {createdKey && (
                <div className="rounded-2xl border border-emerald-400/20 bg-emerald-400/10 p-4">
                  <div className="mb-2 text-sm font-medium text-emerald-200">{text({ zh: '新密钥', en: 'New key' })}</div>
                  <div className="flex items-center gap-2 rounded-2xl border border-white/10 bg-slate-950/80 px-3 py-3">
                    <code className="flex-1 break-all text-xs text-slate-200">
                      {showCreatedKey ? createdKey : '•'.repeat(Math.min(createdKey.length, 40))}
                    </code>
                    <button onClick={() => setShowCreatedKey((value) => !value)} className="text-slate-400 hover:text-slate-200">
                      {showCreatedKey ? <EyeOff size={16} /> : <Eye size={16} />}
                    </button>
                    <button onClick={() => copyToClipboard(createdKey)} className="text-slate-400 hover:text-slate-200">
                      <Copy size={16} />
                    </button>
                  </div>
                </div>
              )}

              {loading ? (
                <p className="text-sm text-slate-500">{text({ zh: '加载中…', en: 'Loading…' })}</p>
              ) : (
                <div className="grid gap-3">
                  {keys.map((key) => (
                    <div key={key.name} className="rounded-2xl border border-white/10 bg-slate-950/85 p-4">
                      <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                        <div className="min-w-0">
                          <div className="text-[11px] uppercase tracking-[0.18em] text-slate-500">
                            {text({ zh: '密钥', en: 'Key' })}
                          </div>
                          <div className="mt-2 break-all font-mono text-xs text-slate-200">{key.name}</div>
                        </div>
                        <div className="flex flex-wrap gap-2">
                          <span className="rounded-full bg-cyan-400/10 px-3 py-1 text-[11px] uppercase tracking-[0.16em] text-cyan-200 ring-1 ring-cyan-400/15">
                            {key.role}
                          </span>
                          <button
                            onClick={() => handleDelete(key.name)}
                            className="inline-flex items-center gap-2 rounded-2xl border border-rose-400/15 px-3 py-2 text-xs text-rose-300 transition hover:bg-rose-500/10"
                          >
                            <Trash2 size={14} />
                            {text({ zh: '删除', en: 'Delete' })}
                          </button>
                        </div>
                      </div>
                      <div className="mt-3 text-xs text-slate-500">
                        {text({ zh: '创建于', en: 'Created' })} {new Date(key.created_at).toLocaleString()}
                      </div>
                    </div>
                  ))}
                  {keys.length === 0 && (
                    <div className="rounded-2xl border border-dashed border-white/10 bg-slate-950/60 px-4 py-8 text-center text-sm text-slate-500">
                      {text({ zh: '还没有服务端 API 密钥。', en: 'No service API keys yet.' })}
                    </div>
                  )}
                </div>
              )}
            </div>
          </Card>
        </div>

        <Card
          title={text({ zh: 'LLM Provider Studio', en: 'LLM Provider Studio' })}
          subtitle={text({
            zh: '按 OpenClaw 的思路提供 provider 目录、模型预设、连接测试和运行时保存，不再只有一个简单 URL 输入框。',
            en: 'Use an OpenClaw-style provider catalog with model presets, connection tests, and runtime save controls.',
          })}
          icon={<WandSparkles size={18} />}
        >
          <div className="space-y-4">
            <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
              {providerCatalog.map((provider) => {
                const active = provider.id === llmSettings.provider
                return (
                  <button
                    key={provider.id}
                    type="button"
                    onClick={() => applyPreset(provider.id)}
                    className={`rounded-2xl border p-4 text-left transition ${
                      active
                        ? 'border-cyan-400/30 bg-cyan-400/10'
                        : 'border-white/10 bg-slate-900/70 hover:bg-white/[0.05]'
                    }`}
                  >
                    <div className="flex items-center justify-between gap-3">
                      <strong className="text-sm text-white">{language === 'zh' ? provider.labelZh : provider.label}</strong>
                      <span className="rounded-full bg-white/6 px-2 py-1 text-[10px] uppercase tracking-[0.16em] text-slate-400">
                        {provider.apiStyle}
                      </span>
                    </div>
                    <div className="mt-2 break-all text-xs text-slate-500">{provider.defaultBaseUrl || text({ zh: '自定义地址', en: 'Custom base URL' })}</div>
                  </button>
                )
              })}
            </div>

            <div className="grid gap-3 md:grid-cols-2">
              <div className="rounded-2xl border border-white/10 bg-slate-900/70 p-4">
                <div className="mb-3 text-[11px] uppercase tracking-[0.18em] text-slate-500">
                  {text({ zh: '当前 Provider', en: 'Current provider' })}
                </div>
                <div className="text-lg font-semibold text-white">
                  {selectedPreset ? (language === 'zh' ? selectedPreset.labelZh : selectedPreset.label) : '—'}
                </div>
                <p className="mt-2 text-sm leading-6 text-slate-400">{selectedPreset?.note || '—'}</p>
                <div className="mt-3 flex flex-wrap gap-2">
                  {(selectedPreset?.modelSuggestions || []).slice(0, 5).map((model) => (
                    <button
                      key={model}
                      type="button"
                      onClick={() => setLlmSettings((current) => ({ ...current, model }))}
                      className={`rounded-full border px-3 py-1 text-xs transition ${
                        llmSettings.model === model
                          ? 'border-cyan-400/30 bg-cyan-400/12 text-cyan-100'
                          : 'border-white/10 bg-white/[0.03] text-slate-300'
                      }`}
                    >
                      {model}
                    </button>
                  ))}
                </div>
              </div>

              <div className="rounded-2xl border border-white/10 bg-slate-900/70 p-4">
                <div className="mb-3 text-[11px] uppercase tracking-[0.18em] text-slate-500">
                  {text({ zh: '状态', en: 'State' })}
                </div>
                <div className="grid gap-3 sm:grid-cols-2">
                  <StatusBox label={text({ zh: 'Provider', en: 'Provider' })} value={selectedPreset ? (language === 'zh' ? selectedPreset.labelZh : selectedPreset.label) : '—'} />
                  <StatusBox label={text({ zh: '模型', en: 'Model' })} value={llmSettings.model || '—'} mono />
                  <StatusBox label={text({ zh: '地址', en: 'Base URL' })} value={llmSettings.baseUrl || '—'} mono />
                  <StatusBox
                    label={text({ zh: '更新', en: 'Updated' })}
                    value={llmSettings.updatedAt ? new Date(llmSettings.updatedAt).toLocaleString() : text({ zh: '未保存', en: 'Not saved' })}
                  />
                </div>
              </div>
            </div>

            <div className="grid gap-3 md:grid-cols-2">
              <input
                type="text"
                className="rounded-2xl border border-white/10 bg-slate-950 px-4 py-3 text-sm text-white placeholder:text-slate-500 focus:border-cyan-400/50 focus:outline-none"
                placeholder={text({ zh: 'Base URL', en: 'Base URL' })}
                value={llmSettings.baseUrl}
                onChange={(event) => setLlmSettings((current) => ({ ...current, baseUrl: event.target.value }))}
              />
              <input
                type="text"
                className="rounded-2xl border border-white/10 bg-slate-950 px-4 py-3 text-sm text-white placeholder:text-slate-500 focus:border-cyan-400/50 focus:outline-none"
                placeholder={text({ zh: '模型名称', en: 'Model id' })}
                value={llmSettings.model}
                onChange={(event) => setLlmSettings((current) => ({ ...current, model: event.target.value }))}
              />
            </div>

            <div className="rounded-2xl border border-white/10 bg-slate-900/70 p-4">
              <div className="mb-2 flex items-center justify-between gap-3">
                <div className="text-[11px] uppercase tracking-[0.18em] text-slate-500">
                  {text({ zh: 'LLM API 密钥', en: 'LLM API key' })}
                </div>
                <div
                  className={`rounded-full px-3 py-1 text-[11px] font-medium ${
                    llmSettings.apiKey
                      ? 'bg-emerald-500/15 text-emerald-200 ring-1 ring-emerald-500/25'
                      : 'bg-amber-500/15 text-amber-200 ring-1 ring-amber-500/25'
                  }`}
                >
                  {llmSettings.apiKey ? text({ zh: '已配置', en: 'Configured' }) : text({ zh: '未配置', en: 'Missing' })}
                </div>
              </div>
              <div className="flex gap-2">
                <input
                  type={showLlmApiKey ? 'text' : 'password'}
                  className="w-full rounded-2xl border border-white/10 bg-slate-950 px-4 py-3 text-sm text-white placeholder:text-slate-500 focus:border-cyan-400/50 focus:outline-none"
                  placeholder={text({ zh: '输入 Provider API 密钥', en: 'Provider API key' })}
                  value={llmSettings.apiKey}
                  onChange={(event) => setLlmSettings((current) => ({ ...current, apiKey: event.target.value }))}
                />
                <button
                  type="button"
                  onClick={() => setShowLlmApiKey((value) => !value)}
                  className="inline-flex items-center justify-center rounded-2xl border border-white/10 px-3 text-slate-300 transition hover:bg-white/[0.05]"
                >
                  {showLlmApiKey ? <EyeOff size={16} /> : <Eye size={16} />}
                </button>
              </div>
            </div>

            <div className="flex flex-wrap gap-3">
              <button
                onClick={saveLLMSettings}
                disabled={llmSaving || llmLoading}
                className="inline-flex items-center gap-2 rounded-2xl bg-cyan-400 px-4 py-2.5 text-sm font-medium text-slate-950 transition hover:bg-cyan-300 disabled:cursor-not-allowed disabled:opacity-60"
              >
                <Save size={15} />
                {llmSaving ? text({ zh: '保存中…', en: 'Saving…' }) : text({ zh: '保存设置', en: 'Save' })}
              </button>
              <button
                onClick={testLLMSettings}
                disabled={llmTesting || llmLoading}
                className="inline-flex items-center gap-2 rounded-2xl border border-white/10 px-4 py-2.5 text-sm text-slate-200 transition hover:bg-white/[0.05] disabled:cursor-not-allowed disabled:opacity-60"
              >
                <WandSparkles size={15} />
                {llmTesting ? text({ zh: '测试中…', en: 'Testing…' }) : text({ zh: '测试连接', en: 'Test' })}
              </button>
            </div>

            {llmTestResult && (
              <div
                className={`rounded-2xl border px-4 py-3 text-sm leading-6 ${
                  llmTestResult.tone === 'healthy'
                    ? 'border-emerald-400/20 bg-emerald-400/10 text-emerald-100'
                    : 'border-rose-400/20 bg-rose-400/10 text-rose-100'
                }`}
              >
                {llmTestResult.message}
              </div>
            )}
          </div>
        </Card>
      </div>
    </div>
  )
}

function StatusBox({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="rounded-2xl border border-white/10 bg-white/[0.04] px-4 py-3">
      <div className="text-[11px] uppercase tracking-[0.18em] text-slate-500">{label}</div>
      <div className={`mt-2 break-all text-sm font-medium text-slate-100 ${mono ? 'font-mono' : ''}`}>{value}</div>
    </div>
  )
}
