import { create } from 'zustand'
import { 
  Node, 
  Edge, 
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

export interface WorkflowNodeData {
  label: string
  nodeType: NodeType
  params: Record<string, any>
  outputVar?: string
}

export interface WorkflowState {
  nodes: Node[]
  edges: Edge[]
  selectedNode: Node | null
  workflows: { id: string; name: string; version: number }[]
  currentWorkflowId: string | null
  executionResult: any | null
  isExecuting: boolean
  originalNodes: Node[]
  originalEdges: Edge[]
  isFromAI: boolean
  
  setNodes: (nodes: Node[]) => void
  setEdges: (edges: Edge[]) => void
  onNodesChange: (changes: NodeChange[]) => void
  onEdgesChange: (changes: EdgeChange[]) => void
  onConnect: (connection: Connection) => void
  addNode: (type: NodeType, position: { x: number; y: number }) => void
  updateNodeParams: (nodeId: string, params: Record<string, any>) => void
  deleteNode: (nodeId: string) => void
  selectNode: (node: Node | null) => void
  setWorkflows: (workflows: { id: string; name: string; version: number }[]) => void
  setCurrentWorkflowId: (id: string | null) => void
  setExecutionResult: (result: any | null) => void
  setIsExecuting: (executing: boolean) => void
  loadWorkflow: (nodes: Node[], edges: Edge[], fromAI?: boolean) => void
  setOriginalWorkflow: (nodes: Node[], edges: Edge[]) => void
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

const defaultParams: Record<NodeType, Record<string, any>> = {
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
      nodes: applyNodeChanges(changes, get().nodes),
    })
  },
  
  onEdgesChange: (changes) => {
    set({
      edges: applyEdgeChanges(changes, get().edges),
    })
  },
  
  onConnect: (connection) => {
    set({
      edges: addEdge({ ...connection, type: 'smoothstep' }, get().edges),
    })
  },
  
  addNode: (type, position) => {
    const id = `node_${Date.now()}`
    const newNode: Node = {
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
          ? { ...node, data: { ...node.data, params: { ...node.data.params, ...params } } }
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
      const { nodes, edges } = JSON.parse(json)
      set({ nodes, edges, selectedNode: null })
    } catch (e) {
      console.error('Failed to import workflow:', e)
    }
  },
}))