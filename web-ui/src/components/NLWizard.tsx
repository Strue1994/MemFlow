import { useState } from 'react'
import { toast } from 'sonner'
import { Sparkles, Loader, RefreshCw, Check } from 'lucide-react'
import { workflowApi } from '../api/client'

interface NLWizardProps {
  workflowId?: string
  onGenerated?: (workflow: any) => void
}

export default function NLWizard({ workflowId: initialWorkflowId, onGenerated }: NLWizardProps) {
  const [description, setDescription] = useState('')
  const [loading, setLoading] = useState(false)
  const [improvements, setImprovements] = useState<string[]>([])
  const [feedback, setFeedback] = useState('')
  const [step, setStep] = useState<'input' | 'improving' | 'result'>('input')

  const handleGenerate = async () => {
    if (!description.trim()) {
      toast.error('Please enter a workflow description')
      return
    }

    setLoading(true)
    setStep('improving')
    try {
      // 如果有现有工作流，进行增强；否则创建新的
      if (initialWorkflowId) {
        const response = await workflowApi.enhanceNLWorkflow(description, initialWorkflowId)
        setImprovements(response.improvements || [])
        setFeedback(response.learning_feedback || '')
        onGenerated?.(response.improved_workflow)
      } else {
        await workflowApi.createWorkflowV2(description)
        setFeedback('Workflow created successfully from natural language description')
        setImprovements([
          'Initial workflow structure generated',
          'Nodes and connections established',
          'Ready for further optimization'
        ])
        setStep('result')
      }
      setStep('result')
      toast.success('Workflow enhanced with AI improvements')
    } catch (error) {
      console.error('Failed to generate workflow:', error)
      toast.error('Failed to generate workflow')
      setStep('input')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="space-y-4">
      {/* 输入阶段 */}
      {step === 'input' && (
        <div className="bg-surface border border-yellow-500/30 rounded-lg p-4">
          <div className="flex items-center gap-3 mb-4">
            <Sparkles size={20} className="text-yellow-400" />
            <h2 className="text-lg font-semibold text-yellow-300">AI 自然语言创建工作流</h2>
          </div>

          <textarea
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder={
              initialWorkflowId
                ? '描述你想如何改进当前工作流...（例如：加快处理速度，增加错误处理，优化参数）'
                : '用自然语言描述你的工作流... (例如：获取用户数据，进行数据验证，然后发送到数据库)'
            }
            className="w-full p-3 bg-background border border-gray-700 rounded text-gray-100 placeholder-gray-500 focus:border-yellow-500 focus:outline-none resize-none h-32"
          />

          <div className="mt-4 flex gap-3">
            <button
              onClick={handleGenerate}
              disabled={loading || !description.trim()}
              className="flex-1 flex items-center justify-center gap-2 py-2 bg-yellow-600 hover:bg-yellow-700 disabled:bg-gray-600 disabled:opacity-50 text-white rounded transition font-semibold"
            >
              {loading ? (
                <>
                  <Loader size={16} className="animate-spin" />
                  正在生成...
                </>
              ) : (
                <>
                  <Sparkles size={16} />
                  {initialWorkflowId ? '优化工作流' : '创建工作流'}
                </>
              )}
            </button>
          </div>

          <div className="mt-4 p-3 bg-blue-500/10 border border-blue-500/30 rounded text-blue-300 text-sm">
            <div className="font-semibold mb-2">💡 提示:</div>
            <ul className="space-y-1 text-xs">
              <li>• 清晰描述工作流的目标和步骤</li>
              <li>• 提及任何特定的集成或数据处理需求</li>
              <li>• AI 会自动优化流程并应用最佳实践</li>
              <li>• 生成后可以进行进一步的微调</li>
            </ul>
          </div>
        </div>
      )}

      {/* 正在改进阶段 */}
      {step === 'improving' && (
        <div className="bg-surface border border-yellow-500/30 rounded-lg p-4">
          <div className="flex items-center justify-center py-8">
            <Loader className="animate-spin text-yellow-400 mr-3" size={24} />
            <div>
              <div className="font-semibold text-yellow-300">AI 正在优化工作流...</div>
              <div className="text-sm text-gray-400 mt-1">分析现有逻辑，应用学习经验</div>
            </div>
          </div>
        </div>
      )}

      {/* 结果阶段 */}
      {step === 'result' && (
        <div className="bg-surface border border-green-500/30 rounded-lg p-4">
          <div className="flex items-center gap-3 mb-4">
            <Check size={20} className="text-green-400" />
            <h2 className="text-lg font-semibold text-green-300">
              {initialWorkflowId ? '工作流已优化' : '工作流已生成'}
            </h2>
          </div>

          {/* 改进列表 */}
          {improvements.length > 0 && (
            <div className="mb-4 space-y-2">
              <div className="text-sm font-semibold text-gray-300">应用的改进:</div>
              {improvements.map((imp, idx) => (
                <div
                  key={idx}
                  className="flex items-start gap-3 p-2 bg-green-500/10 border border-green-500/20 rounded text-green-300 text-sm"
                >
                  <Check size={14} className="mt-1 flex-shrink-0" />
                  <span>{imp}</span>
                </div>
              ))}
            </div>
          )}

          {/* 反馈 */}
          {feedback && (
            <div className="mb-4 p-3 bg-blue-500/10 border border-blue-500/30 rounded text-blue-300 text-sm">
              <div className="font-semibold mb-2">AI 反馈:</div>
              <div>{feedback}</div>
            </div>
          )}

          {/* 操作按钮 */}
          <div className="flex gap-3">
            <button
              onClick={() => {
                setStep('input')
                setDescription('')
                setImprovements([])
                setFeedback('')
              }}
              className="flex-1 flex items-center justify-center gap-2 py-2 bg-gray-700 hover:bg-gray-600 text-gray-300 rounded transition"
            >
              <RefreshCw size={16} />
              再次优化
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
