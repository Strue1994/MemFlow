import { useCallback, useState } from 'react'
import { 
  ReactFlow, 
  Background, 
  Controls, 
  MiniMap,
  BackgroundVariant,
} from '@xyflow/react'
import '@xyflow/react/dist/style.css'
import { useWorkflowStore } from '../stores/workflowStore'
import { workflowApi } from '../api/client'
import NodePalette from './NodePalette'
import PropertyPanel from './PropertyPanel'
import CustomNode from './CustomNode'
import DiffViewer from './DiffViewer'
import { Save, Play, Download, Upload, FolderOpen, Settings, GitCompare } from 'lucide-react'

const nodeTypes = {
  custom: CustomNode,
}

export default function WorkflowEditor() {
  const { 
    nodes, 
    edges, 
    onNodesChange, 
    onEdgesChange, 
    onConnect,
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

  const [showSettings, setShowSettings] = useState(false)
  const [apiKey, setApiKey] = useState(localStorage.getItem('apiKey') || '')
  const [showDiffViewer, setShowDiffViewer] = useState(false)

  const handleSave = async () => {
    try {
      const n8nJson = convertToN8nFormat(nodes, edges)
      const result = await workflowApi.compile(n8nJson, `workflow_${Date.now()}`)
      setCurrentWorkflowId(result.workflow_id)
      alert('Workflow saved!')
    } catch (error) {
      console.error('Failed to save workflow:', error)
      alert('Failed to save workflow')
    }
  }

  const handleSaveDiff = async (diffs: any[]) => {
    if (!currentWorkflowId) {
      alert('Please save the workflow first before saving changes')
      return
    }
    try {
      const n8nJson = convertToN8nFormat(nodes, edges)
      await workflowApi.saveWorkflowDiff(currentWorkflowId, n8nJson, diffs)
      alert('Changes saved as new version!')
    } catch (error) {
      console.error('Failed to save diff:', error)
      alert('Failed to save changes')
    }
  }

  const handleExecute = async () => {
    setIsExecuting(true)
    setExecutionResult(null)
    try {
      const workflowJson = exportWorkflow()
      const n8nJson = convertToN8nFormat(nodes, edges)
      const compiled = await workflowApi.compile(n8nJson)
      const result = await workflowApi.execute(compiled.workflow_id)
      setExecutionResult(result)
    } catch (error: any) {
      setExecutionResult({ error: error.message || 'Execution failed' })
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

  const handleApiKeySave = () => {
    localStorage.setItem('apiKey', apiKey)
    setShowSettings(false)
  }

  return (
    <div className="h-screen flex flex-col">
      <header className="h-14 bg-surface border-b border-gray-700 flex items-center px-4 gap-4">
        <h1 className="text-lg font-semibold text-primary">MemFlow: 记忆驱动的自动化流</h1>
        
        <div className="flex-1" />
        
        <div className="flex items-center gap-2">
          <button 
            onClick={handleSave}
            className="btn btn-secondary flex items-center gap-2"
          >
            <Save size={16} />
            Save
          </button>
          
          <button 
            onClick={handleExecute}
            disabled={isExecuting}
            className="btn btn-primary flex items-center gap-2"
          >
            <Play size={16} />
            {isExecuting ? 'Executing...' : 'Execute'}
          </button>
          
          <div className="w-px h-6 bg-gray-600 mx-2" />
          
          <button 
            onClick={handleExport}
            className="btn btn-secondary flex items-center gap-2"
          >
            <Download size={16} />
            Export
          </button>
          
          <button 
            onClick={handleImport}
            className="btn btn-secondary flex items-center gap-2"
          >
            <Upload size={16} />
            Import
          </button>
          
          <button 
            onClick={handleLoadWorkflows}
            className="btn btn-secondary flex items-center gap-2"
          >
            <FolderOpen size={16} />
            Load
          </button>
          
          {isFromAI && originalNodes.length > 0 && (
            <button 
              onClick={() => setShowDiffViewer(true)}
              className="btn btn-secondary flex items-center gap-2"
            >
              <GitCompare size={16} />
              Diff
            </button>
          )}
          
          <button 
            onClick={() => setShowSettings(!showSettings)}
            className={`btn ${showSettings ? 'btn-primary' : 'btn-secondary'}`}
          >
            <Settings size={16} />
          </button>
        </div>
      </header>

      {showSettings && (
        <div className="bg-surface border-b border-gray-700 p-4">
          <div className="flex items-center gap-4">
            <label className="text-sm text-gray-400">API Key:</label>
            <input
              type="password"
              className="input w-80"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="Enter your API key"
            />
            <button onClick={handleApiKeySave} className="btn btn-primary">
              Save
            </button>
          </div>
        </div>
      )}

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

        <div className="p-2">
          <PropertyPanel />
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
  const n8nNodes = nodes.map((node, index) => ({
    id: node.id,
    name: node.data.label,
    type: node.data.nodeType,
    parameters: node.data.params,
    position: node.position,
    inputs: edges.filter((e) => e.target === node.id).map((e) => e.source),
    outputs: edges.filter((e) => e.source === node.id).map((e) => e.target),
  }))

  return {
    name: 'MemFlow: 记忆驱动的自动化流',
    nodes: n8nNodes,
    connections: edges.map((e) => ({
      from: e.source,
      to: e.target,
    })),
  }
}
