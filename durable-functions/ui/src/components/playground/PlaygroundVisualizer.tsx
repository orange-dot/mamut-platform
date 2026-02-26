import { useMemo, useCallback } from 'react';
import ReactFlow, {
  type Node,
  type Edge,
  Position,
  MarkerType,
  Background,
  Controls,
  MiniMap,
} from 'reactflow';
import 'reactflow/dist/style.css';
import type { WorkflowDefinitionDetail } from '../../api/types';

interface PlaygroundVisualizerProps {
  definition: WorkflowDefinitionDetail | null;
  currentStep?: string | null;
  executedSteps?: string[];
  onStepClick?: (stepName: string) => void;
}

const STATE_TYPE_STYLES: Record<string, { background: string; border: string; icon: string }> = {
  Task: { background: '#1e3a5f', border: '#3b82f6', icon: '⚙️' },
  Choice: { background: '#3b1f5c', border: '#a855f7', icon: '🔀' },
  Wait: { background: '#4a3728', border: '#f59e0b', icon: '⏳' },
  Parallel: { background: '#2d2f5c', border: '#6366f1', icon: '⚡' },
  Succeed: { background: '#1a3d2e', border: '#22c55e', icon: '✅' },
  Fail: { background: '#4a2020', border: '#ef4444', icon: '❌' },
  Compensation: { background: '#4a3a20', border: '#eab308', icon: '↩️' },
};

function getNodeStyle(
  type: string,
  isStart: boolean,
  isCurrentStep: boolean,
  isExecuted: boolean
) {
  const baseStyle = STATE_TYPE_STYLES[type] || { background: '#374151', border: '#6b7280' };

  let borderColor = baseStyle.border;
  let boxShadow = '0 2px 4px rgba(0,0,0,0.3)';

  if (isCurrentStep) {
    borderColor = '#3b82f6';
    boxShadow = '0 0 0 3px rgba(59, 130, 246, 0.5), 0 0 20px rgba(59, 130, 246, 0.3)';
  } else if (isExecuted) {
    borderColor = '#22c55e';
    boxShadow = '0 0 0 2px rgba(34, 197, 94, 0.3)';
  } else if (isStart) {
    borderColor = '#22c55e';
    boxShadow = '0 0 0 2px #22c55e33';
  }

  return {
    background: baseStyle.background,
    border: `2px solid ${borderColor}`,
    borderRadius: '8px',
    padding: '12px 16px',
    minWidth: '150px',
    color: '#e5e7eb',
    boxShadow,
    opacity: isExecuted || isCurrentStep ? 1 : 0.7,
    transition: 'all 0.3s ease',
  };
}

