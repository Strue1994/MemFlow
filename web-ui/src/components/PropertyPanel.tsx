import { useWorkflowStore, NodeType } from '../stores/workflowStore'

const paramFields: Record<NodeType, { key: string; label: string; type: string; placeholder?: string }[]> = {
  http: [
    { key: 'method', label: 'Method', type: 'select', placeholder: 'GET' },
    { key: 'url', label: 'URL', type: 'text', placeholder: 'https://api.example.com' },
    { key: 'headers', label: 'Headers (JSON)', type: 'textarea' },
    { key: 'body', label: 'Body', type: 'textarea' },
    { key: 'outputVar', label: 'Output Variable', type: 'text' },
  ],
  set: [
    { key: 'name', label: 'Variable Name', type: 'text' },
    { key: 'value', label: 'Value', type: 'textarea' },
  ],
  code: [
    { key: 'script', label: 'JavaScript Code', type: 'textarea' },
    { key: 'outputVar', label: 'Output Variable', type: 'text' },
  ],
  if: [
    { key: 'condition', label: 'Condition (JS Expression)', type: 'textarea' },
  ],
  for: [
    { key: 'iterator', label: 'Iterator Variable', type: 'text' },
    { key: 'start', label: 'Start', type: 'number' },
    { key: 'end', label: 'End', type: 'number' },
    { key: 'step', label: 'Step', type: 'number' },
  ],
  db: [
    { key: 'connection', label: 'Connection String', type: 'text' },
    { key: 'query', label: 'SQL Query', type: 'textarea' },
    { key: 'outputVar', label: 'Output Variable', type: 'text' },
  ],
  file: [
    { key: 'operation', label: 'Operation', type: 'select' },
    { key: 'path', label: 'File Path', type: 'text' },
    { key: 'content', label: 'Content', type: 'textarea' },
  ],
  email: [
    { key: 'to', label: 'To', type: 'text' },
    { key: 'subject', label: 'Subject', type: 'text' },
    { key: 'body', label: 'Body', type: 'textarea' },
    { key: 'smtp_config', label: 'SMTP Config', type: 'text' },
  ],
  slack: [
    { key: 'channel', label: 'Channel', type: 'text' },
    { key: 'text', label: 'Message', type: 'textarea' },
    { key: 'token', label: 'Bot Token', type: 'password' },
  ],
  telegram: [
    { key: 'chat_id', label: 'Chat ID', type: 'text' },
    { key: 'text', label: 'Message', type: 'textarea' },
    { key: 'bot_token', label: 'Bot Token', type: 'password' },
  ],
  sheets: [
    { key: 'operation', label: 'Operation', type: 'select' },
    { key: 'spreadsheet_id', label: 'Spreadsheet ID', type: 'text' },
    { key: 'range', label: 'Range', type: 'text' },
    { key: 'access_token', label: 'Access Token', type: 'password' },
  ],
  github: [
    { key: 'operation', label: 'Operation', type: 'select' },
    { key: 'owner', label: 'Owner', type: 'text' },
    { key: 'repo', label: 'Repository', type: 'text' },
    { key: 'title', label: 'Title', type: 'text' },
    { key: 'body', label: 'Body', type: 'textarea' },
    { key: 'token', label: 'Token', type: 'password' },
  ],
  notion: [
    { key: 'operation', label: 'Operation', type: 'select' },
    { key: 'database_id', label: 'Database ID', type: 'text' },
    { key: 'properties', label: 'Properties (JSON)', type: 'textarea' },
    { key: 'token', label: 'Token', type: 'password' },
  ],
  trigger: [
    { key: 'type', label: 'Trigger Type', type: 'select' },
    { key: 'cron', label: 'Cron Expression', type: 'text' },
  ],
}

export default function PropertyPanel() {
  const { selectedNode, updateNodeParams, deleteNode } = useWorkflowStore()

  if (!selectedNode) {
    return (
      <div className="panel w-72">
        <h3 className="text-sm font-semibold text-gray-400 mb-2">Properties</h3>
        <p className="text-gray-500 text-sm">Select a node to edit its properties</p>
      </div>
    )
  }

  const nodeType = selectedNode.data.nodeType
  const params = selectedNode.data.params
  const fields = paramFields[nodeType] || []

  const handleChange = (key: string, value: unknown) => {
    updateNodeParams(selectedNode.id, { [key]: value })
  }

  const getParamValue = (key: string): string | number => {
    const value = params[key]
    if (typeof value === 'string' || typeof value === 'number') {
      return value
    }
    if (value == null) {
      return ''
    }
    return JSON.stringify(value)
  }

  return (
    <div className="panel w-72 flex flex-col">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-sm font-semibold text-gray-400">Properties</h3>
        <button
          onClick={() => deleteNode(selectedNode.id)}
          className="text-red-400 hover:text-red-300 text-sm"
        >
          Delete
        </button>
      </div>

      <div className="flex items-center gap-2 mb-4 p-2 bg-gray-800/50 rounded">
        <div className="text-sm font-medium">{selectedNode.data.label}</div>
      </div>

      <div className="flex flex-col gap-3 flex-1 overflow-y-auto">
        {fields.map((field: { key: string; label: string; type: string; placeholder?: string }) => (
          <div key={field.key}>
            <label className="label">{field.label}</label>
            {field.type === 'select' ? (
              <select
                className="input"
                value={String(params[field.key] ?? '')}
                onChange={(e) => handleChange(field.key, e.target.value)}
              >
                <option value="">Select...</option>
                {field.key === 'method' && (
                  <>
                    <option value="GET">GET</option>
                    <option value="POST">POST</option>
                    <option value="PUT">PUT</option>
                    <option value="DELETE">DELETE</option>
                    <option value="PATCH">PATCH</option>
                  </>
                )}
                {field.key === 'operation' && nodeType === 'file' && (
                  <>
                    <option value="read">Read</option>
                    <option value="write">Write</option>
                    <option value="append">Append</option>
                  </>
                )}
                {field.key === 'operation' && nodeType === 'sheets' && (
                  <>
                    <option value="read">Read</option>
                    <option value="write">Write</option>
                    <option value="append">Append</option>
                  </>
                )}
                {field.key === 'operation' && nodeType === 'github' && (
                  <>
                    <option value="create_issue">Create Issue</option>
                    <option value="list_issues">List Issues</option>
                    <option value="trigger_action">Trigger Action</option>
                  </>
                )}
                {field.key === 'operation' && nodeType === 'notion' && (
                  <>
                    <option value="create_page">Create Page</option>
                    <option value="query_database">Query Database</option>
                    <option value="update_page">Update Page</option>
                  </>
                )}
                {field.key === 'type' && (
                  <>
                    <option value="manual">Manual</option>
                    <option value="cron">Cron Schedule</option>
                    <option value="webhook">Webhook</option>
                  </>
                )}
              </select>
            ) : field.type === 'textarea' ? (
              <textarea
                className="input min-h-[60px]"
                value={String(getParamValue(field.key))}
                onChange={(e) => handleChange(field.key, e.target.value)}
                placeholder={field.placeholder}
              />
            ) : field.type === 'number' ? (
              <input
                type="number"
                className="input"
                value={getParamValue(field.key)}
                onChange={(e) => handleChange(field.key, Number(e.target.value))}
              />
            ) : (
              <input
                type={field.type}
                className="input"
                value={String(getParamValue(field.key))}
                onChange={(e) => handleChange(field.key, e.target.value)}
                placeholder={field.placeholder}
              />
            )}
          </div>
        ))}
      </div>
    </div>
  )
}
