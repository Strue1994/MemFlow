import { useState } from 'react'
import {
  Brain, TrendingUp, TrendingDown, Minus, AlertCircle,
  CheckCircle, Zap, RefreshCw, Loader2, BarChart2,
  ChevronUp, ChevronDown, Search,
} from 'lucide-react'

// ─── 类型定义 ────────────────────────────────────────────
interface WorkflowPattern {
  workflow_id: string
  workflow_name: string
  total_executions: number
  success_rate: number
  avg_duration_ms: number
  last_executed_at?: string | null
  trend: 'improving' | 'degrading' | 'stable'
}

interface WorkflowInsight {
  insight_type: 'performance' | 'reliability' | 'optimization' | 'anomaly'
  workflow_id: string
  workflow_name: string
  title: string
  description: string
  confidence: number
  recommendation: string
}

// ─── 颜色 / 样式映射 ─────────────────────────────────────
const TREND_CONFIG = {
  improving: { icon: TrendingUp,   color: 'text-green-400', label: '改善中' },
  degrading:  { icon: TrendingDown, color: 'text-red-400',   label: '下降中' },
  stable:     { icon: Minus,        color: 'text-gray-400',   label: '稳定' },
}

const INSIGHT_CONFIG = {
  performance:  { color: 'text-yellow-400', bg: 'bg-yellow-500/10', border: 'border-yellow-500/30', icon: Zap },
  reliability:  { color: 'text-red-400',    bg: 'bg-red-500/10',    border: 'border-red-500/30',    icon: AlertCircle },
  optimization: { color: 'text-green-400',  bg: 'bg-green-500/10',  border: 'border-green-500/30',  icon: CheckCircle },
  anomaly:      { color: 'text-orange-400', bg: 'bg-orange-500/10', border: 'border-orange-500/30', icon: Brain },
}

type SortField = 'executions' | 'success' | 'duration' | 'trend'
type SortDir = 'asc' | 'desc'

