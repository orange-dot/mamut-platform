import { useState, useCallback } from 'react';
import { StatusBadge } from '../common/StatusBadge';
import { JsonViewer } from '../common/JsonViewer';
import type { InspectorTab, ExecutedStep, PlaygroundExecutionData } from '../../stores/usePlaygroundStore';
import type { WorkflowDetail, WorkflowDefinitionDetail } from '../../api/types';

interface InspectorPanelProps {
  instanceId: string | null;
  definition: WorkflowDefinitionDetail | null;
  executionData: PlaygroundExecutionData | null;
  workflowStatus: WorkflowDetail | null;
  inspectorTab: InspectorTab;
  selectedStepName: string | null;
  onTabChange: (tab: InspectorTab) => void;
  onSendEvent: (eventName: string, payload: unknown) => Promise<void>;
}

const TABS: { id: InspectorTab; label: string; icon: string }[] = [
  { id: 'state', label: 'State', icon: '📊' },
  { id: 'variables', label: 'Vars', icon: '📝' },
  { id: 'results', label: 'Results', icon: '📦' },
  { id: 'events', label: 'Events', icon: '⚡' },
];

export function InspectorPanel({
  instanceId,
  definition,
  executionData,
  workflowStatus,
  inspectorTab,
  selectedStepName,
  onTabChange,
  onSendEvent,
}: InspectorPanelProps) {
  const [eventName, setEventName] = useState('');
  const [eventPayload, setEventPayload] = useState('{\n  \n}');
  const [eventError, setEventError] = useState<string | null>(null);
  const [sendingEvent, setSendingEvent] = useState(false);

  const handleSendEvent = useCallback(async () => {
    if (!eventName.trim()) {
      setEventError('Event name is required');
      return;
    }

    try {
      const payload = eventPayload.trim() ? JSON.parse(eventPayload) : {};
      setSendingEvent(true);
      setEventError(null);
      await onSendEvent(eventName.trim(), payload);
      setEventName('');
      setEventPayload('{\n  \n}');
    } catch (err) {
      setEventError((err as Error).message);
    } finally {
      setSendingEvent(false);
    }
  }, [eventName, eventPayload, onSendEvent]);

  // Get selected step details
  const selectedStep = executionData?.executedSteps?.find(
    (s: ExecutedStep) => s.stepName === selectedStepName
  );
  const selectedStepDef = definition?.states?.[selectedStepName || ''];
  const selectedStepResult = executionData?.stepResults?.[selectedStepName || ''];

  return (
    <div className="h-full flex flex-col">
      {/* Inspector Tabs */}
      <div className="flex border-b border-dark-border">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            onClick={() => onTabChange(tab.id)}
            className={`flex-1 px-2 py-2 text-xs font-medium transition-colors flex items-center justify-center gap-1 ${
              inspectorTab === tab.id
                ? 'text-blue-400 border-b-2 border-blue-400 bg-dark-hover'
                : 'text-gray-400 hover:text-white hover:bg-dark-hover'
            }`}
          >
            <span>{tab.icon}</span>
            <span>{tab.label}</span>
          </button>
        ))}
      </div>

      {/* Inspector Content */}
      <div className="flex-1 overflow-auto p-3">
        {!instanceId ? (
          <div className="text-center py-8">
            <p className="text-2xl mb-2">🔍</p>
            <p className="text-gray-500 text-sm">Execute a workflow to inspect its state.</p>
          </div>
        ) : (
          <>
            {/* State Tab */}
            {inspectorTab === 'state' && (
              <div className="space-y-4">
                {/* Workflow Info */}
                <div className="p-3 bg-dark-bg border border-dark-border rounded">
                  <div className="flex items-center justify-between mb-2">
                    <p className="text-xs text-gray-400">Status</p>
                    <StatusBadge status={executionData?.status || 'Unknown'} size="sm" />
                  </div>
                  {definition && (
                    <div className="text-xs space-y-1">
                      <p className="text-gray-400">
                        Workflow: <span className="text-white">{definition.name}</span>
                      </p>
                      <p className="text-gray-400">
                        Version: <span className="text-gray-300">{definition.version}</span>
                      </p>
                    </div>
                  )}
                </div>

                {/* Current Step */}
                {executionData?.currentStep && (
                  <div className="p-3 bg-blue-900/20 border border-blue-500/50 rounded">
                    <p className="text-xs text-blue-400 mb-1">Currently Executing</p>
                    <p className="text-sm font-medium text-white">{executionData.currentStep}</p>
                  </div>
                )}

                {/* Selected Step Info */}
                {selectedStepName && selectedStepDef && (
                  <div className="p-3 bg-dark-bg border border-gray-600 rounded">
                    <p className="text-xs text-gray-400 mb-2">Selected Step</p>
                    <p className="font-medium text-white">{selectedStepName}</p>
                    <div className="mt-2 text-xs space-y-1">
                      <p className="text-gray-400">
                        Type: <span className="text-gray-300">{selectedStepDef.type}</span>
                      </p>
                      {selectedStepDef.activity && (
                        <p className="text-gray-400">
                          Activity: <span className="text-gray-300">{selectedStepDef.activity}</span>
                        </p>
                      )}
                      {selectedStep && (
                        <>
                          <p className="text-gray-400">
                            Status:{' '}
                            <span
                              className={
                                selectedStep.status === 'completed'
                                  ? 'text-green-400'
                                  : selectedStep.status === 'failed'
                                    ? 'text-red-400'
                                    : 'text-blue-400'
                              }
                            >
                              {selectedStep.status}
                            </span>
                          </p>
                          {selectedStep.duration != null && (
                            <p className="text-gray-400">
                              Duration: <span className="text-gray-300">{selectedStep.duration}ms</span>
                            </p>
                          )}
                        </>
                      )}
                    </div>
                    {selectedStepResult != null && (
                      <div className="mt-3">
                        <p className="text-xs text-gray-400 mb-1">Result</p>
                        <JsonViewer data={selectedStepResult as Record<string, unknown>} initialExpanded={false} />
                      </div>
                    )}
                  </div>
                )}

                {/* Input */}
                {workflowStatus?.Input != null && (
                  <div>
                    <p className="text-xs text-gray-400 mb-1">Workflow Input</p>
                    <JsonViewer data={workflowStatus.Input as Record<string, unknown>} initialExpanded={false} />
                  </div>
                )}

                {/* Output */}
                {workflowStatus?.Output != null && (
                  <div>
                    <p className="text-xs text-gray-400 mb-1">Workflow Output</p>
                    <JsonViewer data={workflowStatus.Output as Record<string, unknown>} initialExpanded={false} />
                  </div>
                )}
              </div>
            )}

            {/* Variables Tab */}
            {inspectorTab === 'variables' && (
              <div>
                {Object.keys(executionData?.variables || {}).length > 0 ? (
                  <div className="space-y-2">
                    {Object.entries(executionData?.variables || {}).map(([key, value]) => (
                      <div key={key} className="p-2 bg-dark-bg border border-dark-border rounded">
                        <p className="text-xs text-blue-400 mb-1 font-mono">{key}</p>
                        <pre className="text-xs text-gray-300 overflow-auto">
                          {JSON.stringify(value, null, 2)}
                        </pre>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="text-center py-8">
                    <p className="text-2xl mb-2">📝</p>
                    <p className="text-gray-500 text-sm">No workflow variables set.</p>
                    <p className="text-gray-600 text-xs mt-1">
                      Variables are set by workflow states using resultPath
                    </p>
                  </div>
                )}
              </div>
            )}

            {/* Results Tab */}
            {inspectorTab === 'results' && (
              <div>
                {Object.keys(executionData?.stepResults || {}).length > 0 ? (
                  <div className="space-y-2">
                    {Object.entries(executionData?.stepResults || {}).map(([stepName, result]) => (
                      <div key={stepName} className="p-2 bg-dark-bg border border-dark-border rounded">
                        <div className="flex items-center justify-between mb-1">
                          <p className="text-xs font-medium text-white">{stepName}</p>
                          {executionData?.executedSteps?.find((s: ExecutedStep) => s.stepName === stepName)
                            ?.status === 'completed' && (
                            <span className="text-green-400 text-xs">✓</span>
                          )}
                        </div>
                        <pre className="text-xs text-gray-300 overflow-auto max-h-32">
                          {JSON.stringify(result, null, 2)}
                        </pre>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="text-center py-8">
                    <p className="text-2xl mb-2">📦</p>
                    <p className="text-gray-500 text-sm">No step results yet.</p>
                    <p className="text-gray-600 text-xs mt-1">
                      Results appear as steps complete execution
                    </p>
                  </div>
                )}
              </div>
            )}

            {/* Events Tab */}
            {inspectorTab === 'events' && (
              <div className="space-y-4">
                <div className="p-3 bg-dark-bg border border-dark-border rounded">
                  <p className="text-xs text-gray-400 mb-2">Send External Event</p>

                  <div className="space-y-3">
                    <div>
                      <label className="block text-xs text-gray-400 mb-1">Event Name</label>
                      <input
                        type="text"
                        value={eventName}
                        onChange={(e) => setEventName(e.target.value)}
                        placeholder="e.g., ApprovalReceived"
                        className="w-full px-2 py-1.5 bg-dark-card border border-dark-border rounded text-sm text-white focus:outline-none focus:border-blue-500"
                      />
                    </div>

                    <div>
                      <label className="block text-xs text-gray-400 mb-1">Payload (JSON)</label>
                      <textarea
                        value={eventPayload}
                        onChange={(e) => setEventPayload(e.target.value)}
                        placeholder='{"approved": true}'
                        className="w-full h-20 px-2 py-1.5 bg-dark-card border border-dark-border rounded text-sm font-mono text-white resize-none focus:outline-none focus:border-blue-500"
                      />
                    </div>

                    {eventError && (
                      <p className="text-xs text-red-400">{eventError}</p>
                    )}

                    <button
                      onClick={handleSendEvent}
                      disabled={executionData?.status !== 'Running' || sendingEvent}
                      className="w-full px-3 py-2 bg-blue-600 text-white rounded text-sm font-medium hover:bg-blue-700 transition-colors disabled:bg-gray-600 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                    >
                      {sendingEvent ? (
                        <>
                          <span className="animate-spin">⟳</span>
                          Sending...
                        </>
                      ) : (
                        <>
                          ⚡ Send Event
                        </>
                      )}
                    </button>

                    {executionData?.status !== 'Running' && (
                      <p className="text-xs text-gray-500 text-center">
                        Events can only be sent to running workflows
                      </p>
                    )}
                  </div>
                </div>

                {/* Event examples */}
                <div className="p-3 bg-dark-bg border border-dark-border rounded">
                  <p className="text-xs text-gray-400 mb-2">Quick Events</p>
                  <div className="flex flex-wrap gap-2">
                    {['approval', 'continue', 'cancel', 'retry'].map((eventType) => (
                      <button
                        key={eventType}
                        onClick={() => setEventName(eventType)}
                        className="px-2 py-1 text-xs bg-dark-hover border border-dark-border rounded hover:border-blue-500 text-gray-400 hover:text-white transition-colors"
                      >
                        {eventType}
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
