import { useMemo } from 'react';
import ReactFlow, {
  type Node,
  type Edge,
  Position,
  MarkerType,
  useNodesState,
  useEdgesState,
  Background,
  Controls,
  MiniMap,
} from 'reactflow';
import 'reactflow/dist/style.css';
import type { WorkflowDefinitionDetail, WorkflowStateDetail } from '../../api/types';

interface StateMachineDiagramProps {
  definition: WorkflowDefinitionDetail;
}

const STATE_TYPE_STYLES: Record<string, { background: string; border: string }> = {
  Task: { background: '#dbeafe', border: '#3b82f6' },
  Choice: { background: '#f3e8ff', border: '#a855f7' },
  Wait: { background: '#fef3c7', border: '#f59e0b' },
  Parallel: { background: '#e0e7ff', border: '#6366f1' },
  Succeed: { background: '#dcfce7', border: '#22c55e' },
  Fail: { background: '#fee2e2', border: '#ef4444' },
  Compensation: { background: '#ffedd5', border: '#f97316' },
};

function getNodeStyle(type: string | undefined, isStart: boolean) {
  const baseStyle = STATE_TYPE_STYLES[type || ''] || { background: '#f3f4f6', border: '#6b7280' };
  return {
    background: baseStyle.background,
    border: `2px solid ${isStart ? '#22c55e' : baseStyle.border}`,
    borderRadius: '8px',
    padding: '12px 16px',
    minWidth: '150px',
    boxShadow: isStart ? '0 0 0 2px #22c55e33' : undefined,
  };
}

function calculateNodePositions(states: Record<string, WorkflowStateDetail>, startAt: string): Map<string, { x: number; y: number }> {
  const positions = new Map<string, { x: number; y: number }>();
  const visited = new Set<string>();
  const stateNames = Object.keys(states);

  // Calculate levels using BFS from start state
  const levels: Map<string, number> = new Map();
  const queue: Array<{ name: string; level: number }> = [{ name: startAt, level: 0 }];

  while (queue.length > 0) {
    const { name, level } = queue.shift()!;

    if (visited.has(name)) continue;
    visited.add(name);
    levels.set(name, level);

    const state = states[name];
    if (!state) continue;

    // Get next states
    const nextStates: string[] = [];
    if (state.next) nextStates.push(state.next);
    if (state.default) nextStates.push(state.default);
    if (state.choices) {
      state.choices.forEach(c => nextStates.push(c.next));
    }
    if (state.externalEvent?.timeoutNext) nextStates.push(state.externalEvent.timeoutNext);
    if (state.catch) {
      state.catch.forEach(c => nextStates.push(c.next));
    }
    if (state.finalState) nextStates.push(state.finalState);

    // Add to queue
    nextStates.forEach(nextState => {
      if (!visited.has(nextState) && states[nextState]) {
        queue.push({ name: nextState, level: level + 1 });
      }
    });
  }

  // Add any orphan states (not reachable from start)
  stateNames.forEach(name => {
    if (!levels.has(name)) {
      levels.set(name, Math.max(...Array.from(levels.values())) + 1);
    }
  });

  // Group states by level
  const levelGroups: Map<number, string[]> = new Map();
  levels.forEach((level, name) => {
    if (!levelGroups.has(level)) {
      levelGroups.set(level, []);
    }
    levelGroups.get(level)!.push(name);
  });

  // Position nodes
  const NODE_HEIGHT = 80;
  const HORIZONTAL_GAP = 250;
  const VERTICAL_GAP = 120;

  levelGroups.forEach((names, level) => {
    const totalHeight = names.length * NODE_HEIGHT + (names.length - 1) * VERTICAL_GAP;
    const startY = -totalHeight / 2;

    names.forEach((name, index) => {
      positions.set(name, {
        x: level * HORIZONTAL_GAP,
        y: startY + index * (NODE_HEIGHT + VERTICAL_GAP),
      });
    });
  });

  return positions;
}

