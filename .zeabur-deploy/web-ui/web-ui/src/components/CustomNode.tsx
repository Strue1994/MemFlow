import { memo } from 'react'
import { Handle, Position, NodeProps } from '@xyflow/react'
import { 
  Globe, 
  Code, 
  GitBranch, 
  Database, 
  FileText, 
  Mail, 
  MessageSquare, 
  Send,
  Table,
  Github,
  FileBox,
  Play
} from 'lucide-react'
import { WorkflowNode, NodeType } from '../stores/workflowStore'

const iconMap: Record<NodeType, React.ReactNode> = {
  http: <Globe size={16} />,
  code: <Code size={16} />,
  if: <GitBranch size={16} />,
  for: <GitBranch size={16} />,
  set: <Code size={16} />,
  db: <Database size={16} />,
  file: <FileText size={16} />,
  email: <Mail size={16} />,
  slack: <MessageSquare size={16} />,
  telegram: <Send size={16} />,
  sheets: <Table size={16} />,
  github: <Github size={16} />,
  notion: <FileBox size={16} />,
  trigger: <Play size={16} />,
}

const colorMap: Record<NodeType, string> = {
  http: '#4CAF50',
  code: '#FF9800',
  if: '#E91E63',
  for: '#9C27B0',
  set: '#2196F3',
  db: '#00BCD4',
  file: '#795548',
  email: '#F44336',
  slack: '#4A154B',
  telegram: '#0088CC',
  sheets: '#0F9D58',
  github: '#24292E',
  notion: '#000000',
  trigger: '#607D8B',
}

function CustomNode({ data, selected }: NodeProps<WorkflowNode>) {
  const hasInput = data.nodeType !== 'trigger'
  const hasOutput = !['email', 'slack', 'telegram', 'trigger'].includes(data.nodeType)

  return (
    <div className={`custom-node ${selected ? 'selected' : ''}`}>
      {hasInput && (
        <Handle 
          type="target" 
          position={Position.Top} 
          className="!bg-gray-500 !w-3 !h-3"
        />
      )}
      
      <div className="custom-node-header">
        <div 
          className="custom-node-icon"
          style={{ color: colorMap[data.nodeType] }}
        >
          {iconMap[data.nodeType]}
        </div>
        <div className="custom-node-title">{data.label}</div>
      </div>
      
      <div className="custom-node-body">
        {Object.entries(data.params)
          .slice(0, 2)
          .map(([key, value]) => (
            <div key={key} className="truncate">
              <span className="text-gray-500">{key}:</span>{' '}
              {typeof value === 'string' ? value.slice(0, 20) : JSON.stringify(value).slice(0, 20)}
            </div>
          ))}
      </div>
      
      {hasOutput && (
        <Handle 
          type="source" 
          position={Position.Bottom} 
          className="!bg-gray-500 !w-3 !h-3"
        />
      )}
    </div>
  )
}

export default memo(CustomNode)
