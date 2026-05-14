import { useState, useEffect } from 'react'
import { toast } from 'sonner'
import { BookOpen, ChevronDown, Zap, TrendingUp, BarChart3, Loader } from 'lucide-react'
import { workflowApi } from '../api/client'

interface LearningInsight {
  category: 'performance' | 'accuracy' | 'cost' | 'pattern'
  title: string
  description: string
  impact: number
  actionable: boolean
  suggestions: string[]
}

interface LearningReportProps {
  workflowId: string
  onClose?: () => void
}

export default function LearningReport({ workflowId, onClose }: LearningReportProps) {
  const [expanded, setExpanded] = useState(false)
  const [loading, setLoading] = useState(false)
  const [insights, setInsights] = useState<LearningInsight[]>([])
  const [stats, setStats] = useState<{
    totalExecutions: number
    successRate: number
    avgDuration: number
    lastUpdated: string
  } | null>(null)

  useEffect(() => {
    if (expanded && insights.length === 0) {
      fetchLearningReport()
    }
  }, [expanded])

  const fetchLearningReport = async () => {
    if (!workflowId) {
      toast.error('Please save the workflow first')
      return
    }
    setLoading(true)
    try {
      const response = await workflowApi.summarize(workflowId)
      setInsights(response.insights || [])
      setStats({
        totalExecutions: response.total_executions || 0,
        successRate: response.success_rate || 0,
        avgDuration: response.avg_duration || 0,
        lastUpdated: new Date(response.updated_at || Date.now()).toLocaleDateString(),
      })
    } catch (error) {
      console.error('Failed to fetch learning report:', error)
      toast.error('Failed to fetch learning report')
    } finally {
      setLoading(false)
    }
  }

  const getCategoryColor = (category: string) => {
    switch (category) {
      case 'performance':
        return 'bg-blue-500/10 border-blue-500/30 text-blue-300'
      case 'accuracy':
        return 'bg-green-500/10 border-green-500/30 text-green-300'
      case 'cost':
        return 'bg-yellow-500/10 border-yellow-500/30 text-yellow-300'
      case 'pattern':
        return 'bg-purple-500/10 border-purple-500/30 text-purple-300'
      default:
        return 'bg-gray-500/10 border-gray-500/30 text-gray-300'
    }
  }

  const getCategoryIcon = (category: string) => {
    switch (category) {
      case 'performance':
        return <Zap size={16} />
      case 'accuracy':
        return <TrendingUp size={16} />
      case 'cost':
        return <BarChart3 size={16} />
      case 'pattern':
        return <BookOpen size={16} />
      default:
        return null
    }
  }

  return (
    <div className="border border-blue-500/30 rounded-lg overflow-hidden bg-blue-500/5">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full px-4 py-3 flex items-center justify-between hover:bg-blue-500/10 transition"
      >
        <div className="flex items-center gap-3">
          <BookOpen size={18} className="text-blue-400" />
          <span className="font-semibold text-blue-300">学习报告 - 自我总结与优化建议</span>
        </div>
        <ChevronDown size={18} className={`transition ${expanded ? 'rotate-180' : ''}`} />
      </button>

      {expanded && (
        <div className="border-t border-blue-500/20 p-4 space-y-4">
          {loading ? (
            <div className="flex items-center justify-center py-8">
              <Loader className="animate-spin text-blue-400" />
              <span className="ml-3 text-blue-300">生成学习报告中...</span>
            </div>
          ) : !stats ? (
            <button
              onClick={fetchLearningReport}
              className="w-full py-2 bg-blue-600 hover:bg-blue-700 text-white rounded transition"
            >
              生成学习报告
            </button>
          ) : (
            <>
              {/* 统计信息 */}
              <div className="grid grid-cols-2 gap-3 mb-4">
                <div className="bg-blue-500/10 border border-blue-500/30 p-3 rounded">
                  <div className="text-blue-400 text-xs font-semibold uppercase">执行次数</div>
                  <div className="text-blue-300 text-xl font-bold">{stats.totalExecutions}</div>
                </div>
                <div className="bg-green-500/10 border border-green-500/30 p-3 rounded">
                  <div className="text-green-400 text-xs font-semibold uppercase">成功率</div>
                  <div className="text-green-300 text-xl font-bold">{(stats.successRate * 100).toFixed(1)}%</div>
                </div>
              </div>

              {/* 平均执行时间 */}
              <div className="bg-surface/50 border border-blue-500/20 p-3 rounded">
                <div className="text-blue-300 text-sm font-semibold">平均执行时长</div>
                <div className="text-blue-200 text-lg mt-1">
                  {stats.avgDuration >= 1000
                    ? `${(stats.avgDuration / 1000).toFixed(1)}s`
                    : `${stats.avgDuration.toFixed(0)}ms`}
                </div>
              </div>

              {/* 学习洞察 */}
              {insights.length > 0 && (
                <div className="space-y-3">
                  <h3 className="text-sm font-semibold text-blue-300">关键洞察</h3>
                  {insights.map((insight, idx) => (
                    <div
                      key={idx}
                      className={`border p-3 rounded ${getCategoryColor(insight.category)}`}
                    >
                      <div className="flex items-start gap-2">
                        <div className="mt-1">{getCategoryIcon(insight.category)}</div>
                        <div className="flex-1">
                          <div className="font-semibold">{insight.title}</div>
                          <div className="text-sm opacity-80 mt-1">{insight.description}</div>
                          {insight.suggestions.length > 0 && (
                            <div className="mt-2 space-y-1">
                              <div className="text-xs font-semibold opacity-75">建议:</div>
                              {insight.suggestions.map((sug, sidx) => (
                                <div key={sidx} className="text-xs opacity-75 pl-3 border-l border-current">
                                  • {sug}
                                </div>
                              ))}
                            </div>
                          )}
                          <div className="mt-2 text-xs opacity-60">
                            影响度: <span className="font-semibold">{(insight.impact * 100).toFixed(0)}%</span>
                          </div>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}

              {/* 最后更新 */}
              <div className="text-xs text-gray-400 text-center pt-2 border-t border-blue-500/20">
                最后更新: {stats.lastUpdated}
              </div>

              <button
                onClick={onClose}
                className="w-full py-2 mt-2 bg-gray-700 hover:bg-gray-600 text-gray-300 rounded transition"
              >
                关闭
              </button>
            </>
          )}
        </div>
      )}
    </div>
  )
}
