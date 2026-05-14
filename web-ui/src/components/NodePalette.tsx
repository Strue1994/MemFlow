import { DragEvent } from 'react'
import { useWorkflowStore, NodeType } from '../stores/workflowStore'
import { 
  Globe, 
  Code, 
  Database, 
  FileText, 
  Mail, 
  Play,
} from 'lucide-react'

interface NodeItem {
  type: NodeType
  label: string
  icon: React.ReactNode
  color: string
}

const nodeItems: NodeItem[] = [
  { type: 'trigger', label: 'Trigger', icon: <Play size={16} />, color: '#607D8B' },
  { type: 'http', label: 'HTTP Request', icon: <Globe size={16} />, color: '#4CAF50' },
  { type: 'set', label: 'Set Variable', icon: <Code size={16} />, color: '#2196F3' },
  { type: 'db', label: 'Database', icon: <Database size={16} />, color: '#00BCD4' },
  { type: 'file', label: 'File', icon: <FileText size={16} />, color: '#795548' },
  { type: 'email', label: 'Email', icon: <Mail size={16} />, color: '#F44336' },
]

export default function NodePalette() {
  const addNode = useWorkflowStore((s) => s.addNode)

  const onDragStart = (event: DragEvent, nodeType: NodeType) => {
    event.dataTransfer.setData('application/reactflow', nodeType)
    event.dataTransfer.effectAllowed = 'move'
  }

  const handleDragOver = (event: DragEvent) => {
    event.preventDefault()
    event.dataTransfer.dropEffect = 'move'
  }

  const handleDrop = (event: DragEvent) => {
    event.preventDefault()
    const nodeType = event.dataTransfer.getData('application/reactflow') as NodeType
    if (nodeType) {
      const reactFlowBounds = document.querySelector('.react-flow')?.getBoundingClientRect()
      if (reactFlowBounds) {
        const position = {
          x: event.clientX - reactFlowBounds.left - 90,
          y: event.clientY - reactFlowBounds.top - 30,
        }
        addNode(nodeType, position)
      }
    }
  }

  return (
    <div 
      className="panel w-56 flex flex-col gap-2"
      onDragOver={handleDragOver}
      onDrop={handleDrop}
    >
      <h3 className="text-sm font-semibold text-gray-400 mb-2">Node Palette</h3>
      
      <div className="flex flex-col gap-1">
        {nodeItems.map((item) => (
          <div
            key={item.type}
            draggable
            onDragStart={(e) => onDragStart(e, item.type)}
            className="flex items-center gap-3 p-2 rounded cursor-grab hover:bg-white/5 transition-colors"
          >
            <div 
              className="w-8 h-8 rounded flex items-center justify-center"
              style={{ backgroundColor: item.color + '22', color: item.color }}
            >
              {item.icon}
            </div>
            <span className="text-sm">{item.label}</span>
          </div>
        ))}
      </div>

      <div className="mt-4 p-2 bg-blue-900/30 rounded text-xs text-blue-300">
        Drag supported nodes to canvas
      </div>
    </div>
  )
}
