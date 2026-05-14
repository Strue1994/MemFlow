import axios from 'axios'

const API_BASE = import.meta.env.VITE_API_URL || import.meta.env.VITE_API_BASE_URL || '/api'

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

export interface Pattern {
  id: string
  name: string
  category: string
  description: string
  triggers: string[]
  nodes: string[]
}

export interface ValidationIssue {
  ruleId: string
  severity: 'error' | 'warning'
  message: string
  suggestion?: string
}

export interface PipelineStepEvent {
  step: string
  status: 'start' | 'completed' | 'failed' | 'awaiting_confirmation'
  payload?: any
}

export interface TaskExecutionResponse {
  route: 'workflow' | 'generated_workflow' | 'agent' | 'clarification'
  repeatable: boolean
  confidence: 'high' | 'medium' | 'low'
  reason: string
  success: boolean
  clarificationQuestion?: string
  workflow: { workflowId: string; generated: boolean } | null
  result: unknown
  failureCategory?: string
}

export interface AutonomyStatus {
  enabled: boolean
  running: boolean
  objective: string
  intervalSeconds: number
  lastTickAt: string | null
  nextTickAt: string | null
  lastAction: string | null
  lastError: string | null
  recent: Array<{
    at: string
    kind: 'observe' | 'reflect' | 'act' | 'error'
    message: string
    data?: unknown
  }>
}

export interface ComputerCapabilities {
  root: string
  platform: string
  browser: {
    openUrl: boolean
    fetchPage: boolean
    automation: boolean
    automationNote?: string
  }
  filesystem: {
    list: boolean
    search: boolean
    read: boolean
    write: boolean
    sandboxRoot: string
  }
  terminal: {
    run: boolean
    safeMode: boolean
    timeoutMs: number
    allowPatterns: string[]
  }
}

export interface ComputerDirectoryListing {
  root: string
  path: string
  items: Array<{
    name: string
    path: string
    kind: 'file' | 'directory'
    size: number | null
    modifiedAt: string
  }>
}

export interface ComputerSearchResults {
  root: string
  path: string
  query: string
  limit: number
  truncated: boolean
  items: Array<{
    name: string
    path: string
    kind: 'file' | 'directory'
    size: number | null
    modifiedAt: string
  }>
}

export interface LLMSettings {
  provider:
    | 'deepai'
    | 'gmn'
    | 'openai'
    | 'openrouter'
    | 'anthropic'
    | 'google'
    | 'deepseek'
    | 'groq'
    | 'mistral'
    | 'xai'
    | 'ollama'
    | 'openai-compatible'
  apiKey: string
  baseUrl: string
  model: string
  updatedAt: string | null
}

export interface LLMProviderPreset {
  id: LLMSettings['provider']
  label: string
  labelZh: string
  apiStyle: 'openai' | 'anthropic' | 'google'
  defaultBaseUrl: string
  defaultModel: string
  modelSuggestions: string[]
  note: string
}

export interface CommandResult {
  allowed: boolean
  command: string
  cwd: string
  stdout: string
  stderr: string
  exitCode: number | null
  durationMs: number
}

export const workflowApi = {
  compile: async (n8nJson: any, name?: string): Promise<CompileResponse> => {
    const response = await api.post('/compile', { n8n_json: n8nJson, name })
    return response.data
  },

  execute: async (workflowId: string, params?: Record<string, any>): Promise<any> => {
    try {
      const response = await api.post('/execute', { workflow_id: workflowId, params })
      return response.data
    } catch (error: any) {
      const message = error?.response?.data?.error || error?.response?.data?.message || error?.message || ''
      if (typeof message === 'string' && message.includes('Invalid return: no value to return')) {
        return {
          success: true,
          warning: 'Workflow completed without an explicit return value.',
          result: null,
        }
      }
      throw error
    }
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
    const response = await api.post('/admin/keys', { key: name, role, rate_limit: rateLimit })
    return response.data
  },

  deleteApiKey: async (key: string): Promise<void> => {
    await api.delete(`/admin/keys/${encodeURIComponent(key)}`)
  },

  listApiKeys: async (): Promise<{ name: string; role: string; created_at: string }[]> => {
    const response = await api.get('/admin/keys')
    return response.data
  },

  getStats: async (): Promise<{
    total_workflows: number
    total_executions: number
    successful_executions: number
    failed_executions: number
    success_rate: number
    avg_duration_ms: number
    executions_last_24h: number
    active_workflows: number
  }> => {
    const response = await api.get('/stats')
    return response.data
  },

  checkHealth: async (): Promise<{ status: string; db: string; active_workflows: number }> => {
    const response = await api.get('/health')
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

  createWorkflowV2: async (
    description: string,
    onStep?: (event: PipelineStepEvent) => void
  ): Promise<string> => {
    const response = await fetch(`${API_BASE}/create_workflow_v2`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...(localStorage.getItem('apiKey') ? { 'Authorization': `Bearer ${localStorage.getItem('apiKey')}` } : {})
      },
      body: JSON.stringify({ user_request: description, step: 1 })
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }

    if (response.headers.get('content-type')?.includes('text/event-stream')) {
      const reader = response.body?.getReader();
      const decoder = new TextDecoder();

      while (true) {
        const { done, value } = await reader!.read();
        if (done) break;

        const chunk = decoder.decode(value);
        const lines = chunk.split('\n');

        for (const line of lines) {
          if (line.startsWith('data: ')) {
            try {
              const event = JSON.parse(line.slice(6)) as PipelineStepEvent;
              onStep?.(event);
              if (event.step === 'completed' || event.status === 'completed') {
                return event.payload?.workflowId || '';
              }
            } catch {
              // Skip invalid JSON
            }
          }
        }
      }
      return '';
    }

    const data = await response.json();
    return data.workflow_id || '';
  },

  confirmWorkflowDesign: async (sessionId: string, design: any): Promise<any> => {
    const response = await api.post('/create_workflow_v2', {
      session_id: sessionId,
      step: 4,
      confirmed: true,
      design
    });
    return response.data;
  },

  findPatterns: async (query: string): Promise<Pattern[]> => {
    const response = await api.post('/patterns/match', { query });
    return response.data.patterns || [];
  },

  validateWorkflow: async (workflowJson: any): Promise<ValidationIssue[]> => {
    const response = await api.post('/validate', { n8n_json: workflowJson });
    return response.data.issues || [];
  },

  // ─── 自动优化 API ────────────────────────────────────────
  optimize: async (workflowId: string): Promise<{
    params: Array<{
      name: string
      current: number
      recommended: number
      impact: 'high' | 'medium' | 'low'
      description: string
    }>
    estimated_speedup: number
    estimated_accuracy: number
    estimated_cost_savings: number
  }> => {
    const response = await api.post('/optimize', { workflow_id: workflowId })
    return response.data
  },

  applyTuning: async (workflowId: string, params: Record<string, any>): Promise<{ success: boolean }> => {
    const response = await api.post('/apply-tuning', { 
      workflow_id: workflowId, 
      params 
    })
    return response.data
  },

  // ─── 学习总结 API ────────────────────────────────────────
  summarize: async (workflowId: string): Promise<{
    insights: Array<{
      category: 'performance' | 'accuracy' | 'cost' | 'pattern'
      title: string
      description: string
      impact: number
      actionable: boolean
      suggestions: string[]
    }>
    total_executions: number
    success_rate: number
    avg_duration: number
    updated_at: string
  }> => {
    const response = await api.post('/summarize', { workflow_id: workflowId })
    return response.data
  },

  // ─── 自然语言创建增强 API ───────────────────────────────
  enhanceNLWorkflow: async (
    description: string, 
    workflowId?: string
  ): Promise<{
    improved_workflow: any
    improvements: string[]
    learning_feedback: string
  }> => {
    const response = await api.post('/enhance-nl-workflow', {
      description,
      workflow_id: workflowId
    })
    return response.data
  },
}