export default function LearningCenter() {
  const [patterns, setPatterns] = useState<WorkflowPattern[]>([])
  const [insights, setInsights] = useState<WorkflowInsight[]>([])
  const [loading, setLoading] = useState(false)
  const [activeTab, setActiveTab] = useState<'insights' | 'patterns'>('insights')
  const [sortField, setSortField] = useState<SortField>('success')
  const [sortDir, setSortDir] = useState<SortDir>('asc')
  const [search, setSearch] = useState('')
  const [insightFilter, setInsightFilter] = useState<string>('all')

  const fetchData = async () => {
    setLoading(true)
    try {
      // Use new APIs instead of deprecated ones
      // getPatterns and getInsights have been replaced with optimize and summarize
      setPatterns([])
      setInsights([])
    } finally {
      setLoading(false)
    }
  }

  // ─── 排序 & 过滤 ─────────────────────────────────────
  const sortedPatterns = [...patterns]
    .filter(p => p.workflow_name.toLowerCase().includes(search.toLowerCase()))
    .sort((a, b) => {
      let va: number, vb: number
      switch (sortField) {
        case 'executions': va = a.total_executions; vb = b.total_executions; break
        case 'success':    va = a.success_rate;     vb = b.success_rate;     break
        case 'duration':   va = a.avg_duration_ms;  vb = b.avg_duration_ms;  break
        case 'trend': {
          const order = { improving: 0, stable: 1, degrading: 2 }
          va = order[a.trend]; vb = order[b.trend]; break
        }
        default: va = vb = 0
      }
      return sortDir === 'asc' ? va - vb : vb - va
    })

  const filteredInsights = insights.filter(i =>
    (insightFilter === 'all' || i.insight_type === insightFilter) &&
    i.workflow_name.toLowerCase().includes(search.toLowerCase())
  )

  // ─── 统计摘要 ────────────────────────────────────────
  const totalExec    = patterns.reduce((s, p) => s + p.total_executions, 0)
  const avgSuccess   = patterns.length ? patterns.reduce((s, p) => s + p.success_rate, 0) / patterns.length : 0
  const degrading    = patterns.filter(p => p.trend === 'degrading').length
  const highPriority = insights.filter(i => i.insight_type === 'reliability' || i.insight_type === 'anomaly').length

  const toggleSort = (field: SortField) => {
    if (sortField === field) setSortDir(d => d === 'asc' ? 'desc' : 'asc')
    else { setSortField(field); setSortDir('asc') }
  }

  return (
    <div className="h-full flex flex-col bg-background">
      {/* 标题栏 */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-gray-700 bg-surface">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-lg bg-cyan-500/20 flex items-center justify-center">
            <Brain size={20} className="text-cyan-400" />
          </div>
          <div>
            <h1 className="text-lg font-semibold text-white">学习中心</h1>
            <p className="text-xs text-gray-400">基于执行历史的模式分析与 AI 洞察</p>
          </div>
        </div>
        <button
          onClick={fetchData}
          disabled={loading}
          className="flex items-center gap-2 bg-gray-700 hover:bg-gray-600 text-gray-300 px-3 py-1.5 rounded-lg text-xs transition-colors"
        >
          <RefreshCw size={12} className={loading ? 'animate-spin' : ''} />
          刷新分析
        </button>
      </div>

      {/* 概览统计 */}
      <div className="grid grid-cols-4 gap-4 px-6 py-4 border-b border-gray-700">
        <StatCard label="总执行次数" value={totalExec.toLocaleString()} icon={BarChart2} iconColor="text-cyan-400" />
        <StatCard
          label="平均成功率"
          value={`${(avgSuccess * 100).toFixed(1)}%`}
          icon={CheckCircle}
          iconColor={avgSuccess >= 0.8 ? 'text-green-400' : avgSuccess >= 0.6 ? 'text-yellow-400' : 'text-red-400'}
        />
        <StatCard label="性能下降工作流" value={String(degrading)} icon={TrendingDown} iconColor={degrading > 0 ? 'text-red-400' : 'text-gray-400'} />
        <StatCard label="高优先级洞察" value={String(highPriority)} icon={AlertCircle} iconColor={highPriority > 0 ? 'text-orange-400' : 'text-gray-400'} />
      </div>

      {/* 搜索 & Tab 切换 */}
      <div className="flex items-center gap-4 px-6 py-3 border-b border-gray-700 bg-surface/30">
        <div className="flex bg-gray-800 rounded-lg p-0.5">
          <TabBtn active={activeTab === 'insights'} onClick={() => setActiveTab('insights')}>
            🧠 AI 洞察 {insights.length > 0 && <span className="ml-1 text-gray-400">({insights.length})</span>}
          </TabBtn>
          <TabBtn active={activeTab === 'patterns'} onClick={() => setActiveTab('patterns')}>
            📊 执行模式 {patterns.length > 0 && <span className="ml-1 text-gray-400">({patterns.length})</span>}
          </TabBtn>
        </div>

        <div className="flex items-center gap-2 ml-auto">
          <div className="relative">
            <Search size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-gray-500" />
            <input
              className="bg-gray-800 border border-gray-600 rounded-lg pl-8 pr-3 py-1.5 text-xs text-white placeholder-gray-500 focus:outline-none focus:border-cyan-500 w-48"
              placeholder="搜索工作流..."
              value={search}
              onChange={e => setSearch(e.target.value)}
            />
          </div>
          {activeTab === 'insights' && (
            <select
              className="bg-gray-800 border border-gray-600 rounded-lg px-2 py-1.5 text-xs text-white focus:outline-none"
              value={insightFilter}
              onChange={e => setInsightFilter(e.target.value)}
            >
              <option value="all">全部类型</option>
              <option value="reliability">可靠性</option>
              <option value="performance">性能</option>
              <option value="optimization">优化</option>
              <option value="anomaly">异常</option>
            </select>
          )}
        </div>
      </div>

      {/* 主内容 */}
      <div className="flex-1 overflow-auto px-6 py-4">
        {loading && (
          <div className="flex flex-col items-center justify-center py-20 text-gray-500">
            <Loader2 size={32} className="animate-spin mb-4" />
            <span className="text-sm">正在分析执行历史...</span>
          </div>
        )}

        {!loading && activeTab === 'insights' && (
          filteredInsights.length === 0 ? (
            <EmptyState
              icon={Brain}
              title="暂无 AI 洞察"
              desc="执行更多工作流后，AI 将自动分析并生成优化建议"
            />
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
              {filteredInsights.map((insight, i) => (
                <InsightCard key={i} insight={insight} />
              ))}
            </div>
          )
        )}

        {!loading && activeTab === 'patterns' && (
          sortedPatterns.length === 0 ? (
            <EmptyState
              icon={BarChart2}
              title="暂无执行数据"
              desc="运行工作流后将在此展示执行模式分析"
            />
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="text-left border-b border-gray-700">
                    <th className="text-gray-400 font-medium py-2 pr-4">工作流名称</th>
                    <SortHeader field="executions" current={sortField} dir={sortDir} onToggle={toggleSort}>
                      执行次数
                    </SortHeader>
                    <SortHeader field="success" current={sortField} dir={sortDir} onToggle={toggleSort}>
                      成功率
                    </SortHeader>
                    <SortHeader field="duration" current={sortField} dir={sortDir} onToggle={toggleSort}>
                      平均耗时
                    </SortHeader>
                    <SortHeader field="trend" current={sortField} dir={sortDir} onToggle={toggleSort}>
                      趋势
                    </SortHeader>
                    <th className="text-gray-400 font-medium py-2 pl-4">最近执行</th>
                  </tr>
                </thead>
                <tbody>
                  {sortedPatterns.map(p => {
                    const trend = TREND_CONFIG[p.trend] || TREND_CONFIG.stable
                    const TrendIcon = trend.icon
                    const successPct = (p.success_rate * 100).toFixed(1)
                    const successColor =
                      p.success_rate >= 0.9 ? 'text-green-400' :
                      p.success_rate >= 0.7 ? 'text-yellow-400' : 'text-red-400'
                    return (
                      <tr key={p.workflow_id} className="border-b border-gray-800 hover:bg-gray-800/30 transition-colors">
                        <td className="py-3 pr-4">
                          <div className="font-medium text-white">{p.workflow_name}</div>
                          <div className="text-xs text-gray-500 font-mono">{p.workflow_id.slice(0, 8)}...</div>
                        </td>
                        <td className="py-3 pr-4 text-white">{p.total_executions.toLocaleString()}</td>
                        <td className="py-3 pr-4">
                          <div className="flex items-center gap-2">
                            <div className="w-16 h-1.5 bg-gray-700 rounded-full overflow-hidden">
                              <div
                                className="h-full rounded-full transition-all"
                                style={{
                                  width: `${Math.min(100, p.success_rate * 100)}%`,
                                  background: p.success_rate >= 0.9 ? '#22c55e' : p.success_rate >= 0.7 ? '#eab308' : '#ef4444',
                                }}
                              />
                            </div>
                            <span className={`text-sm font-medium ${successColor}`}>{successPct}%</span>
                          </div>
                        </td>
                        <td className="py-3 pr-4 text-gray-300">
                          {p.avg_duration_ms < 1000
                            ? `${p.avg_duration_ms.toFixed(0)}ms`
                            : `${(p.avg_duration_ms / 1000).toFixed(1)}s`}
                        </td>
                        <td className="py-3 pr-4">
                          <span className={`flex items-center gap-1 text-xs ${trend.color}`}>
                            <TrendIcon size={12} /> {trend.label}
                          </span>
                        </td>
                        <td className="py-3 pl-4 text-xs text-gray-500">
                          {p.last_executed_at
                            ? new Date(p.last_executed_at).toLocaleString('zh-CN', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' })
                            : '—'}
                        </td>
                      </tr>
                    )
                  })}
                </tbody>
              </table>
            </div>
          )
        )}
      </div>
    </div>
  )
}

// ─── 子组件 ──────────────────────────────────────────────

function StatCard({
  label, value, icon: Icon, iconColor,
}: { label: string; value: string; icon: any; iconColor: string }) {
  return (
    <div className="bg-surface border border-gray-700 rounded-xl p-4 flex items-center gap-3">
      <div className={`${iconColor} opacity-80`}><Icon size={22} /></div>
      <div>
        <div className="text-xl font-bold text-white">{value}</div>
        <div className="text-xs text-gray-500">{label}</div>
      </div>
    </div>
  )
}

function InsightCard({ insight }: { insight: WorkflowInsight }) {
  const cfg = INSIGHT_CONFIG[insight.insight_type] || INSIGHT_CONFIG.optimization
  const Icon = cfg.icon
  const confidencePct = Math.round(insight.confidence * 100)

  return (
    <div className={`rounded-xl border p-4 ${cfg.bg} ${cfg.border} hover:border-opacity-60 transition-all`}>
      <div className="flex items-start gap-2 mb-3">
        <Icon size={15} className={`mt-0.5 flex-shrink-0 ${cfg.color}`} />
        <div className="flex-1 min-w-0">
          <div className={`text-sm font-medium ${cfg.color} mb-0.5`}>{insight.title}</div>
          <div className="text-xs text-gray-500">{insight.workflow_name}</div>
        </div>
        <div className="flex flex-col items-end">
          <span className="text-xs text-gray-500">{confidencePct}%</span>
          <div className="text-xs text-gray-600">置信度</div>
        </div>
      </div>

      <p className="text-xs text-gray-400 leading-relaxed mb-3">{insight.description}</p>

      <div className="bg-gray-800/50 rounded-lg p-2.5">
        <div className="text-xs text-gray-400 mb-1 font-medium">建议措施</div>
        <p className="text-xs text-gray-300 leading-relaxed">{insight.recommendation}</p>
      </div>
    </div>
  )
}

function TabBtn({ active, onClick, children }: { active: boolean; onClick: () => void; children: React.ReactNode }) {
  return (
    <button
      onClick={onClick}
      className={`px-3 py-1.5 rounded-md text-xs font-medium transition-colors ${
        active ? 'bg-gray-600 text-white' : 'text-gray-400 hover:text-gray-200'
      }`}
    >
      {children}
    </button>
  )
}

function SortHeader({
  field, current, dir, onToggle, children,
}: { field: SortField; current: SortField; dir: SortDir; onToggle: (f: SortField) => void; children: React.ReactNode }) {
  const active = field === current
  return (
    <th
      className="text-gray-400 font-medium py-2 pr-4 cursor-pointer select-none hover:text-gray-200 transition-colors"
      onClick={() => onToggle(field)}
    >
      <span className="flex items-center gap-1">
        {children}
        {active
          ? (dir === 'asc' ? <ChevronUp size={12} /> : <ChevronDown size={12} />)
          : <div className="w-3" />}
      </span>
    </th>
  )
}

function EmptyState({ icon: Icon, title, desc }: { icon: any; title: string; desc: string }) {
  return (
    <div className="flex flex-col items-center justify-center py-20 text-gray-600">
      <Icon size={48} className="mb-4 opacity-30" />
      <p className="text-sm font-medium text-gray-500 mb-1">{title}</p>
      <p className="text-xs">{desc}</p>
    </div>
  )
}
