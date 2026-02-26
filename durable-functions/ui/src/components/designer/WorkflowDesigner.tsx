import { useCallback, useState } from 'react';
import ReactFlow, {
  type Node,
  type Connection,
  addEdge,
  Position,
  MarkerType,
  useNodesState,
  useEdgesState,
  Background,
  Controls,
  MiniMap,
  Panel,
} from 'reactflow';
import 'reactflow/dist/style.css';

interface WorkflowState {
  name: string;
  type: 'Task' | 'Choice' | 'Wait' | 'Parallel' | 'Succeed' | 'Fail';
  activity?: string;
  next?: string;
}

const STATE_TYPES = [
  { type: 'Task', label: 'Task', icon: '⚙️', description: 'Execute an activity' },
  { type: 'Choice', label: 'Choice', icon: '🔀', description: 'Branch based on condition' },
  { type: 'Wait', label: 'Wait', icon: '⏳', description: 'Wait for event or timeout' },
  { type: 'Parallel', label: 'Parallel', icon: '⚡', description: 'Execute branches in parallel' },
  { type: 'Succeed', label: 'Succeed', icon: '✅', description: 'End with success' },
  { type: 'Fail', label: 'Fail', icon: '❌', description: 'End with failure' },
];

const STATE_TYPE_STYLES: Record<string, { background: string; border: string }> = {
  Task: { background: '#1e3a5f', border: '#3b82f6' },
  Choice: { background: '#3b1f5c', border: '#a855f7' },
  Wait: { background: '#4a3728', border: '#f59e0b' },
  Parallel: { background: '#2d2f5c', border: '#6366f1' },
  Succeed: { background: '#1a3d2e', border: '#22c55e' },
  Fail: { background: '#4a2020', border: '#ef4444' },
};

function getNodeStyle(type: string, isStart: boolean) {
  const baseStyle = STATE_TYPE_STYLES[type] || { background: '#374151', border: '#6b7280' };
  return {
    background: baseStyle.background,
    border: `2px solid ${isStart ? '#22c55e' : baseStyle.border}`,
    borderRadius: '8px',
    padding: '12px 16px',
    minWidth: '150px',
    color: '#e5e7eb',
    boxShadow: isStart ? '0 0 0 2px #22c55e33' : '0 2px 4px rgba(0,0,0,0.3)',
  };
}

let nodeId = 0;
const getNodeId = () => `state_${nodeId++}`;

