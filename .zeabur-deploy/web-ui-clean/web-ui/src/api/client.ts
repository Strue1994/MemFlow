import axios from 'axios'

const API_BASE = import.meta.env.VITE_API_URL || '/api'

const api = axios.create({
  baseURL: API_BASE,
  headers: {
    'Content-Type': 'application/json',
  },
})

api.interceptors.request.use((config) => {
  const apiKey = localStorage.getItem('apiKey')
  if (apiKey) {
    config.headers['Authorization'] = `Bearer ${apiKey}`
  }
  return config
})

export interface CompileRequest {
  n8n_json: any
  name?: string
}

export interface CompileResponse {
  workflow_id: string
  version: number
}

export interface ExecuteRequest {
  workflow_id: string
  params?: Record<string, any>
}

export interface WorkflowInfo {
  id: string
  name: string
  version: number
}

export interface ExecutionLog {
  id: string
  workflow_id: string
  started_at: string
  finished_at: string
  duration_ms: number
  result?: any
  error?: string
}

export const workflowApi = {
  compile: async (n8nJson: any, name?: string): Promise<CompileResponse> => {
    const response = await api.post('/compile', { n8n_json: n8nJson, name })
    return response.data
  },

  execute: async (workflowId: string, params?: Record<string, any>): Promise<any> => {
    const response = await api.post('/execute', { workflow_id: workflowId, params })
    return response.data
  },

  getWorkflow: async (id: string, version?: number): Promise<any> => {
    const response = await api.get(`/workflow/${id}`, { params: { version } })
    return response.data
  },

  listWorkflows: async (): Promise<WorkflowInfo[]> => {
    const response = await api.get('/workflows')
    return response.data
  },

  deleteWorkflow: async (id: string): Promise<void> => {
    await api.delete(`/workflow/${id}`)
  },

  getLogs: async (id: string, limit?: number): Promise<ExecutionLog[]> => {
    const response = await api.get(`/workflow/${id}/logs`, { params: { limit } })
    return response.data
  },

  getRecentLogs: async (limit?: number): Promise<ExecutionLog[]> => {
    const response = await api.get('/logs', { params: { limit } })
    return response.data
  },

  createApiKey: async (name: string, role: string, rateLimit: number): Promise<{ key: string }> => {
    const response = await api.post('/admin/keys', { name, role, rate_limit: rateLimit })
    return response.data
  },

  listApiKeys: async (): Promise<{ name: string; role: string; created_at: string }[]> => {
    const response = await api.get('/admin/keys')
    return response.data
  },

  saveWorkflowDiff: async (
    workflowId: string, 
    modifiedN8nJson: any, 
    diffPatch: any[],
    userId?: string
  ): Promise<{ version: number }> => {
    const response = await api.post('/workflow/diff', {
      workflow_id: workflowId,
      modified_n8n_json: modifiedN8nJson,
      diff_patch: diffPatch,
      user_id: userId || 'anonymous',
    })
    return response.data
  },

  getWorkflowVersions: async (id: string): Promise<{ version: number; created_at: string }[]> => {
    const response = await api.get(`/workflow/${id}/versions`)
    return response.data
  },

  getPromptVersions: async (): Promise<{ id: number; version: string; created_at: string }[]> => {
    const response = await api.get('/prompts/versions')
    return response.data
  },
}

export default api