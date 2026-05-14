import { useState } from 'react'
import { toast } from 'sonner'
import { 
  ReactFlow, 
  Background, 
  Controls, 
  MiniMap,
  BackgroundVariant,
  type NodeTypes,
} from '@xyflow/react'
import '@xyflow/react/dist/style.css'
import { useWorkflowStore } from '../stores/workflowStore'
import { workflowApi } from '../api/client'
import NodePalette from './NodePalette'
import PropertyPanel from './PropertyPanel'
import CustomNode from './CustomNode'
import DiffViewer from './DiffViewer'
import AutoTuner from './AutoTuner'
import LearningReport from './LearningReport'
import NLWizard from './NLWizard'
import { Save, Play, Download, Upload, FolderOpen, Settings, GitCompare, Sparkles, Bot, Zap, BookOpen } from 'lucide-react'
import { Link } from 'react-router-dom'
import { useLanguage } from '../lib/language'
import type { NodeType, WorkflowNode } from '../stores/workflowStore'

const nodeTypes: NodeTypes = {
  custom: CustomNode,
}

export default function WorkflowEditor() {
  const { text } = useLanguage()
  const { 
    nodes, 
    edges, 
    onNodesChange, 
    onEdgesChange, 
    onConnect,
    addNode,
    exportWorkflow,
    importWorkflow,
    setWorkflows,
    setExecutionResult,
    isExecuting,
    setIsExecuting,
    originalNodes,
    originalEdges,
    isFromAI,
    currentWorkflowId,
    setCurrentWorkflowId,
  } = useWorkflowStore()

  const [showDiffViewer, setShowDiffViewer] = useState(false)
  const [showAIOptimizer, setShowAIOptimizer] = useState(false)
  const [showAutoTuner, setShowAutoTuner] = useState(false)
  const [showLearningReport, setShowLearningReport] = useState(false)
  const [showNLWizard, setShowNLWizard] = useState(false)

  const handleDragOver = (event: React.DragEvent) => {
    event.preventDefault()
    event.dataTransfer.dropEffect = 'move'
  }

  const handleDrop = (event: React.DragEvent) => {
    event.preventDefault()
    const nodeType = event.dataTransfer.getData('application/reactflow') as import('../stores/workflowStore').NodeType
    if (!nodeType) {
      return
    }

    const reactFlowBounds = document.querySelector('.react-flow')?.getBoundingClientRect()
    if (!reactFlowBounds) {
      return
    }

    addNode(nodeType, {
      x: event.clientX - reactFlowBounds.left - 90,
      y: event.clientY - reactFlowBounds.top - 30,
    })
  }

  const handleSave = async () => {
    try {
      const n8nJson = convertToN8nFormat(nodes, edges)
      const result = await workflowApi.compile(n8nJson, `workflow_${Date.now()}`)
      setCurrentWorkflowId(result.workflow_id)
      toast.success(text({ zh: '工作流已保存', en: 'Workflow saved' }))
    } catch (error) {
      console.error('Failed to save workflow:', error)
      toast.error(text({ zh: '保存工作流失败', en: 'Failed to save workflow' }))
    }
  }

  const handleSaveDiff = async (diffs: any[]) => {
    if (!currentWorkflowId) {
      toast.warning(text({ zh: '请先保存工作流，再保存差异版本', en: 'Save the workflow before saving changes' }))
      return
    }
    try {
      const n8nJson = convertToN8nFormat(nodes, edges)
      await workflowApi.saveWorkflowDiff(currentWorkflowId, n8nJson, diffs)
      toast.success(text({ zh: '差异版本已保存', en: 'Changes saved as a new version' }))
    } catch (error) {
      console.error('Failed to save diff:', error)
      toast.error(text({ zh: '保存差异失败', en: 'Failed to save changes' }))
    }
  }

  const handleExecute = async () => {
    setIsExecuting(true)
    setExecutionResult(null)
    try {
      const n8nJson = convertToN8nFormat(nodes, edges)
      const compiled = await workflowApi.compile(n8nJson)
      const result = await workflowApi.execute(compiled.workflow_id)
      setExecutionResult(result)
    } catch (error: any) {
      setExecutionResult({ error: error.message || text({ zh: '执行失败', en: 'Execution failed' }) })
    } finally {
      setIsExecuting(false)
    }
  }

  const handleExport = () => {
    const json = exportWorkflow()
    const blob = new Blob([json], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `workflow_${Date.now()}.json`
    a.click()
    URL.revokeObjectURL(url)
  }

  const handleImport = () => {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = '.json'
    input.onchange = (e) => {
      const file = (e.target as HTMLInputElement).files?.[0]
      if (file) {
        const reader = new FileReader()
        reader.onload = (e) => {
          importWorkflow(e.target?.result as string)
        }
        reader.readAsText(file)
      }
    }
    input.click()
  }

  const handleLoadWorkflows = async () => {
    try {
      const workflows = await workflowApi.listWorkflows()
      setWorkflows(workflows)
    } catch (error) {
      console.error('Failed to load workflows:', error)
    }
  }

  return (
    <div className="h-screen flex flex-col">
      <header className="h-14 bg-surface border-b border-gray-700 flex items-center px-4 gap-4">
        <h1 className="text-lg font-semibold text-primary">
          {text({ zh: 'MemFlow 工作流编辑器', en: 'MemFlow Workflow Editor' })}
        </h1>
        
        <div className="flex-1" />
        
        <div className="flex items-center gap-2">
          <button 
            onClick={handleSave}
            className="btn btn-secondary flex items-center gap-2"
          >
            <Save size={16} />
            {text({ zh: '保存', en: 'Save' })}
          </button>
          
          <button 
            onClick={handleExecute}
            disabled={isExecuting}
            className="btn btn-primary flex items-center gap-2"
          >
            <Play size={16} />
            {isExecuting ? text({ zh: '执行中…', en: 'Executing…' }) : text({ zh: '执行', en: 'Execute' })}
          </button>
          
          <div className="w-px h-6 bg-gray-600 mx-2" />
          
          <button 
            onClick={handleExport}
            className="btn btn-secondary flex items-center gap-2"
          >
            <Download size={16} />
            {text({ zh: '导出', en: 'Export' })}
          </button>
          
          <button 
            onClick={handleImport}
            className="btn btn-secondary flex items-center gap-2"
          >
            <Upload size={16} />
            {text({ zh: '导入', en: 'Import' })}
          </button>
          
          <button 
            onClick={handleLoadWorkflows}
            className="btn btn-secondary flex items-center gap-2"
          >
            <FolderOpen size={16} />
            {text({ zh: '加载', en: 'Load' })}
          </button>
          
          {isFromAI && originalNodes.length > 0 && (
            <button 
              onClick={() => setShowDiffViewer(true)}
              className="btn btn-secondary flex items-center gap-2"
            >
              <GitCompare size={16} />
              {text({ zh: '差异', en: 'Diff' })}
            </button>
          )}
          
          <div className="w-px h-6 bg-gray-600 mx-1" />

          <Link
            to="/create"
            className="btn btn-secondary flex items-center gap-2"
            title={text({ zh: '自然语言创建工作流', en: 'Create workflow from natural language' })}
          >
            <Sparkles size={16} />
            {text({ zh: '创建', en: 'Create' })}
          </Link>

          <button
            onClick={() => setShowAIOptimizer(v => !v)}
            className={`btn flex items-center gap-2 ${showAIOptimizer ? 'btn-primary' : 'btn-secondary'}`}
            title={text({ zh: 'AI 优化建议', en: 'AI optimization hints' })}
          >
            <Bot size={16} />
            {text({ zh: '优化', en: 'Optimize' })}
          </button>

          <button
            onClick={() => setShowAutoTuner(v => !v)}
            disabled={!currentWorkflowId}
            className={`btn flex items-center gap-2 ${showAutoTuner ? 'btn-primary' : 'btn-secondary'}`}
            title={text({ zh: '自动参数调整', en: 'Automatic parameter tuning' })}
          >
            <Zap size={16} />
            {text({ zh: '调参', en: 'Tune' })}
          </button>

          <button
            onClick={() => setShowLearningReport(v => !v)}
            disabled={!currentWorkflowId}
            className={`btn flex items-center gap-2 ${showLearningReport ? 'btn-primary' : 'btn-secondary'}`}
            title={text({ zh: '学习报告与优化建议', en: 'Learning report and hints' })}
          >
            <BookOpen size={16} />
            {text({ zh: '学习', en: 'Learn' })}
          </button>

          <button
            onClick={() => setShowNLWizard(v => !v)}
            className={`btn flex items-center gap-2 ${showNLWizard ? 'btn-primary' : 'btn-secondary'}`}
            title={text({ zh: '自然语言增强', en: 'Natural language enhancement' })}
          >
            <Sparkles size={16} />
            {text({ zh: '增强', en: 'Enhance' })}
          </button>

          <Link
            to="/computer"
            className="btn btn-secondary flex items-center gap-2"
            title={text({ zh: '电脑操作代理', en: 'Computer operation agent' })}
          >
            <Bot size={16} />
            {text({ zh: '电脑代理', en: 'Computer' })}
          </Link>

          <Link
            to="/settings"
            className="btn btn-secondary flex items-center gap-2"
            title={text({ zh: '统一设置入口', en: 'Unified settings entry' })}
          >
            <Settings size={16} />
            {text({ zh: '设置', en: 'Settings' })}
          </Link>
        </div>
      </header>

      <div className="flex-1 flex">
        <div className="p-2">
          <NodePalette />
        </div>

        <div className="flex-1 bg-background">
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            onDragOver={handleDragOver}
            onDrop={handleDrop}
            nodeTypes={nodeTypes}
            fitView
            snapToGrid
            snapGrid={[15, 15]}
          >
            <Background variant={BackgroundVariant.Dots} gap={15} size={1} />
            <Controls />
            <MiniMap 
              nodeColor={(node) => {
                const colors: Record<string, string> = {
                  http: '#4CAF50',
                  code: '#FF9800',
                  if: '#E91E63',
                  set: '#2196F3',
                }
                return colors[node.data?.nodeType as string] || '#666'
              }}
            />
          </ReactFlow>
        </div>

        <div className="p-2 w-80 bg-surface border-l border-gray-700 overflow-y-auto space-y-4">
          {!showAutoTuner && !showLearningReport && !showNLWizard && (
            <PropertyPanel />
          )}

          {showAutoTuner && currentWorkflowId && (
            <AutoTuner 
              workflowId={currentWorkflowId}
              onApplyTuning={() => {
                toast.success(text({ zh: '参数已应用', en: 'Parameters tuned successfully' }))
              }}
            />
          )}

          {showLearningReport && currentWorkflowId && (
            <LearningReport 
              workflowId={currentWorkflowId}
              onClose={() => setShowLearningReport(false)}
            />
          )}

          {showNLWizard && (
            <NLWizard 
              workflowId={currentWorkflowId || undefined}
              onGenerated={() => {
                toast.success(text({ zh: '工作流增强完成', en: 'Workflow enhanced successfully' }))
                setShowNLWizard(false)
              }}
            />
          )}
        </div>
      </div>

      {showDiffViewer && (
        <DiffViewer
          originalNodes={originalNodes}
          originalEdges={originalEdges}
          onApplyPatch={handleSaveDiff}
          onClose={() => setShowDiffViewer(false)}
        />
      )}
    </div>
  )
}

function convertToN8nFormat(nodes: any[], edges: any[]): any {
  const n8nNodes = nodes.map((node) => {
    const workflowNode = node as WorkflowNode
    const compiledNode = toCompilerNode(workflowNode.data.nodeType, workflowNode.data.params)

    return {
      id: workflowNode.id,
      name: String(workflowNode.data.label),
      type: compiledNode.type,
      parameters: compiledNode.parameters,
      position: [workflowNode.position.x, workflowNode.position.y],
      inputs: edges.filter((e) => e.target === workflowNode.id).map((e) => e.source),
      outputs: edges.filter((e) => e.source === workflowNode.id).map((e) => e.target),
    }
  })

  return {
    name: 'MemFlow Workflow',
    nodes: n8nNodes,
    connections: edges.map((e) => ({
      from: e.source,
      to: e.target,
    })),
  }
}

function toCompilerNode(nodeType: NodeType, params: Record<string, unknown>): { type: string; parameters: Record<string, unknown> } {
  switch (nodeType) {
    case 'trigger':
      return { type: 'trigger', parameters: {} }
    case 'http':
      return {
        type: 'httpRequest',
        parameters: (() => {
      const headers = params.headers
      const headerEntries = headers && typeof headers === 'object' && !Array.isArray(headers)
        ? Object.entries(headers as Record<string, unknown>).map(([key, value]) => [key, String(value)])
        : []
          return {
            url: params.url || '',
            method: params.method || 'GET',
            headers: headerEntries,
            response_format: 'response',
            body: params.body || undefined,
          }
        })(),
      }
    case 'set': {
      const key = typeof params.name === 'string' && params.name.trim() ? params.name.trim() : 'result'
      return {
        type: 'set',
        parameters: {
          values: {
            [key]: params.value ?? '',
          },
        },
      }
    }
    case 'db':
      return {
        type: 'postgres',
        parameters: {
          connection: params.connection || 'default',
          query: params.query || '',
          params: [],
        },
      }
    case 'file': {
      const operation = params.operation === 'write' || params.operation === 'append' ? 'writeFile' : 'readFile'
      if (operation === 'writeFile') {
        return {
          type: 'writeFile',
          parameters: {
            path: params.path || '',
            content: params.content ?? '',
            append: params.operation === 'append',
          },
        }
      }
      return {
        type: 'readFile',
        parameters: {
          path: params.path || '',
        },
      }
    }
    case 'email':
      return {
        type: 'email',
        parameters: {
          to: params.to || '',
          subject: params.subject || '',
          body: params.body || '',
          smtp_config: params.smtp_config || 'default',
        },
      }
    default:
      throw new Error(`Unsupported node type for compiler: ${nodeType}`)
  }
}
