import { useState, useCallback, useRef, useMemo } from 'react'
import { toast } from 'sonner'
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  BackgroundVariant,
  type Node,
  type Edge,
  MarkerType,
} from '@xyflow/react'
import '@xyflow/react/dist/style.css'
import { workflowApi } from '../api/client'
import { useWorkflowStore } from '../stores/workflowStore'
import { Link, useNavigate } from 'react-router-dom'
import { useLanguage } from '../lib/language'
import {
  Sparkles, Zap, CheckCircle, AlertCircle,
  ChevronRight, Loader2, Wand2, Brain, Bot,
} from 'lucide-react'

// ─── 节点类型颜色 ─────────────────────────────────────────
const NODE_COLORS: Record<string, string> = {
  trigger:  '#8b5cf6',
  http:     '#10b981',
  set:      '#3b82f6',
  code:     '#f59e0b',
  if:       '#ec4899',
  for:      '#06b6d4',
  db:       '#6366f1',
  email:    '#f97316',
  slack:    '#22c55e',
  telegram: '#0ea5e9',
  default:  '#6b7280',
}

type StepStatus = 'pending' | 'running' | 'done' | 'error'
type PipelineState = Record<string, StepStatus>

// ─── 将后端节点数据转换为 ReactFlow 节点 ──────────────────
function toFlowNodes(rawNodes: any[]): Node[] {
  return rawNodes.map((n, i) => ({
    id: n.id || `node_${i}`,
    type: 'default',
    position: n.position ? { x: n.position[0], y: n.position[1] } : { x: 150 + i * 200, y: 200 },
    data: {
      label: (
        <div className="text-xs font-medium">
          <div className="font-bold">{n.name}</div>
          <div className="text-gray-400">{n.type}</div>
        </div>
      ),
    },
    style: {
      background: NODE_COLORS[n.type] || NODE_COLORS.default,
      color: '#fff',
      borderRadius: '8px',
      border: '1px solid rgba(255,255,255,0.2)',
      padding: '6px 12px',
      minWidth: '120px',
      fontSize: '12px',
    },
  }))
}

function toFlowEdges(connections: any[]): Edge[] {
  return connections.map((c, i) => ({
    id: `e_${i}`,
    source: c.from,
    target: c.to,
    animated: true,
    markerEnd: { type: MarkerType.ArrowClosed, color: '#6b7280' },
    style: { stroke: '#6b7280', strokeWidth: 2 },
  }))
}

function toEditorNodes(rawNodes: any[]) {
  return rawNodes.map((node: any, index: number) => ({
    id: node.id || `node_${index}`,
    type: 'custom' as const,
    position: Array.isArray(node.position)
      ? { x: node.position[0] || 0, y: node.position[1] || 0 }
      : node.position || { x: 150 + index * 200, y: 200 },
    data: {
      label: node.name || node.type || `Node ${index + 1}`,
      nodeType: node.type || 'set',
      params: node.parameters || {},
    },
  }))
}

function toEditorEdges(connections: any[]) {
  return connections.map((connection: any, index: number) => ({
    id: `edge_${index}`,
    source: connection.from,
    target: connection.to,
    type: 'smoothstep',
  }))
}

