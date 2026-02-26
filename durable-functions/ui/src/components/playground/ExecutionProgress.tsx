import { useMemo } from 'react';
import type { WorkflowDefinitionDetail } from '../../api/types';
import type { ExecutedStep } from '../../stores/usePlaygroundStore';

interface ExecutionProgressProps {
  definition: WorkflowDefinitionDetail | null;
  instanceId: string | null;
  status: string | null;
  currentStep: string | null;
  executedSteps: ExecutedStep[];
  error: { message: string; code: string } | null;
  onStepClick?: (stepName: string) => void;
  selectedStepName: string | null;
}

// Status to color mapping
const STATUS_COLORS: Record<string, { bg: string; border: string; text: string; dot: string }> = {
  Pending: { bg: 'bg-yellow-900/20', border: 'border-yellow-500/50', text: 'text-yellow-400', dot: 'bg-yellow-400' },
  Running: { bg: 'bg-blue-900/20', border: 'border-blue-500/50', text: 'text-blue-400', dot: 'bg-blue-400' },
  Completed: { bg: 'bg-green-900/20', border: 'border-green-500/50', text: 'text-green-400', dot: 'bg-green-400' },
  Failed: { bg: 'bg-red-900/20', border: 'border-red-500/50', text: 'text-red-400', dot: 'bg-red-400' },
  Terminated: { bg: 'bg-gray-900/20', border: 'border-gray-500/50', text: 'text-gray-400', dot: 'bg-gray-400' },
};

// Step status to styling
const STEP_STATUS_STYLES: Record<string, { icon: string; color: string }> = {
  completed: { icon: '✓', color: 'text-green-400' },
  failed: { icon: '✗', color: 'text-red-400' },
  running: { icon: '●', color: 'text-blue-400 animate-pulse' },
};

