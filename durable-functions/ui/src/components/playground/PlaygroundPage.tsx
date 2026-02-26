import { useCallback, useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { ResizablePanels } from './ResizablePanels';
import { PlaygroundVisualizer } from './PlaygroundVisualizer';
import { JsonEditor } from './JsonEditor';
import { TemplateList } from './TemplateList';
import { ExecutionProgress } from './ExecutionProgress';
import { InspectorPanel } from './InspectorPanel';
import { usePlaygroundStore, type ExecutedStep } from '../../stores/usePlaygroundStore';
import { useDefinitions, useDefinitionDetail } from '../../hooks/useDefinitions';
import { useStartWorkflow, useTerminateWorkflow, useRaiseEvent } from '../../hooks/useWorkflows';
import { fetchWorkflowDetail } from '../../api/workflows';
import { StatusBadge } from '../common/StatusBadge';
import type { WorkflowDefinitionDetail } from '../../api/types';

export function PlaygroundPage() {
  const {
    definition,
    definitionJson,
    definitionTab,
    entityId,
    inputData,
    instanceId,
    executionData,
    isExecuting,
    panelSizes,
    timelineExpanded,
    inspectorTab,
    selectedStepName,
    validationErrors,
    setDefinition,
    setDefinitionJson,
    setDefinitionTab,
    setEntityId,
    setInstanceId,
    setExecutionData,
    setIsExecuting,
    setPanelSizes,
    setTimelineExpanded,
    setInspectorTab,
    setSelectedStepName,
    addToHistory,
    clearExecution,
  } = usePlaygroundStore();

  // Selected definition ID for loading
  const [selectedDefId, setSelectedDefId] = useState<string>('');

  // Fetch definitions list for dropdown
  const { data: definitionsData } = useDefinitions();

  // Fetch full definition detail when selected
  const { data: definitionDetail } = useDefinitionDetail(selectedDefId);

  // Update definition when detail is loaded
  useEffect(() => {
    if (definitionDetail) {
      setDefinition(definitionDetail);
    }
  }, [definitionDetail, setDefinition]);

  // Poll workflow status when executing
  const { data: workflowStatus } = useQuery({
    queryKey: ['playground-workflow', instanceId],
    queryFn: () => fetchWorkflowDetail(instanceId!),
    enabled: !!instanceId,
    refetchInterval: executionData?.status === 'Running' || executionData?.status === 'Pending' ? 2000 : false,
  });

  // Mutations
  const startWorkflow = useStartWorkflow();
  const terminateWorkflow = useTerminateWorkflow();
  const raiseEventMutation = useRaiseEvent();

  // Parse workflow status into execution data
  useEffect(() => {
    if (workflowStatus) {
      const customStatus = workflowStatus.CustomStatus as Record<string, unknown> | null;
      setExecutionData({
        status: workflowStatus.Status,
        currentStep: (customStatus?.currentStep as string) || null,
        executedSteps: (customStatus?.executedSteps as Array<{
          stepName: string;
          stepType: string;
          executedAt: string;
          duration?: number;
          status: 'completed' | 'failed' | 'running';
        }>) || [],
        stepResults: (customStatus?.stepResults as Record<string, unknown>) || {},
        variables: (customStatus?.variables as Record<string, unknown>) || {},
        error: (customStatus?.error as { message: string; code: string }) || null,
      });

      // Auto-stop executing flag when workflow completes
      if (workflowStatus.Status !== 'Running' && workflowStatus.Status !== 'Pending') {
        setIsExecuting(false);
      }
    }
  }, [workflowStatus, setExecutionData, setIsExecuting]);

  // Handle definition selection from dropdown
  const handleLoadDefinition = useCallback(
    (defId: string) => {
      setSelectedDefId(defId);
    },
    []
  );

  // Handle execute workflow
  const handleExecute = useCallback(async () => {
    if (!definition?.id || !entityId.trim()) {
      return;
    }

    setIsExecuting(true);
    clearExecution();

    try {
      let parsedInput = {};
      try {
        parsedInput = inputData.trim() ? JSON.parse(inputData) : {};
      } catch {
        // Invalid JSON, use empty object
      }

      const result = await startWorkflow.mutateAsync({
        workflowType: definition.id,
        entityId: entityId.trim(),
        data: parsedInput,
      });

      setInstanceId(result.InstanceId);
      addToHistory({
        instanceId: result.InstanceId,
        workflowName: definition.name || definition.id,
        status: 'Running',
      });
    } catch (error) {
      console.error('Failed to start workflow:', error);
      setIsExecuting(false);
    }
  }, [
    definition,
    entityId,
    inputData,
    startWorkflow,
    setIsExecuting,
    clearExecution,
    setInstanceId,
    addToHistory,
  ]);

  // Handle terminate
  const handleTerminate = useCallback(async () => {
    if (!instanceId) return;
    try {
      await terminateWorkflow.mutateAsync(instanceId);
    } catch (error) {
      console.error('Failed to terminate workflow:', error);
    }
  }, [instanceId, terminateWorkflow]);

  // Handle send event
  const handleSendEvent = useCallback(
    async (eventName: string, payload: unknown) => {
      if (!instanceId) throw new Error('No workflow instance');
      await raiseEventMutation.mutateAsync({ instanceId, eventName, eventData: payload });
    },
    [instanceId, raiseEventMutation]
  );

  // Handle keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Ctrl+Enter to execute
      if (e.ctrlKey && e.key === 'Enter') {
        e.preventDefault();
        if (definition && entityId && !isExecuting) {
          handleExecute();
        }
      }
      // Escape to clear selection
      if (e.key === 'Escape') {
        setSelectedStepName(null);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [definition, entityId, isExecuting, handleExecute, setSelectedStepName]);

  // Left Panel: Definition
  const leftPanel = (
    <div className="h-full flex flex-col">
      {/* Definition Tabs */}
      <div className="flex border-b border-dark-border">
        {(['visual', 'json', 'templates'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setDefinitionTab(tab)}
            className={`px-4 py-2 text-sm font-medium transition-colors ${
              definitionTab === tab
                ? 'text-blue-400 border-b-2 border-blue-400 bg-dark-hover'
                : 'text-gray-400 hover:text-white hover:bg-dark-hover'
            }`}
          >
            {tab === 'visual' ? 'Visual' : tab === 'json' ? 'JSON' : 'Templates'}
          </button>
        ))}
      </div>

      {/* Definition Content */}
      <div className="flex-1 overflow-hidden">
        {definitionTab === 'visual' && (
          <PlaygroundVisualizer
            definition={definition}
            currentStep={executionData?.currentStep}
            executedSteps={executionData?.executedSteps?.map((s: ExecutedStep) => s.stepName) || []}
            onStepClick={setSelectedStepName}
          />
        )}

        {definitionTab === 'json' && (
          <div className="h-full flex flex-col">
            <JsonEditor
              value={definitionJson}
              onChange={setDefinitionJson}
              placeholder="Paste workflow definition JSON here..."
            />
            {validationErrors.length > 0 && (
              <div className="p-2 bg-red-900/20 border-t border-red-500/50">
                {validationErrors.map((err, i) => (
                  <p key={i} className="text-red-400 text-xs">
                    {err}
                  </p>
                ))}
              </div>
            )}
          </div>
        )}

        {definitionTab === 'templates' && (
          <TemplateList
            onSelect={(templateDef: WorkflowDefinitionDetail) => {
              setDefinition(templateDef);
              setDefinitionTab('visual');
            }}
          />
        )}
      </div>
    </div>
  );

  // Center Panel: Execution Visualization
  const centerPanel = (
    <div className="h-full flex flex-col">
      {/* Toolbar */}
      <div className="flex items-center gap-4 p-3 border-b border-dark-border bg-dark-card">
        {/* Definition Selector */}
        <select
          value={selectedDefId}
          onChange={(e) => handleLoadDefinition(e.target.value)}
          className="px-3 py-1.5 bg-dark-bg border border-dark-border rounded text-sm text-white focus:outline-none focus:border-blue-500"
        >
          <option value="">Select Definition...</option>
          {definitionsData?.definitions.map((def) => (
            <option key={def.Id} value={def.Id}>
              {def.Name || def.Id}
            </option>
          ))}
        </select>

        {/* Entity ID */}
        <input
          type="text"
          value={entityId}
          onChange={(e) => setEntityId(e.target.value)}
          placeholder="Entity ID"
          className="px-3 py-1.5 bg-dark-bg border border-dark-border rounded text-sm text-white w-40 focus:outline-none focus:border-blue-500"
        />

        {/* Execute Button */}
        <button
          onClick={handleExecute}
          disabled={!definition || !entityId.trim() || isExecuting}
          className={`px-4 py-1.5 rounded text-sm font-medium transition-colors flex items-center gap-2 ${
            !definition || !entityId.trim() || isExecuting
              ? 'bg-gray-600 text-gray-400 cursor-not-allowed'
              : 'bg-blue-600 text-white hover:bg-blue-700'
          }`}
        >
          {isExecuting ? (
            <>
              <span className="animate-spin">⟳</span>
              Running...
            </>
          ) : (
            <>▶ Execute</>
          )}
        </button>

        {/* Terminate Button (when running) */}
        {instanceId && executionData?.status === 'Running' && (
          <button
            onClick={handleTerminate}
            className="px-3 py-1.5 bg-red-600 text-white rounded text-sm font-medium hover:bg-red-700 transition-colors"
          >
            ⬛ Stop
          </button>
        )}

        {/* Reset Button */}
        <button
          onClick={clearExecution}
          disabled={!instanceId}
          className="px-3 py-1.5 bg-dark-bg border border-dark-border rounded text-sm text-gray-400 hover:text-white hover:border-gray-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          Reset
        </button>

        {/* Status Badge */}
        {executionData && (
          <div className="ml-auto">
            <StatusBadge status={executionData.status} size="md" />
          </div>
        )}
      </div>

      {/* Execution Progress Area */}
      <div className="flex-1 overflow-hidden">
        <ExecutionProgress
          definition={definition}
          instanceId={instanceId}
          status={executionData?.status || null}
          currentStep={executionData?.currentStep || null}
          executedSteps={executionData?.executedSteps || []}
          error={executionData?.error || null}
          onStepClick={setSelectedStepName}
          selectedStepName={selectedStepName}
        />
      </div>
    </div>
  );

  // Right Panel: Inspector
  const rightPanel = (
    <InspectorPanel
      instanceId={instanceId}
      definition={definition}
      executionData={executionData}
      workflowStatus={workflowStatus || null}
      inspectorTab={inspectorTab}
      selectedStepName={selectedStepName}
      onTabChange={setInspectorTab}
      onSendEvent={handleSendEvent}
    />
  );

  // Bottom Panel: Timeline
  const bottomPanel = (
    <div className="h-full overflow-auto p-3">
      {executionData?.executedSteps && executionData.executedSteps.length > 0 ? (
        <div className="flex items-center gap-2 overflow-x-auto pb-2">
          {executionData.executedSteps.map((step, i) => (
            <div key={i} className="flex items-center">
              <button
                onClick={() => setSelectedStepName(step.stepName)}
                className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
                  step.status === 'completed'
                    ? 'bg-green-600/20 text-green-400 border border-green-500/50'
                    : step.status === 'failed'
                      ? 'bg-red-600/20 text-red-400 border border-red-500/50'
                      : 'bg-blue-600/20 text-blue-400 border border-blue-500/50 animate-pulse'
                } ${selectedStepName === step.stepName ? 'ring-2 ring-white' : ''}`}
              >
                {step.stepName}
              </button>
              {i < executionData.executedSteps.length - 1 && (
                <div className="w-8 h-0.5 bg-dark-border mx-1" />
              )}
            </div>
          ))}
          {executionData.currentStep &&
            !executionData.executedSteps.find((s) => s.stepName === executionData.currentStep) && (
              <>
                <div className="w-8 h-0.5 bg-dark-border mx-1" />
                <div className="px-3 py-1 rounded-full text-xs font-medium bg-blue-600/20 text-blue-400 border border-blue-500/50 animate-pulse">
                  {executionData.currentStep}
                </div>
              </>
            )}
        </div>
      ) : (
        <p className="text-gray-500 text-sm text-center">
          Execute a workflow to see the timeline.
        </p>
      )}
    </div>
  );

  return (
    <div className="h-full">
      <ResizablePanels
        leftPanel={leftPanel}
        centerPanel={centerPanel}
        rightPanel={rightPanel}
        bottomPanel={bottomPanel}
        leftWidth={panelSizes.left}
        rightWidth={panelSizes.right}
        bottomHeight={panelSizes.bottom}
        onLeftWidthChange={(w) => setPanelSizes({ left: w })}
        onRightWidthChange={(w) => setPanelSizes({ right: w })}
        onBottomHeightChange={(h) => setPanelSizes({ bottom: h })}
        bottomExpanded={timelineExpanded}
        onBottomExpandedChange={setTimelineExpanded}
      />
    </div>
  );
}
