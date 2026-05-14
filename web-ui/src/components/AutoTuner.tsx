import { useState, useEffect } from 'react'
import { toast } from 'sonner'
import { ChevronDown, Zap, TrendingUp, Loader } from 'lucide-react'
import { workflowApi } from '../api/client'

interface AutoTuneParam {
  name: string
  current: number
  recommended: number
  impact: 'high' | 'medium' | 'low'
  description: string
}

interface AutoTunerProps {
  workflowId: string
  onApplyTuning?: (params: Record<string, any>) => void
}

export default function AutoTuner({ workflowId, onApplyTuning }: AutoTunerProps) {
  const [expanded, setExpanded] = useState(false)
  const [loading, setLoading] = useState(false)
  const [params, setParams] = useState<AutoTuneParam[]>([])
  const [selectedParams, setSelectedParams] = useState<Set<string>>(new Set())
  const [improvement, setImprovement] = useState<{
    speedup: number
    accuracy: number
    costSavings: number
  } | null>(null)

  useEffect(() => {
    if (expanded && params.length === 0) {
      fetchTuningRecommendations()
    }
  }, [expanded])

  const fetchTuningRecommendations = async () => {
    if (!workflowId) {
      toast.error('Please save the workflow first')
      return
    }
    setLoading(true)
    try {
      const response = await workflowApi.optimize(workflowId)
      setParams(response.params || [])
      setImprovement({
        speedup: response.estimated_speedup || 1.2,
        accuracy: response.estimated_accuracy || 0.95,
        costSavings: response.estimated_cost_savings || 0.15,
      })
    } catch (error) {
      console.error('Failed to fetch tuning recommendations:', error)
      toast.error('Failed to fetch tuning recommendations')
    } finally {
      setLoading(false)
    }
  }

  const handleToggleParam = (paramName: string) => {
    const newSelected = new Set(selectedParams)
    if (newSelected.has(paramName)) {
      newSelected.delete(paramName)
    } else {
      newSelected.add(paramName)
    }
    setSelectedParams(newSelected)
  }

  const handleApplyTuning = async () => {
    const tuningParams: Record<string, any> = {}
    params.forEach((p) => {
      if (selectedParams.has(p.name)) {
        tuningParams[p.name] = p.recommended
      }
    })

    if (Object.keys(tuningParams).length === 0) {
      toast.warning('Please select at least one parameter')
      return
    }

    setLoading(true)
    try {
      await workflowApi.applyTuning(workflowId, tuningParams)
      toast.success(`Applied ${Object.keys(tuningParams).length} tuning recommendations`)
      onApplyTuning?.(tuningParams)
      setExpanded(false)
    } catch (error) {
      console.error('Failed to apply tuning:', error)
      toast.error('Failed to apply tuning')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="border border-purple-500/30 rounded-lg overflow-hidden bg-purple-500/5">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full px-4 py-3 flex items-center justify-between hover:bg-purple-500/10 transition"
      >
        <div className="flex items-center gap-3">
          <Zap size={18} className="text-purple-400" />
          <span className="font-semibold text-purple-300">AutoTuner - 参数自动调整</span>
        </div>
        <ChevronDown size={18} className={`transition ${expanded ? 'rotate-180' : ''}`} />
      </button>

      {expanded && (
        <div className="border-t border-purple-500/20 p-4 space-y-4">
          {loading ? (
            <div className="flex items-center justify-center py-8">
              <Loader className="animate-spin text-purple-400" />
              <span className="ml-3 text-purple-300">分析工作流中...</span>
            </div>
          ) : params.length === 0 ? (
            <button
              onClick={fetchTuningRecommendations}
              className="w-full py-2 bg-purple-600 hover:bg-purple-700 text-white rounded transition"
            >
              生成优化建议
            </button>
          ) : (
            <>
              {improvement && (
                <div className="grid grid-cols-3 gap-3 mb-4">
                  <div className="bg-green-500/10 border border-green-500/30 p-3 rounded">
                    <div className="text-green-400 text-sm font-semibold">性能提升</div>
                    <div className="text-green-300 text-lg">{(improvement.speedup * 100).toFixed(0)}%</div>
                  </div>
                  <div className="bg-blue-500/10 border border-blue-500/30 p-3 rounded">
                    <div className="text-blue-400 text-sm font-semibold">准确度</div>
                    <div className="text-blue-300 text-lg">{(improvement.accuracy * 100).toFixed(1)}%</div>
                  </div>
                  <div className="bg-yellow-500/10 border border-yellow-500/30 p-3 rounded">
                    <div className="text-yellow-400 text-sm font-semibold">成本节省</div>
                    <div className="text-yellow-300 text-lg">{(improvement.costSavings * 100).toFixed(0)}%</div>
                  </div>
                </div>
              )}

              <div className="space-y-3 max-h-64 overflow-y-auto">
                {params.map((param) => (
                  <div key={param.name} className="p-3 bg-surface/50 rounded border border-purple-500/20 hover:border-purple-500/50 transition">
                    <div className="flex items-start gap-3">
                      <input
                        type="checkbox"
                        checked={selectedParams.has(param.name)}
                        onChange={() => handleToggleParam(param.name)}
                        className="mt-1 w-4 h-4 accent-purple-500"
                      />
                      <div className="flex-1">
                        <div className="flex items-center gap-2">
                          <span className="font-semibold text-purple-300">{param.name}</span>
                          <span className={`text-xs px-2 py-1 rounded ${
                            param.impact === 'high' ? 'bg-red-500/20 text-red-300' :
                            param.impact === 'medium' ? 'bg-yellow-500/20 text-yellow-300' :
                            'bg-green-500/20 text-green-300'
                          }`}>
                            {param.impact.toUpperCase()}
                          </span>
                        </div>
                        <div className="text-sm text-gray-400 mt-1">{param.description}</div>
                        <div className="flex items-center gap-4 mt-2 text-xs text-gray-300">
                          <span>当前: <span className="text-yellow-300 font-semibold">{param.current}</span></span>
                          <span>→</span>
                          <span>推荐: <span className="text-green-300 font-semibold">{param.recommended}</span></span>
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>

              <div className="flex gap-2 pt-3">
                <button
                  onClick={handleApplyTuning}
                  disabled={selectedParams.size === 0 || loading}
                  className="flex-1 py-2 bg-purple-600 hover:bg-purple-700 disabled:bg-gray-600 disabled:opacity-50 text-white rounded transition flex items-center justify-center gap-2"
                >
                  <TrendingUp size={16} />
                  应用调整 ({selectedParams.size})
                </button>
                <button
                  onClick={() => setExpanded(false)}
                  className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-gray-300 rounded transition"
                >
                  关闭
                </button>
              </div>
            </>
          )}
        </div>
      )}
    </div>
  )
}