export const autonomyApi = {
  status: async (): Promise<AutonomyStatus> => {
    const response = await api.get('/autonomy/status')
    return response.data
  },

  start: async (objective?: string, intervalSeconds?: number): Promise<AutonomyStatus> => {
    const response = await api.post('/autonomy/start', {
      objective,
      intervalSeconds,
    })
    return response.data
  },

  stop: async (): Promise<AutonomyStatus> => {
    const response = await api.post('/autonomy/stop')
    return response.data
  },

  tick: async (): Promise<AutonomyStatus> => {
    const response = await api.post('/autonomy/tick')
    return response.data
  },
}

export const taskApi = {
  execute: async (text: string): Promise<TaskExecutionResponse> => {
    const response = await api.post('/tasks/execute', { text })
    return response.data
  },

  history: async (): Promise<any[]> => {
    const response = await api.get('/tasks/history')
    return response.data.items || []
  },
}

export const computerApi = {
  capabilities: async (): Promise<ComputerCapabilities> => {
    const response = await api.get('/computer/capabilities')
    return response.data
  },

  listDirectory: async (relativePath: string): Promise<ComputerDirectoryListing> => {
    const response = await api.get('/computer/fs/list', { params: { path: relativePath } })
    return response.data
  },

  searchFiles: async (query: string, relativePath = '.'): Promise<ComputerSearchResults> => {
    const response = await api.get('/computer/fs/search', { params: { q: query, path: relativePath } })
    return response.data
  },

  readFile: async (relativePath: string): Promise<{ path: string; size: number; content: string }> => {
    const response = await api.get('/computer/fs/read', { params: { path: relativePath } })
    return response.data
  },

  writeFile: async (relativePath: string, content: string, append = false): Promise<{ path: string; size: number; modifiedAt: string; mode: string }> => {
    const response = await api.post('/computer/fs/write', { path: relativePath, content, append })
    return response.data
  },

  runCommand: async (command: string, cwd?: string): Promise<CommandResult> => {
    const response = await api.post('/computer/terminal/run', { command, cwd })
    return response.data
  },

  openUrl: async (url: string): Promise<{ opened: boolean; url: string }> => {
    const response = await api.post('/computer/browser/open', { url })
    return response.data
  },

  fetchUrl: async (url: string): Promise<{ url: string; status: number; contentType: string | null; title: string | null; bodyPreview: string }> => {
    const response = await api.post('/computer/browser/fetch', { url })
    return response.data
  },
}

export const llmSettingsApi = {
  get: async (): Promise<LLMSettings> => {
    const response = await api.get('/llm-settings')
    return response.data
  },

  getCatalog: async (): Promise<{ providers: LLMProviderPreset[] }> => {
    const response = await api.get('/llm-settings/catalog')
    return response.data
  },

  save: async (settings: Partial<LLMSettings>): Promise<LLMSettings> => {
    const response = await api.post('/llm-settings', settings)
    return response.data
  },

  test: async (settings: Partial<LLMSettings> & { prompt?: string }): Promise<{ success: boolean; provider: string; model: string; content: string }> => {
    const response = await api.post('/llm-settings/test', settings)
    return response.data
  },
}

export default api
