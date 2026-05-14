import { create } from 'zustand'
import { 
  Node as FlowNode,
  Edge as FlowEdge,
  Connection, 
  addEdge, 
  applyNodeChanges, 
  applyEdgeChanges,
  NodeChange,
  EdgeChange
} from '@xyflow/react'

export type NodeType = 
  | 'http' 
  | 'set' 
  | 'code' 
  | 'if' 
  | 'for' 
  | 'db' 
  | 'file' 
  | 'email'
  | 'slack'
  | 'telegram'
  | 'sheets'
  | 'github'
  | 'notion'
  | 'trigger'

export interface WorkflowNodeData extends Record<string, unknown> {
  label: string
  nodeType: NodeType
  params: Record<string, unknown>
  outputVar?: string
}

export type WorkflowNode = FlowNode<WorkflowNodeData, 'custom'>
export type WorkflowEdge = FlowEdge

export interface WorkflowState {
  nodes: WorkflowNode[]
  edges: WorkflowEdge[]
  selectedNode: WorkflowNode | null
  workflows: { id: string; name: string; version: number }[]
  currentWorkflowId: string | null
  executionResult: unknown | null
  isExecuting: boolean
  originalNodes: WorkflowNode[]
  originalEdges: WorkflowEdge[]
  isFromAI: boolean
  
  setNodes: (nodes: WorkflowNode[]) => void
  setEdges: (edges: WorkflowEdge[]) => void
  onNodesChange: (changes: NodeChange<WorkflowNode>[]) => void
  onEdgesChange: (changes: EdgeChange<WorkflowEdge>[]) => void
  onConnect: (connection: Connection) => void
  addNode: (type: NodeType, position: { x: number; y: number }) => void
  updateNodeParams: (nodeId: string, params: Record<string, unknown>) => void
  deleteNode: (nodeId: string) => void
  selectNode: (node: WorkflowNode | null) => void
  setWorkflows: (workflows: { id: string; name: string; version: number }[]) => void
  setCurrentWorkflowId: (id: string | null) => void
  setExecutionResult: (result: unknown | null) => void
  setIsExecuting: (executing: boolean) => void
  loadWorkflow: (nodes: WorkflowNode[], edges: WorkflowEdge[], fromAI?: boolean) => void
  setOriginalWorkflow: (nodes: WorkflowNode[], edges: WorkflowEdge[]) => void
  exportWorkflow: () => string
  importWorkflow: (json: string) => void
}

const nodeLabels: Record<NodeType, string> = {
  http: 'HTTP Request',
  set: 'Set Variable',
  code: 'Code',
  if: 'If Condition',
  for: 'For Loop',
  db: 'Database Query',
  file: 'File Operations',
  email: 'Send Email',
  slack: 'Slack Message',
  telegram: 'Telegram Message',
  sheets: 'Google Sheets',
  github: 'GitHub',
  notion: 'Notion',
  trigger: 'Trigger',
}

const defaultParams: Record<NodeType, Record<string, unknown>> = {
  http: { method: 'GET', url: '', headers: {}, body: '' },
  set: { name: '', value: '' },
  code: { script: '' },
  if: { condition: '' },
  for: { iterator: '', start: 0, end: 10, step: 1 },
  db: { query: '' },
  file: { operation: 'read', path: '' },
  email: { to: '', subject: '', body: '' },
  slack: { channel: '', text: '' },
  telegram: { chat_id: '', text: '' },
  sheets: { operation: 'read', spreadsheet_id: '', range: 'Sheet1!A1' },
  github: { operation: 'create_issue', owner: '', repo: '', title: '', body: '' },
  notion: { operation: 'create_page', database_id: '', properties: {} },
  trigger: { type: 'manual', cron: '' },
}

export const useWorkflowStore = create<WorkflowState>((set, get) => ({
  nodes: [],
  edges: [],
  selectedNode: null,
  workflows: [],
  currentWorkflowId: null,
  executionResult: null,
  isExecuting: false,
  originalNodes: [],
  originalEdges: [],
  isFromAI: false,

  setNodes: (nodes) => set({ nodes }),
  setEdges: (edges) => set({ edges }),
  
  onNodesChange: (changes) => {
    set({
      nodes: applyNodeChanges<WorkflowNode>(changes, get().nodes),
    })
  },
  
  onEdgesChange: (changes) => {
    set({
      edges: applyEdgeChanges<WorkflowEdge>(changes, get().edges),
    })
  },
  
  onConnect: (connection) => {
    set({
      edges: addEdge({ ...connection, type: 'smoothstep' }, get().edges) as WorkflowEdge[],
    })
  },
  
  addNode: (type, position) => {
    const id = `node_${Date.now()}`
    const newNode: WorkflowNode = {
      id,
      type: 'custom',
      position,
      data: {
        label: nodeLabels[type],
        nodeType: type,
        params: { ...defaultParams[type] },
      },
    }
    set({ nodes: [...get().nodes, newNode] })
  },
  
  updateNodeParams: (nodeId, params) => {
    set({
      nodes: get().nodes.map((node) =>
        node.id === nodeId
          ? {
              ...node,
              data: {
                ...node.data,
                params: {
                  ...node.data.params,
                  ...params,
                },
              },
            }
          : node
      ),
    })
  },
  
  deleteNode: (nodeId) => {
    set({
      nodes: get().nodes.filter((node) => node.id !== nodeId),
      edges: get().edges.filter(
        (edge) => edge.source !== nodeId && edge.target !== nodeId
      ),
      selectedNode: get().selectedNode?.id === nodeId ? null : get().selectedNode,
    })
  },
  
  selectNode: (node) => set({ selectedNode: node }),
  
  setWorkflows: (workflows) => set({ workflows }),
  setCurrentWorkflowId: (id) => set({ currentWorkflowId: id }),
  setExecutionResult: (result) => set({ executionResult: result }),
  setIsExecuting: (executing) => set({ isExecuting: executing }),

  loadWorkflow: (nodes, edges, fromAI = false) => set({ 
    nodes, 
    edges, 
    selectedNode: null,
    originalNodes: nodes,
    originalEdges: edges,
    isFromAI: fromAI,
  }),
  
  setOriginalWorkflow: (nodes, edges) => set({ originalNodes: nodes, originalEdges: edges }),
  
  exportWorkflow: () => {
    const { nodes, edges } = get()
    return JSON.stringify({ nodes, edges }, null, 2)
  },
  
  importWorkflow: (json) => {
    try {
      const { nodes, edges } = JSON.parse(json) as {
        nodes: WorkflowNode[]
        edges: WorkflowEdge[]
      }
      set({ nodes, edges, selectedNode: null })
    } catch (e) {
      console.error('Failed to import workflow:', e)
    }
  },
}))