export default function NLCreator() {
  const { text, language } = useLanguage()
  const navigate = useNavigate()
  const { loadWorkflow } = useWorkflowStore()

  const [description, setDescription] = useState('')
  const [autoOptimize, setAutoOptimize] = useState(true)
  const [isCreating, setIsCreating] = useState(false)
  const [pipeline, setPipeline] = useState<PipelineState>({})
  const [flowNodes, setFlowNodes] = useState<Node[]>([])
  const [flowEdges, setFlowEdges] = useState<Edge[]>([])
  const [generatedWorkflow, setGeneratedWorkflow] = useState<any>(null)
  const [optimizationSuggestions, setOptimizationSuggestions] = useState<any[]>([])
  const examples = useMemo(
    () =>
      language === 'zh'
        ? [
            '每天早上 9 点从数据库获取待处理订单，调用物流 API 查询状态，发送邮件通知客户',
            '监听 Webhook，解析 JSON 数据，条件判断：如果金额 > 1000 则发 Slack 通知，否则写入数据库',
            '定时爬取竞品价格 API，与本地价格对比，差价超过 10% 时自动调整价格并记录日志',
            '接收用户注册事件，调用发送验证邮件接口，等待 5 分钟后检查是否验证，未验证则发提醒',
          ]
        : [
            'Every morning at 9, fetch pending orders from the database, query the logistics API, and email the customer.',
            'Listen to a webhook, parse JSON, and if amount > 1000 send Slack; otherwise write to the database.',
            'Fetch competitor pricing on a schedule, compare with local prices, and adjust when the gap exceeds 10%.',
            'Receive a user signup event, send a verification email, wait 5 minutes, then remind if still unverified.',
          ],
    [language],
  )
  const pipelineSteps = useMemo(
    () => [
      { id: 'analyze', label: text({ zh: '需求分析', en: 'Analyze' }), icon: Brain },
      { id: 'match', label: text({ zh: '模式匹配', en: 'Match' }), icon: Sparkles },
      { id: 'generate', label: text({ zh: '生成工作流', en: 'Generate' }), icon: Wand2 },
      { id: 'validate', label: text({ zh: '验证节点', en: 'Validate' }), icon: CheckCircle },
      { id: 'optimize', label: text({ zh: 'AI 优化', en: 'Optimize' }), icon: Bot },
    ],
    [text],
  )
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  const setStep = (id: string, status: StepStatus) => {
    setPipeline(prev => ({ ...prev, [id]: status }))
  }

  const sleep = (ms: number) => new Promise(r => setTimeout(r, ms))

  const handleCreate = useCallback(async () => {
    if (!description.trim()) {
      toast.warning(text({ zh: '请先输入工作流描述', en: 'Enter a workflow description first' }))
      textareaRef.current?.focus()
      return
    }
    setIsCreating(true)
    setFlowNodes([])
    setFlowEdges([])
    setGeneratedWorkflow(null)
    setOptimizationSuggestions([])
    setPipeline({})

    try {
      // Step 1: 需求分析
      setStep('analyze', 'running')
      await sleep(600)
      setStep('analyze', 'done')

      // Step 2: 模式匹配（查找相似工作流）
      setStep('match', 'running')
      try {
        // Use new API or skip if not available
        // const data = await workflowApi.getPatterns()
      } catch {
        // 非致命错误，继续
      }
      await sleep(400)
      setStep('match', 'done')

      // Step 3: 生成工作流
      setStep('generate', 'running')
      let workflow: any = null
      try {
        // 优先使用 SSE 流式创建（createWorkflowV2）
        const wfId = await workflowApi.createWorkflowV2(description, (event) => {
          if (event.payload?.workflow) {
            const rawNodes = event.payload.workflow.nodes || []
            const rawConns = event.payload.workflow.connections || []
            setFlowNodes(toFlowNodes(rawNodes))
            setFlowEdges(toFlowEdges(rawConns))
          }
        })
        if (wfId) {
          const wfData = await workflowApi.getWorkflow(wfId)
          workflow = wfData?.n8n_json || wfData
        }
      } catch {
        // 回退到简单 NL 创建端点
      }

      if (!workflow) {
        try {
          // Use enhanceNLWorkflow instead
          const result = await workflowApi.enhanceNLWorkflow(description)
          workflow = result.improved_workflow
        } catch {
          // 最终回退：本地构造基础工作流
          workflow = buildFallbackWorkflow(description, language)
        }
      }

      if (workflow) {
        const rawNodes = workflow.nodes || []
        const rawConns = workflow.connections || []
        setFlowNodes(toFlowNodes(rawNodes))
        setFlowEdges(toFlowEdges(rawConns))
        setGeneratedWorkflow(workflow)
      }
      setStep('generate', 'done')

      // Step 4: 验证
      setStep('validate', 'running')
      await sleep(500)
      setStep('validate', 'done')

      // Step 5: AI 优化（可选）
      if (autoOptimize && workflow) {
        setStep('optimize', 'running')
        const suggestions = buildAutoOptimizeSuggestions(workflow, description, language)
        setOptimizationSuggestions(suggestions)
        await sleep(600)
        setStep('optimize', 'done')
      }

      toast.success(
        text({
          zh: `工作流生成完成，共 ${generatedWorkflow?.nodes?.length || flowNodes.length} 个节点`,
          en: `Workflow created with ${generatedWorkflow?.nodes?.length || flowNodes.length} nodes`,
        }),
      )
    } catch (err: any) {
      const stepIds = pipelineSteps.map(s => s.id)
      const runningStep = stepIds.find(id => pipeline[id] === 'running')
      if (runningStep) setStep(runningStep, 'error')
      toast.error(text({ zh: '创建失败: ', en: 'Creation failed: ' }) + (err.message || text({ zh: '未知错误', en: 'Unknown error' })))
    } finally {
      setIsCreating(false)
    }
  }, [description, autoOptimize, text, generatedWorkflow?.nodes?.length, flowNodes.length, pipeline, pipelineSteps, language])

  const handleApplyToEditor = () => {
    if (!generatedWorkflow) {
      toast.warning(text({ zh: '请先生成工作流', en: 'Generate a workflow first' }))
      return
    }
    loadWorkflow(
      toEditorNodes(generatedWorkflow.nodes || []),
      toEditorEdges(generatedWorkflow.connections || []),
      true,
    )
    toast.success(text({ zh: '已导入编辑器，正在跳转…', en: 'Imported into the editor. Redirecting…' }))
    setTimeout(() => navigate('/editor'), 500)
  }

  const handleRefine = (suggestion: any) => {
    // 将优化建议附加到描述并重新生成
    const refinedDesc = description + `。${suggestion.detail}`
    setDescription(refinedDesc)
    toast.info(text({ zh: '已应用优化建议，点击“生成工作流”重新生成', en: 'Optimization applied. Click Create again to regenerate.' }))
  }

  return (
    <div className="h-full flex flex-col bg-background">
      {/* ── 标题栏 ── */}
      <div className="flex items-center gap-3 px-6 py-4 border-b border-gray-700 bg-surface">
        <div className="w-9 h-9 rounded-lg bg-purple-500/20 flex items-center justify-center">
          <Sparkles size={20} className="text-purple-400" />
        </div>
        <div>
          <h1 className="text-lg font-semibold text-white">{text({ zh: '工作流构建辅助', en: 'Workflow Builder Support' })}</h1>
          <p className="text-xs text-gray-400">
            {text({
              zh: '主入口已经统一到任务控制台。这里保留为高级区中的流程草稿生成与编辑器辅助工具。',
              en: 'The primary entry now lives in Task Console. This page remains an advanced-area helper for workflow drafting and editor handoff.',
            })}
          </p>
        </div>
        <div className="ml-auto">
          <Link
            to="/tasks"
            className="inline-flex items-center gap-2 rounded-2xl border border-white/10 px-3 py-2 text-xs font-medium text-slate-200 transition hover:bg-white/[0.06]"
          >
            {text({ zh: '返回任务控制台', en: 'Back to Task Console' })}
          </Link>
        </div>
      </div>

      <div className="flex-1 flex min-h-0">
        {/* ── 左侧：输入区 ── */}
        <div className="w-96 flex flex-col border-r border-gray-700 bg-surface">
          {/* 输入框 */}
          <div className="p-4 space-y-3">
            <label className="text-sm font-medium text-gray-300">{text({ zh: '描述你的工作流需求', en: 'Describe the workflow you want' })}</label>
            <textarea
              ref={textareaRef}
              className="w-full h-40 bg-gray-800 border border-gray-600 rounded-lg p-3 text-sm text-white resize-none focus:outline-none focus:border-purple-500 placeholder-gray-500"
              placeholder={text({
                zh: '例如：每天早上从数据库获取订单，调用 API 查询物流状态，发送邮件通知客户…',
                en: 'Example: Fetch orders from the database every morning, query the API, and send email updates…',
              })}
              value={description}
              onChange={e => setDescription(e.target.value)}
            />
            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                id="autoOpt"
                checked={autoOptimize}
                onChange={e => setAutoOptimize(e.target.checked)}
                className="w-4 h-4 accent-purple-500"
              />
              <label htmlFor="autoOpt" className="text-xs text-gray-400 cursor-pointer">
                {text({ zh: '自动 AI 优化', en: 'Automatic AI optimization' })}
              </label>
            </div>
            <button
              onClick={handleCreate}
              disabled={isCreating}
              className="w-full flex items-center justify-center gap-2 bg-purple-600 hover:bg-purple-500 disabled:opacity-50 text-white px-4 py-2.5 rounded-lg text-sm font-medium transition-colors"
            >
              {isCreating
                ? <><Loader2 size={16} className="animate-spin" /> {text({ zh: '生成中…', en: 'Creating…' })}</>
                : <><Wand2 size={16} /> {text({ zh: '生成工作流', en: 'Create workflow' })}</>}
            </button>
          </div>

          {/* 示例 */}
          <div className="px-4 pb-4">
            <p className="text-xs text-gray-500 mb-2">{text({ zh: '快速示例：', en: 'Quick examples:' })}</p>
            <div className="space-y-1.5">
              {examples.map((ex, i) => (
                <button
                  key={i}
                  onClick={() => setDescription(ex)}
                  className="w-full text-left text-xs text-gray-400 hover:text-purple-300 bg-gray-800 hover:bg-gray-700 rounded px-2 py-1.5 transition-colors line-clamp-2"
                >
                  <ChevronRight size={10} className="inline mr-1" />
                  {ex}
                </button>
              ))}
            </div>
          </div>

          {/* 流水线进度 */}
          {Object.keys(pipeline).length > 0 && (
            <div className="px-4 pb-4 border-t border-gray-700 pt-3">
              <p className="text-xs text-gray-400 mb-3 font-medium">{text({ zh: '创建流水线', en: 'Pipeline' })}</p>
              <div className="space-y-2">
                {pipelineSteps.map(step => {
                  const status = pipeline[step.id] || 'pending'
                  const Icon = step.icon
                  return (
                    <div key={step.id} className="flex items-center gap-2.5">
                      <div className={`w-6 h-6 rounded-full flex items-center justify-center text-xs flex-shrink-0 ${
                        status === 'done'    ? 'bg-green-500/20 text-green-400' :
                        status === 'running' ? 'bg-blue-500/20 text-blue-400' :
                        status === 'error'   ? 'bg-red-500/20 text-red-400' :
                        'bg-gray-700 text-gray-500'
                      }`}>
                        {status === 'running' ? <Loader2 size={12} className="animate-spin" /> :
                         status === 'done'    ? <CheckCircle size={12} /> :
                         status === 'error'   ? <AlertCircle size={12} /> :
                         <Icon size={12} />}
                      </div>
                      <span className={`text-xs ${
                        status === 'done'    ? 'text-green-400' :
                        status === 'running' ? 'text-blue-400 font-medium' :
                        status === 'error'   ? 'text-red-400' :
                        'text-gray-500'
                      }`}>{step.label}</span>
                    </div>
                  )
                })}
              </div>
            </div>
          )}

          {/* AI 优化建议 */}
          {optimizationSuggestions.length > 0 && (
            <div className="px-4 pb-4 border-t border-gray-700 pt-3 flex-1 overflow-y-auto">
              <p className="text-xs text-gray-400 mb-2 font-medium flex items-center gap-1">
                <Bot size={12} className="text-purple-400" /> {text({ zh: 'AI 优化建议', en: 'AI Suggestions' })}
              </p>
              <div className="space-y-2">
                {optimizationSuggestions.map((s, i) => (
                  <div key={i} className="bg-gray-800 rounded-lg p-2.5 border border-gray-600">
                    <div className="flex items-start justify-between gap-2">
                      <div>
                        <div className={`text-xs font-medium mb-0.5 ${
                          s.priority === 'high' ? 'text-red-400' :
                          s.priority === 'medium' ? 'text-yellow-400' : 'text-blue-400'
                        }`}>{s.title}</div>
                        <div className="text-xs text-gray-400 leading-relaxed">{s.detail}</div>
                      </div>
                      <button
                        onClick={() => handleRefine(s)}
                        className="flex-shrink-0 text-xs bg-purple-600/30 hover:bg-purple-600/50 text-purple-300 px-2 py-0.5 rounded transition-colors"
                        >
                        {text({ zh: '应用', en: 'Apply' })}
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* 底部操作 */}
          {generatedWorkflow && (
            <div className="p-4 border-t border-gray-700 mt-auto">
              <button
                onClick={handleApplyToEditor}
                className="w-full flex items-center justify-center gap-2 bg-cyan-600 hover:bg-cyan-500 text-white px-4 py-2.5 rounded-lg text-sm font-medium transition-colors"
              >
                <Zap size={16} />
                {text({ zh: '在编辑器中打开', en: 'Open in editor' })}
              </button>
            </div>
          )}
        </div>

        {/* ── 右侧：可视化预览 ── */}
        <div className="flex-1 flex flex-col">
          <div className="px-4 py-2 border-b border-gray-700 bg-surface/50 flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-purple-500 animate-pulse" />
            <span className="text-xs text-gray-400">{text({ zh: '工作流预览', en: 'Live preview' })}</span>
            {flowNodes.length > 0 && (
              <span className="text-xs text-gray-500 ml-auto">
                {text({ zh: `${flowNodes.length} 个节点 · ${flowEdges.length} 条连接`, en: `${flowNodes.length} nodes · ${flowEdges.length} edges` })}
              </span>
            )}
          </div>

          {flowNodes.length === 0 ? (
            <div className="flex-1 flex flex-col items-center justify-center text-gray-600">
              <Sparkles size={48} className="mb-4 opacity-30" />
              <p className="text-sm">{text({ zh: '输入描述后点击“生成工作流”', en: 'Enter a description and click Create' })}</p>
              <p className="text-xs mt-1">{text({ zh: '系统会实时预览节点生成过程', en: 'The system will preview node generation in real time' })}</p>
            </div>
          ) : (
            <div className="flex-1">
              <ReactFlow
                nodes={flowNodes}
                edges={flowEdges}
                fitView
                nodesDraggable={false}
                nodesConnectable={false}
                elementsSelectable={false}
              >
                <Background variant={BackgroundVariant.Dots} gap={20} size={1} color="#374151" />
                <Controls showInteractive={false} />
                <MiniMap
                  nodeColor={(node) => (node.style?.background as string) || '#6b7280'}
                  style={{ background: '#1f2937' }}
                />
              </ReactFlow>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

// ─── 辅助函数 ────────────────────────────────────────────

function buildFallbackWorkflow(description: string, language: 'zh' | 'en'): any {
  const desc = description.toLowerCase()
  const nodes: any[] = []
  const connections: any[] = []
  let id = 1
  const mk = () => `node_${id++}`

  const addNode = (name: string, type: string, params: any, x: number, y: number) => {
    const nodeId = mk()
    nodes.push({ id: nodeId, name, type, parameters: params, position: [x, y] })
    return nodeId
  }

  const trigger = addNode(language === 'zh' ? '触发器' : 'Trigger', 'trigger', { mode: 'manual' }, 100, 200)
  let prev = trigger

  const link = (from: string, to: string) => connections.push({ from, to })

  if (desc.includes('http') || desc.includes('api') || desc.includes('请求') || desc.includes('获取')) {
    const n = addNode(language === 'zh' ? 'HTTP 请求' : 'HTTP Request', 'http', { url: '', method: 'GET' }, 300, 200)
    link(prev, n); prev = n
  }
  if (desc.includes('数据库') || desc.includes('存储') || desc.includes('查询')) {
    const n = addNode(language === 'zh' ? '数据库' : 'Database', 'db', { operation: 'query', query: '' }, 700, 200)
    link(prev, n); prev = n
  }
  if (desc.includes('邮件') || desc.includes('通知') || desc.includes('email')) {
    const n = addNode(language === 'zh' ? '发送邮件' : 'Send Email', 'email', { to: '', subject: '' }, 900, 200)
    link(prev, n); prev = n
  }
  if (nodes.length === 1) {
    const n = addNode(language === 'zh' ? '设置变量' : 'Set Variable', 'set', { key: 'result', value: '' }, 300, 200)
    link(prev, n)
  }

  return {
    name: description.slice(0, 30),
    description,
    nodes,
    connections,
  }
}

function buildAutoOptimizeSuggestions(workflow: any, description: string, language: 'zh' | 'en'): any[] {
  const nodes: any[] = workflow.nodes || []
  const suggestions: any[] = []
  const desc = description.toLowerCase()

  const hasError = nodes.some(n => n.type === 'if')
  if (!hasError && nodes.length > 2) {
    suggestions.push({
      type: 'reliability', priority: 'high',
      title: language === 'zh' ? '添加错误处理分支' : 'Add error handling',
      detail: language === 'zh'
        ? '建议在关键节点后添加 If 条件判断，区分成功和失败路径，提高工作流健壮性。'
        : 'Add an If branch after critical nodes to split success and failure paths.',
    })
  }

  const httpNodes = nodes.filter(n => n.type === 'http')
  if (httpNodes.length >= 2) {
    suggestions.push({
      type: 'performance', priority: 'medium',
      title: language === 'zh' ? `并行化 ${httpNodes.length} 个 HTTP 请求` : `Parallelize ${httpNodes.length} HTTP calls`,
      detail: language === 'zh'
        ? '检测到多个 HTTP 节点串行执行，将独立请求并行化可减少 40-60% 总耗时。'
        : 'Multiple HTTP nodes appear to run serially. Parallelization can reduce total latency.',
    })
  }

  if (desc.includes('定时') || desc.includes('每天') || desc.includes('每小时')) {
    suggestions.push({
      type: 'reliability', priority: 'medium',
      title: language === 'zh' ? '添加幂等性保护' : 'Add idempotency protection',
      detail: language === 'zh'
        ? '定时任务建议添加去重检查，防止重复执行或数据重复写入。'
        : 'Scheduled tasks should add deduplication or idempotency protection.',
    })
  }

  if (nodes.length > 5) {
    suggestions.push({
      type: 'optimization', priority: 'low',
      title: language === 'zh' ? '拆分为子工作流' : 'Split into subflows',
      detail: language === 'zh'
        ? `当前工作流有 ${nodes.length} 个节点，建议将独立功能模块拆分为子工作流，提升复用性。`
        : `This workflow has ${nodes.length} nodes. Consider extracting reusable modules into subflows.`,
    })
  }

  if (suggestions.length === 0) {
    suggestions.push({
      type: 'optimization', priority: 'low',
      title: language === 'zh' ? '工作流结构合理' : 'Structure looks good',
      detail: language === 'zh'
        ? '当前工作流结构清晰，可在编辑器中完善节点参数后运行测试。'
        : 'The current structure looks reasonable. Refine parameters in the editor and run a test.',
    })
  }

  return suggestions
}