export function StateMachineDiagram({ definition }: StateMachineDiagramProps) {
  const { nodes: initialNodes, edges: initialEdges } = useMemo(() => {
    const states = definition.states || {};
    const positions = calculateNodePositions(states, definition.startAt);

    // Create nodes
    const nodes: Node[] = Object.entries(states).map(([name, state]) => {
      const pos = positions.get(name) || { x: 0, y: 0 };
      const isStart = name === definition.startAt;

      return {
        id: name,
        position: pos,
        data: {
          label: (
            <div className="text-center">
              <div className="font-semibold text-sm">{name}</div>
              <div className="text-xs text-gray-500 mt-1">{state.type}</div>
              {state.activity && (
                <div className="text-xs text-blue-600 mt-1 truncate max-w-[140px]">
                  {state.activity}
                </div>
              )}
              {state.externalEvent && (
                <div className="text-xs text-yellow-600 mt-1">
                  waits: {state.externalEvent.eventName}
                </div>
              )}
            </div>
          ),
        },
        style: getNodeStyle(state.type, isStart),
        sourcePosition: Position.Right,
        targetPosition: Position.Left,
      };
    });

    // Create edges
    const edges: Edge[] = [];
    let edgeId = 0;

    Object.entries(states).forEach(([name, state]) => {
      // Normal next transition
      if (state.next && states[state.next]) {
        edges.push({
          id: `e${edgeId++}`,
          source: name,
          target: state.next,
          type: 'smoothstep',
          animated: false,
          style: { stroke: '#6b7280' },
          markerEnd: { type: MarkerType.ArrowClosed, color: '#6b7280' },
        });
      }

      // Default transition (for Choice states)
      if (state.default && states[state.default]) {
        edges.push({
          id: `e${edgeId++}`,
          source: name,
          target: state.default,
          type: 'smoothstep',
          animated: false,
          style: { stroke: '#9ca3af', strokeDasharray: '5,5' },
          label: 'default',
          labelStyle: { fontSize: 10, fill: '#6b7280' },
          labelBgStyle: { fill: 'white' },
          markerEnd: { type: MarkerType.ArrowClosed, color: '#9ca3af' },
        });
      }

      // Choice transitions
      if (state.choices) {
        state.choices.forEach((choice) => {
          if (states[choice.next]) {
            edges.push({
              id: `e${edgeId++}`,
              source: name,
              target: choice.next,
              type: 'smoothstep',
              animated: false,
              style: { stroke: '#a855f7' },
              label: `${choice.condition.comparisonType || 'if'}`,
              labelStyle: { fontSize: 10, fill: '#7c3aed' },
              labelBgStyle: { fill: 'white' },
              markerEnd: { type: MarkerType.ArrowClosed, color: '#a855f7' },
            });
          }
        });
      }

      // Wait timeout transition
      if (state.externalEvent?.timeoutNext && states[state.externalEvent.timeoutNext]) {
        edges.push({
          id: `e${edgeId++}`,
          source: name,
          target: state.externalEvent.timeoutNext,
          type: 'smoothstep',
          animated: true,
          style: { stroke: '#f59e0b', strokeDasharray: '5,5' },
          label: 'timeout',
          labelStyle: { fontSize: 10, fill: '#d97706' },
          labelBgStyle: { fill: 'white' },
          markerEnd: { type: MarkerType.ArrowClosed, color: '#f59e0b' },
        });
      }

      // Catch/error transitions
      if (state.catch) {
        state.catch.forEach(catchDef => {
          if (states[catchDef.next]) {
            edges.push({
              id: `e${edgeId++}`,
              source: name,
              target: catchDef.next,
              type: 'smoothstep',
              animated: false,
              style: { stroke: '#ef4444', strokeDasharray: '3,3' },
              label: 'error',
              labelStyle: { fontSize: 10, fill: '#dc2626' },
              labelBgStyle: { fill: 'white' },
              markerEnd: { type: MarkerType.ArrowClosed, color: '#ef4444' },
            });
          }
        });
      }

      // Compensation final state
      if (state.finalState && states[state.finalState]) {
        edges.push({
          id: `e${edgeId++}`,
          source: name,
          target: state.finalState,
          type: 'smoothstep',
          animated: false,
          style: { stroke: '#f97316' },
          label: 'final',
          labelStyle: { fontSize: 10, fill: '#ea580c' },
          labelBgStyle: { fill: 'white' },
          markerEnd: { type: MarkerType.ArrowClosed, color: '#f97316' },
        });
      }
    });

    return { nodes, edges };
  }, [definition]);

  const [nodes, , onNodesChange] = useNodesState(initialNodes);
  const [edges, , onEdgesChange] = useEdgesState(initialEdges);

  return (
    <div className="h-[500px] w-full border border-gray-200 rounded-lg overflow-hidden">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        fitView
        fitViewOptions={{ padding: 0.2 }}
        minZoom={0.1}
        maxZoom={2}
      >
        <Background color="#e5e7eb" gap={16} />
        <Controls />
        <MiniMap
          nodeColor={(node) => {
            const stateType = definition.states?.[node.id]?.type;
            return STATE_TYPE_STYLES[stateType || '']?.border || '#6b7280';
          }}
          maskColor="rgb(255, 255, 255, 0.8)"
        />
      </ReactFlow>

      {/* Legend */}
      <div className="absolute bottom-4 left-4 bg-white p-3 rounded-lg shadow-md border border-gray-200 text-xs">
        <div className="font-semibold mb-2">Legend</div>
        <div className="grid grid-cols-2 gap-x-4 gap-y-1">
          {Object.entries(STATE_TYPE_STYLES).map(([type, style]) => (
            <div key={type} className="flex items-center gap-1">
              <div
                className="w-3 h-3 rounded"
                style={{ background: style.background, border: `1px solid ${style.border}` }}
              />
              <span>{type}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