export function PlaygroundVisualizer({
  definition,
  currentStep,
  executedSteps = [],
  onStepClick,
}: PlaygroundVisualizerProps) {
  // Convert definition to nodes and edges
  const { nodes, edges } = useMemo(() => {
    if (!definition?.states) {
      return { nodes: [], edges: [] };
    }

    const stateEntries = Object.entries(definition.states);
    const nodeMap = new Map<string, { x: number; y: number }>();

    // Simple layout algorithm
    const cols = Math.ceil(Math.sqrt(stateEntries.length));
    const nodeWidth = 200;
    const nodeHeight = 80;
    const gapX = 100;
    const gapY = 100;

    stateEntries.forEach(([name], index) => {
      const col = index % cols;
      const row = Math.floor(index / cols);
      nodeMap.set(name, {
        x: col * (nodeWidth + gapX) + 50,
        y: row * (nodeHeight + gapY) + 50,
      });
    });

    const resultNodes: Node[] = stateEntries.map(([name, state]) => {
      const position = nodeMap.get(name) || { x: 0, y: 0 };
      const type = state.type || 'Task';
      const style = STATE_TYPE_STYLES[type] || STATE_TYPE_STYLES.Task;
      const isStart = definition.startAt === name;
      const isCurrentStepNode = currentStep === name;
      const isExecutedNode = executedSteps.includes(name);

      return {
        id: name,
        position,
        data: {
          label: (
            <div
              className="flex items-center gap-2 cursor-pointer"
              onClick={() => onStepClick?.(name)}
            >
              <span className="text-lg">{style.icon}</span>
              <div className="flex flex-col">
                <span className="font-medium text-sm">{name}</span>
                <span className="text-xs text-gray-400">{type}</span>
              </div>
              {isStart && (
                <span className="ml-auto text-xs text-green-400" title="Start state">
                  ▶
                </span>
              )}
              {isCurrentStepNode && (
                <span className="ml-auto animate-pulse">🔵</span>
              )}
              {isExecutedNode && !isCurrentStepNode && (
                <span className="ml-auto text-green-400">✓</span>
              )}
            </div>
          ),
        },
        sourcePosition: Position.Right,
        targetPosition: Position.Left,
        style: getNodeStyle(type, isStart, isCurrentStepNode, isExecutedNode),
      };
    });

    const resultEdges: Edge[] = [];
    let edgeId = 0;

    stateEntries.forEach(([name, state]) => {
      // Handle next transitions
      if (state.next && nodeMap.has(state.next)) {
        resultEdges.push({
          id: `e-${edgeId++}`,
          source: name,
          target: state.next,
          type: 'smoothstep',
          style: { stroke: '#6b7280' },
          markerEnd: { type: MarkerType.ArrowClosed, color: '#6b7280' },
        });
      }

      // Handle choice transitions
      if (state.choices) {
        state.choices.forEach((choice) => {
          if (choice.next && nodeMap.has(choice.next)) {
            resultEdges.push({
              id: `e-${edgeId++}`,
              source: name,
              target: choice.next,
              type: 'smoothstep',
              style: { stroke: '#a855f7', strokeDasharray: '5,5' },
              markerEnd: { type: MarkerType.ArrowClosed, color: '#a855f7' },
              label: choice.condition?.variable || 'condition',
              labelStyle: { fill: '#9ca3af', fontSize: 10 },
              labelBgStyle: { fill: '#1f2937', fillOpacity: 0.8 },
            });
          }
        });

        // Default transition for choice
        if (state.default && nodeMap.has(state.default)) {
          resultEdges.push({
            id: `e-${edgeId++}`,
            source: name,
            target: state.default,
            type: 'smoothstep',
            style: { stroke: '#6b7280', strokeDasharray: '5,5' },
            markerEnd: { type: MarkerType.ArrowClosed, color: '#6b7280' },
            label: 'default',
            labelStyle: { fill: '#9ca3af', fontSize: 10 },
            labelBgStyle: { fill: '#1f2937', fillOpacity: 0.8 },
          });
        }
      }

      // Handle catch transitions
      if (state.catch) {
        state.catch.forEach((catchItem) => {
          if (catchItem.next && nodeMap.has(catchItem.next)) {
            resultEdges.push({
              id: `e-${edgeId++}`,
              source: name,
              target: catchItem.next,
              type: 'smoothstep',
              style: { stroke: '#ef4444', strokeDasharray: '3,3' },
              markerEnd: { type: MarkerType.ArrowClosed, color: '#ef4444' },
              label: 'error',
              labelStyle: { fill: '#ef4444', fontSize: 10 },
              labelBgStyle: { fill: '#1f2937', fillOpacity: 0.8 },
            });
          }
        });
      }
    });

    return { nodes: resultNodes, edges: resultEdges };
  }, [definition, currentStep, executedSteps, onStepClick]);

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      onStepClick?.(node.id);
    },
    [onStepClick]
  );

  if (!definition) {
    return (
      <div className="h-full flex items-center justify-center text-gray-500">
        <div className="text-center">
          <p className="text-2xl mb-2">📋</p>
          <p>Select a workflow definition to visualize</p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full w-full">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodeClick={onNodeClick}
        fitView
        fitViewOptions={{ padding: 0.2 }}
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable={true}
        panOnScroll
        zoomOnScroll
        minZoom={0.3}
        maxZoom={2}
      >
        <Background color="#374151" gap={20} size={1} />
        <Controls
          showInteractive={false}
          className="bg-dark-card border border-dark-border rounded-lg"
        />
        <MiniMap
          nodeColor={(node) => {
            const type = node.data?.type || 'Task';
            return STATE_TYPE_STYLES[type]?.border || '#6b7280';
          }}
          maskColor="rgba(0, 0, 0, 0.8)"
          className="bg-dark-bg border border-dark-border rounded-lg"
        />
      </ReactFlow>
    </div>
  );
}