export function ExecutionProgress({
  definition,
  instanceId,
  status,
  currentStep,
  executedSteps,
  error,
  onStepClick,
  selectedStepName,
}: ExecutionProgressProps) {
  // Calculate execution progress
  const progress = useMemo(() => {
    if (!definition?.states) return { completed: 0, total: 0, percentage: 0 };
    const total = Object.keys(definition.states).length;
    const completed = executedSteps.filter(s => s.status === 'completed').length;
    return {
      completed,
      total,
      percentage: Math.round((completed / total) * 100),
    };
  }, [definition, executedSteps]);

  // Get execution timeline
  const timeline = useMemo(() => {
    return executedSteps.map((step, index) => ({
      ...step,
      index,
      isLast: index === executedSteps.length - 1,
      isCurrent: step.stepName === currentStep,
    }));
  }, [executedSteps, currentStep]);

  // Status colors
  const statusStyle = STATUS_COLORS[status || 'Pending'] || STATUS_COLORS.Pending;

  if (!instanceId) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center max-w-md mx-auto">
          <div className="text-6xl mb-4">🎮</div>
          <h2 className="text-xl font-medium text-white mb-2">Workflow Playground</h2>
          <p className="text-gray-400 mb-4">
            Execute workflows and watch them run in real-time. Select a definition, enter an Entity ID, and click Execute.
          </p>
          <div className="flex flex-wrap justify-center gap-2 text-xs text-gray-500">
            <span className="px-2 py-1 bg-dark-card border border-dark-border rounded">
              Ctrl+Enter to execute
            </span>
            <span className="px-2 py-1 bg-dark-card border border-dark-border rounded">
              Esc to reset
            </span>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col overflow-hidden">
      {/* Header with status */}
      <div className={`p-4 ${statusStyle.bg} border-b ${statusStyle.border}`}>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className={`w-3 h-3 rounded-full ${statusStyle.dot} ${status === 'Running' ? 'animate-pulse' : ''}`} />
            <div>
              <h3 className={`font-medium ${statusStyle.text}`}>{status || 'Unknown'}</h3>
              <p className="text-xs text-gray-400 font-mono">{instanceId}</p>
            </div>
          </div>
          {progress.total > 0 && (
            <div className="text-right">
              <div className={`text-lg font-bold ${statusStyle.text}`}>{progress.percentage}%</div>
              <div className="text-xs text-gray-400">{progress.completed}/{progress.total} steps</div>
            </div>
          )}
        </div>

        {/* Progress bar */}
        {progress.total > 0 && (
          <div className="mt-3 h-1.5 bg-dark-bg rounded-full overflow-hidden">
            <div
              className={`h-full ${status === 'Failed' ? 'bg-red-500' : 'bg-blue-500'} transition-all duration-500 ease-out`}
              style={{ width: `${progress.percentage}%` }}
            />
          </div>
        )}
      </div>

      {/* Current step highlight */}
      {currentStep && status === 'Running' && (
        <div className="px-4 py-2 bg-blue-900/10 border-b border-blue-500/30 flex items-center gap-2">
          <span className="animate-spin text-blue-400">⟳</span>
          <span className="text-sm text-blue-400">
            Currently executing: <strong>{currentStep}</strong>
          </span>
        </div>
      )}

      {/* Execution timeline */}
      <div className="flex-1 overflow-auto p-4">
        {timeline.length === 0 && status === 'Pending' && (
          <div className="text-center text-gray-500 py-8">
            <div className="animate-pulse text-4xl mb-2">⏳</div>
            <p>Waiting for workflow to start...</p>
          </div>
        )}

        {timeline.length > 0 && (
          <div className="space-y-1">
            {timeline.map((step) => {
              const stepStyle = STEP_STATUS_STYLES[step.status] || STEP_STATUS_STYLES.running;
              const isSelected = step.stepName === selectedStepName;

              return (
                <button
                  key={`${step.stepName}-${step.index}`}
                  onClick={() => onStepClick?.(step.stepName)}
                  className={`w-full text-left p-3 rounded-lg transition-all ${
                    isSelected
                      ? 'bg-blue-900/30 border-2 border-blue-500'
                      : 'bg-dark-card border border-dark-border hover:border-gray-600'
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <span className={`text-lg ${stepStyle.color}`}>{stepStyle.icon}</span>
                      <div>
                        <p className="font-medium text-white">{step.stepName}</p>
                        <p className="text-xs text-gray-400">{step.stepType}</p>
                      </div>
                    </div>
                    <div className="text-right text-xs text-gray-500">
                      {step.duration != null && <p>{step.duration}ms</p>}
                      <p>{new Date(step.executedAt).toLocaleTimeString()}</p>
                    </div>
                  </div>

                  {/* Connection line to next step */}
                  {!step.isLast && (
                    <div className="ml-2.5 mt-1 mb-0 h-4 border-l-2 border-gray-600 border-dashed" />
                  )}
                </button>
              );
            })}

            {/* Current step indicator if running and has current step */}
            {status === 'Running' && currentStep && !timeline.find(t => t.stepName === currentStep) && (
              <div className="p-3 bg-blue-900/20 border-2 border-blue-500/50 rounded-lg animate-pulse">
                <div className="flex items-center gap-3">
                  <span className="text-lg text-blue-400">●</span>
                  <div>
                    <p className="font-medium text-blue-300">{currentStep}</p>
                    <p className="text-xs text-blue-400">In progress...</p>
                  </div>
                </div>
              </div>
            )}
          </div>
        )}

        {/* Error display */}
        {error && (
          <div className="mt-4 p-4 bg-red-900/20 border border-red-500/50 rounded-lg">
            <div className="flex items-start gap-3">
              <span className="text-2xl">⚠️</span>
              <div>
                <h4 className="font-medium text-red-400">Workflow Error</h4>
                <p className="text-sm text-red-300 mt-1">{error.message}</p>
                {error.code && (
                  <p className="text-xs text-red-500 mt-2">Error Code: {error.code}</p>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Completed message */}
        {status === 'Completed' && (
          <div className="mt-4 p-4 bg-green-900/20 border border-green-500/50 rounded-lg">
            <div className="flex items-center gap-3">
              <span className="text-2xl">🎉</span>
              <div>
                <h4 className="font-medium text-green-400">Workflow Completed</h4>
                <p className="text-sm text-green-300 mt-1">
                  Successfully executed {progress.completed} steps
                </p>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
