import { useState, useEffect } from 'react'
import { useWorkflowStore, Node, Edge } from '../stores/workflowStore'
import { X, ArrowRight, Plus, Minus, RefreshCw } from 'lucide-react'

interface DiffPatch {
  op: 'add' | 'remove' | 'replace'
  path: string
  value?: any
}

interface DiffViewerProps {
  originalNodes: Node[]
  originalEdges: Edge[]
  onApplyPatch: (patch: DiffPatch[]) => void
  onClose: () => void
}

export default function DiffViewer({ originalNodes, originalEdges, onApplyPatch, onClose }: DiffViewerProps) {
  const { nodes, edges, loadWorkflow } = useWorkflowStore()
  const [diffs, setDiffs] = useState<DiffPatch[]>([])

  useEffect(() => {
    calculateDiffs()
  }, [nodes, edges, originalNodes, originalEdges])

  const calculateDiffs = () => {
    const patches: DiffPatch[] = []

    const originalNodeMap = new Map(originalNodes.map(n => [n.id, n]))
    const currentNodeMap = new Map(nodes.map(n => [n.id, n]))

    for (const [id, node] of currentNodeMap) {
      const original = originalNodeMap.get(id)
      if (!original) {
        patches.push({ op: 'add', path: `/nodes/${id}`, value: node })
      } else if (JSON.stringify(node.data.params) !== JSON.stringify(original.data.params)) {
        patches.push({ 
          op: 'replace', 
          path: `/nodes/${id}/params`, 
          value: node.data.params 
        })
      }
      if (node.position.x !== original.position.x || node.position.y !== original.position.y) {
        patches.push({
          op: 'replace',
          path: `/nodes/${id}/position`,
          value: node.position
        })
      }
    }

    for (const [id] of originalNodeMap) {
      if (!currentNodeMap.has(id)) {
        patches.push({ op: 'remove', path: `/nodes/${id}` })
      }
    }

    const originalEdgeMap = new Map(originalEdges.map(e => [`${e.source}->${e.target}`, e]))
    const currentEdgeMap = new Map(edges.map(e => [`${e.source}->${e.target}`, e]))

    for (const [key, edge] of currentEdgeMap) {
      if (!originalEdgeMap.has(key)) {
        patches.push({ op: 'add', path: `/edges/${edge.id}`, value: edge })
      }
    }

    for (const [key] of originalEdgeMap) {
      if (!currentEdgeMap.has(key)) {
        const edge = originalEdges.find(e => `${e.source}->${e.target}` === key)
        if (edge) {
          patches.push({ op: 'remove', path: `/edges/${edge.id}` })
        }
      }
    }

    setDiffs(patches)
  }

  const getOpIcon = (op: string) => {
    switch (op) {
      case 'add': return <Plus size={14} className="text-green-400" />
      case 'remove': return <Minus size={14} className="text-red-400" />
      case 'replace': return <RefreshCw size={14} className="text-yellow-400" />
    }
  }

  const getPathLabel = (path: string) => {
    if (path.includes('/nodes/')) {
      const id = path.split('/nodes/')[1]?.split('/')[0]
      const node = nodes.find(n => n.id === id) || originalNodes.find(n => n.id === id)
      const label = node?.data?.label || id
      const field = path.split('/params/')[1] || path.split('/position/')[1]
      return field ? `${label} → ${field}` : label
    }
    if (path.includes('/edges/')) {
      return path.split('/edges/')[1]?.split('/')[0] || 'edge'
    }
    return path
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-surface rounded-lg w-[600px] max-h-[80vh] flex flex-col">
        <div className="flex items-center justify-between p-4 border-b border-gray-700">
          <h3 className="text-lg font-semibold">Changes from Original</h3>
          <button onClick={onClose} className="text-gray-400 hover:text-white">
            <X size={20} />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-4">
          {diffs.length === 0 ? (
            <p className="text-gray-500 text-center py-8">No changes detected</p>
          ) : (
            <div className="space-y-2">
              {diffs.map((diff, i) => (
                <div key={i} className="flex items-start gap-3 p-3 bg-gray-800/50 rounded">
                  <div className="mt-1">{getOpIcon(diff.op)}</div>
                  <div className="flex-1">
                    <div className="text-sm font-medium">{getPathLabel(diff.path)}</div>
                    <div className="text-xs text-gray-400 mt-1">
                      {diff.op === 'replace' && diff.value && (
                        <span className="text-yellow-400">Modified</span>
                      )}
                      {diff.op === 'add' && (
                        <span className="text-green-400">Added</span>
                      )}
                      {diff.op === 'remove' && (
                        <span className="text-red-400">Removed</span>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="p-4 border-t border-gray-700 flex gap-2">
          <button 
            onClick={() => {
              loadWorkflow(originalNodes, originalEdges)
              onClose()
            }}
            className="btn btn-secondary flex-1"
          >
            Reset to Original
          </button>
          <button 
            onClick={() => {
              onApplyPatch(diffs)
              onClose()
            }}
            className="btn btn-primary flex-1"
          >
            Save as New Version
          </button>
        </div>
      </div>
    </div>
  )
}