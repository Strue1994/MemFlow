import { useState, useEffect } from 'react'
import { workflowApi } from '../api/client'
import { Bot, Zap, AlertTriangle, TrendingDown, Info, CheckCircle, Loader2, RefreshCw } from 'lucide-react'

interface Suggestion {
  type: string
  priority: 'high' | 'medium' | 'low'
  title: string
  detail: string
}

interface AIOptimizerProps {
  workflowId?: string | null
  onClose?: () => void
}

const PRIORITY_CONFIG = {
  high:   { color: 'text-red-400',    bg: 'bg-red-500/10',    border: 'border-red-500/30',    icon: AlertTriangle },
  medium: { color: 'text-yellow-400', bg: 'bg-yellow-500/10', border: 'border-yellow-500/30', icon: TrendingDown },
  low:    { color: 'text-blue-400',   bg: 'bg-blue-500/10',   border: 'border-blue-500/30',   icon: Info },
}

const TYPE_LABELS: Record<string, string> = {
  error_handling: '错误处理',
  performance:    '性能优化',
  reliability:    '可靠性',
  optimization:   '结构优化',
  info:           '提示',
}

export default function AIOptimizer({ workflowId, onClose }: AIOptimizerProps) {
  const [loading, setLoading] = useState(false)
  const [suggestions, setSuggestions] = useState<Suggestion[]>([])
  const [applied, setApplied] = useState<Set<number>>(new Set())
  const [error, setError] = useState<string | null>(null)

  const fetchSuggestions = async () => {
    if (!workflowId) {
      setSuggestions(getGenericSuggestions())
      return
    }
    setLoading(true)
    setError(null)
    try {
      // Use the new optimize API instead of deprecated aiSuggest
      const data = await workflowApi.optimize(workflowId)
      // Transform the response to match Suggestion interface
      setSuggestions((data.params || []).map((p: any) => ({
        type: 'optimization',
        priority: p.impact === 'high' ? 'high' : p.impact === 'medium' ? 'medium' : 'low',
        title: p.name,
        detail: p.description,
      })))
    } catch (e: any) {
      setError('获取建议失败，显示通用建议')
      setSuggestions(getGenericSuggestions())
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchSuggestions()
  }, [workflowId])

  const handleApply = (index: number) => {
    setApplied(prev => new Set([...prev, index]))
    // 实际应用逻辑可在此处通过回调或 store 更新节点
  }

  const high   = suggestions.filter(s => s.priority === 'high')
  const medium = suggestions.filter(s => s.priority === 'medium')
  const low    = suggestions.filter(s => s.priority === 'low')

  return (
    <div className="w-80 flex flex-col h-full border-l border-gray-700 bg-gray-900">
      {/* 标题栏 */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-gray-700 bg-surface">
        <div className="flex items-center gap-2">
          <div className="w-7 h-7 rounded bg-purple-500/20 flex items-center justify-center">
            <Bot size={14} className="text-purple-400" />
          </div>
          <span className="text-sm font-medium text-white">AI 优化建议</span>
          {suggestions.length > 0 && (
            <span className="text-xs bg-purple-500/20 text-purple-300 px-1.5 py-0.5 rounded">
              {suggestions.length}
            </span>
          )}
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={fetchSuggestions}
            disabled={loading}
            title="刷新"
            className="p-1.5 rounded hover:bg-gray-700 text-gray-400 hover:text-white transition-colors"
          >
            <RefreshCw size={12} className={loading ? 'animate-spin' : ''} />
          </button>
          {onClose && (
            <button
              onClick={onClose}
              className="p-1.5 rounded hover:bg-gray-700 text-gray-400 hover:text-white transition-colors text-xs"
            >
              ✕
            </button>
          )}
        </div>
      </div>

      {/* 内容区 */}
      <div className="flex-1 overflow-y-auto p-3 space-y-3">
        {loading && (
          <div className="flex flex-col items-center justify-center py-12 text-gray-500">
            <Loader2 size={24} className="animate-spin mb-3" />
            <span className="text-xs">正在分析工作流...</span>
          </div>
        )}

        {!loading && error && (
          <div className="text-xs text-yellow-400 bg-yellow-500/10 border border-yellow-500/20 rounded p-2.5 mb-2">
            {error}
          </div>
        )}

        {!loading && suggestions.length === 0 && (
          <div className="flex flex-col items-center justify-center py-12 text-gray-500">
            <CheckCircle size={24} className="mb-3 text-green-500" />
            <span className="text-xs text-center">未发现优化建议<br />工作流状态良好</span>
          </div>
        )}

        {!loading && suggestions.length > 0 && (
          <>
            {/* 概览 */}
            <div className="grid grid-cols-3 gap-2">
              <StatBadge label="高优先级" count={high.length} color="text-red-400" bg="bg-red-500/10" />
              <StatBadge label="中优先级" count={medium.length} color="text-yellow-400" bg="bg-yellow-500/10" />
              <StatBadge label="低优先级" count={low.length} color="text-blue-400" bg="bg-blue-500/10" />
            </div>

            {/* 建议列表 */}
            {suggestions.map((s, i) => {
              const cfg = PRIORITY_CONFIG[s.priority] || PRIORITY_CONFIG.low
              const Icon = cfg.icon
              const isApplied = applied.has(i)
              return (
                <div
                  key={i}
                  className={`rounded-lg border p-3 transition-opacity ${cfg.bg} ${cfg.border} ${isApplied ? 'opacity-50' : ''}`}
                >
                  <div className="flex items-start gap-2 mb-1.5">
                    <Icon size={13} className={`mt-0.5 flex-shrink-0 ${cfg.color}`} />
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between gap-1 mb-0.5">
                        <span className={`text-xs font-medium ${cfg.color}`}>{s.title}</span>
                        <span className="text-xs text-gray-500 whitespace-nowrap">
                          {TYPE_LABELS[s.type] || s.type}
                        </span>
                      </div>
                      <p className="text-xs text-gray-400 leading-relaxed">{s.detail}</p>
                    </div>
                  </div>
                  <div className="flex justify-end mt-2">
                    {isApplied ? (
                      <span className="text-xs text-green-400 flex items-center gap-1">
                        <CheckCircle size={11} /> 已标记
                      </span>
                    ) : (
                      <button
                        onClick={() => handleApply(i)}
                        className="text-xs bg-gray-700 hover:bg-gray-600 text-gray-200 px-2.5 py-1 rounded transition-colors"
                      >
                        采纳建议
                      </button>
                    )}
                  </div>
                </div>
              )
            })}
          </>
        )}
      </div>

      {/* 底部：一键优化 */}
      {!loading && suggestions.length > 0 && (
        <div className="p-3 border-t border-gray-700">
          <button
            onClick={() => {
              const newSet = new Set(suggestions.map((_, i) => i))
              setApplied(newSet)
            }}
            className="w-full flex items-center justify-center gap-2 bg-purple-600 hover:bg-purple-500 text-white px-4 py-2 rounded-lg text-xs font-medium transition-colors"
          >
            <Zap size={13} />
            一键全部采纳
          </button>
        </div>
      )}
    </div>
  )
}

function StatBadge({
  label, count, color, bg,
}: { label: string; count: number; color: string; bg: string }) {
  return (
    <div className={`${bg} rounded-lg p-2 text-center`}>
      <div className={`text-lg font-bold ${color}`}>{count}</div>
      <div className="text-xs text-gray-500">{label}</div>
    </div>
  )
}

function getGenericSuggestions(): Suggestion[] {
  return [
    {
      type: 'error_handling', priority: 'high',
      title: '添加错误处理', detail: '在关键节点后添加 If 分支，区分成功和失败路径，避免静默失败。',
    },
    {
      type: 'performance', priority: 'medium',
      title: '并行化独立节点', detail: '将互不依赖的节点改为并行执行，可减少总耗时。',
    },
    {
      type: 'optimization', priority: 'low',
      title: '添加日志节点', detail: '在关键步骤前后添加日志记录，便于排查问题。',
    },
  ]
}