export function WorkflowDesigner() {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);
  const [startState, setStartState] = useState<string | null>(null);

  const onConnect = useCallback(
    (params: Connection) => {
      setEdges((eds) =>
        addEdge(
          {
            ...params,
            type: 'smoothstep',
            style: { stroke: '#6b7280' },
            markerEnd: { type: MarkerType.ArrowClosed, color: '#6b7280' },
          },
          eds
        )
      );
    },
    [setEdges]
  );

  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();

      const type = event.dataTransfer.getData('application/reactflow');
      if (!type) return;

      const position = {
        x: event.clientX - 300,
        y: event.clientY - 100,
      };

      const newNodeId = getNodeId();
      const isFirstNode = nodes.length === 0;

      if (isFirstNode) {
        setStartState(newNodeId);
      }

      const newNode: Node = {
        id: newNodeId,
        type: 'default',
        position,
        data: {
          label: (
            <div className="text-center">
              <div className="font-semibold text-sm">{newNodeId}</div>
              <div className="text-xs text-gray-400 mt-1">{type}</div>
            </div>
          ),
          stateType: type,
          stateName: newNodeId,
        },
        style: getNodeStyle(type, isFirstNode),
        sourcePosition: Position.Right,
        targetPosition: Position.Left,
      };

      setNodes((nds) => nds.concat(newNode));
    },
    [nodes, setNodes]
  );

  const onNodeClick = useCallback((_: React.MouseEvent, node: Node) => {
    setSelectedNode(node);
  }, []);

  const setAsStartState = useCallback(() => {
    if (!selectedNode) return;

    setStartState(selectedNode.id);
    setNodes((nds) =>
      nds.map((n) => ({
        ...n,
        style: getNodeStyle(n.data.stateType, n.id === selectedNode.id),
      }))
    );
  }, [selectedNode, setNodes]);

  const deleteSelectedNode = useCallback(() => {
    if (!selectedNode) return;

    setNodes((nds) => nds.filter((n) => n.id !== selectedNode.id));
    setEdges((eds) => eds.filter((e) => e.source !== selectedNode.id && e.target !== selectedNode.id));

    if (startState === selectedNode.id) {
      setStartState(null);
    }
    setSelectedNode(null);
  }, [selectedNode, startState, setNodes, setEdges]);

  const exportDefinition = useCallback(() => {
    if (nodes.length === 0) {
      alert('Add at least one state to export');
      return;
    }

    const states: Record<string, WorkflowState> = {};

    nodes.forEach((node) => {
      const outgoingEdge = edges.find((e) => e.source === node.id);
      states[node.id] = {
        name: node.id,
        type: node.data.stateType,
        next: outgoingEdge?.target,
      };
    });

    const definition = {
      name: 'NewWorkflow',
      version: '1.0.0',
      startAt: startState || nodes[0]?.id,
      states,
    };

    const json = JSON.stringify(definition, null, 2);
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'workflow-definition.json';
    a.click();
    URL.revokeObjectURL(url);
  }, [nodes, edges, startState]);

  const clearCanvas = useCallback(() => {
    if (nodes.length > 0 && !confirm('Clear all states?')) return;
    setNodes([]);
    setEdges([]);
    setStartState(null);
    setSelectedNode(null);
    nodeId = 0;
  }, [nodes, setNodes, setEdges]);

  return (
    <div className="h-full flex">
      {/* State Palette */}
      <div className="w-64 bg-dark-card border-r border-dark-border p-4 overflow-y-auto">
        <h2 className="text-lg font-semibold text-gray-100 mb-4">State Types</h2>
        <div className="space-y-2">
          {STATE_TYPES.map((stateType) => (
            <div
              key={stateType.type}
              className="p-3 bg-dark-bg border border-dark-border rounded-lg cursor-grab hover:border-blue-500 transition-colors"
              draggable
              onDragStart={(event) => {
                event.dataTransfer.setData('application/reactflow', stateType.type);
                event.dataTransfer.effectAllowed = 'move';
              }}
            >
              <div className="flex items-center gap-2">
                <span>{stateType.icon}</span>
                <span className="font-medium text-gray-200">{stateType.label}</span>
              </div>
              <p className="text-xs text-gray-500 mt-1">{stateType.description}</p>
            </div>
          ))}
        </div>

        <div className="mt-6 pt-4 border-t border-dark-border">
          <h3 className="text-sm font-semibold text-gray-300 mb-2">Instructions</h3>
          <ul className="text-xs text-gray-500 space-y-1">
            <li>• Drag states onto canvas</li>
            <li>• Connect by dragging handles</li>
            <li>• Click node to select</li>
            <li>• First node is start state</li>
          </ul>
        </div>
      </div>

      {/* Canvas */}
      <div className="flex-1 relative">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          onDrop={onDrop}
          onDragOver={onDragOver}
          onNodeClick={onNodeClick}
          fitView
          fitViewOptions={{ padding: 0.2 }}
          minZoom={0.1}
          maxZoom={2}
          className="bg-dark-bg"
        >
          <Background color="#30363d" gap={20} />
          <Controls className="bg-dark-card border-dark-border" />
          <MiniMap
            nodeColor={(node) => STATE_TYPE_STYLES[node.data?.stateType]?.border || '#6b7280'}
            maskColor="rgba(13, 17, 23, 0.8)"
            className="bg-dark-card border-dark-border"
          />

          <Panel position="top-right" className="space-x-2">
            <button
              onClick={exportDefinition}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 text-sm"
            >
              Export JSON
            </button>
            <button
              onClick={clearCanvas}
              className="px-4 py-2 bg-dark-card border border-dark-border text-gray-300 rounded-lg hover:bg-dark-hover text-sm"
            >
              Clear
            </button>
          </Panel>
        </ReactFlow>

        {/* Empty State */}
        {nodes.length === 0 && (
          <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
            <div className="text-center text-gray-500">
              <div className="text-4xl mb-2">🎨</div>
              <p>Drag state types from the left panel</p>
              <p className="text-sm">to start building your workflow</p>
            </div>
          </div>
        )}
      </div>

      {/* Properties Panel */}
      {selectedNode && (
        <div className="w-72 bg-dark-card border-l border-dark-border p-4">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-100">Properties</h2>
            <button
              onClick={() => setSelectedNode(null)}
              className="text-gray-500 hover:text-gray-300"
            >
              ✕
            </button>
          </div>

          <div className="space-y-4">
            <div>
              <label className="block text-sm text-gray-400 mb-1">State Name</label>
              <input
                type="text"
                value={selectedNode.data.stateName || selectedNode.id}
                readOnly
                className="w-full px-3 py-2 bg-dark-bg border border-dark-border rounded-lg text-gray-200"
              />
            </div>

            <div>
              <label className="block text-sm text-gray-400 mb-1">Type</label>
              <div className="px-3 py-2 bg-dark-bg border border-dark-border rounded-lg text-gray-200">
                {selectedNode.data.stateType}
              </div>
            </div>

            <div>
              <label className="block text-sm text-gray-400 mb-1">Start State</label>
              <div className="flex items-center gap-2">
                <span className={startState === selectedNode.id ? 'text-green-400' : 'text-gray-500'}>
                  {startState === selectedNode.id ? '✓ Yes' : 'No'}
                </span>
                {startState !== selectedNode.id && (
                  <button
                    onClick={setAsStartState}
                    className="text-xs text-blue-400 hover:text-blue-300"
                  >
                    Set as start
                  </button>
                )}
              </div>
            </div>

            <div className="pt-4 border-t border-dark-border">
              <button
                onClick={deleteSelectedNode}
                className="w-full px-4 py-2 bg-red-900/50 text-red-400 rounded-lg hover:bg-red-900/70 text-sm"
              >
                Delete State
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
