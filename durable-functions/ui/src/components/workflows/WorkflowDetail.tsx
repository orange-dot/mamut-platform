import { useState } from 'react';
import { useParams, Link } from '@tanstack/react-router';
import { useWorkflowDetail, useTerminateWorkflow, useRaiseEvent } from '../../hooks/useWorkflows';
import { StatusBadge } from '../common/StatusBadge';
import { JsonViewer } from '../common/JsonViewer';
import { LoadingSpinner } from '../common/LoadingSpinner';

export function WorkflowDetail() {
  const { instanceId } = useParams({ strict: false }) as { instanceId: string };
  const { data, isLoading, error } = useWorkflowDetail(instanceId);
  const terminateWorkflow = useTerminateWorkflow();
  const raiseEvent = useRaiseEvent();

  const [showEventModal, setShowEventModal] = useState(false);
  const [eventName, setEventName] = useState('');
  const [eventData, setEventData] = useState('{}');
  const [showTerminateConfirm, setShowTerminateConfirm] = useState(false);

  const handleTerminate = async () => {
    try {
      await terminateWorkflow.mutateAsync(instanceId);
      setShowTerminateConfirm(false);
    } catch (err) {
      console.error('Failed to terminate workflow:', err);
    }
  };

  const handleRaiseEvent = async () => {
    try {
      let parsedData = {};
      try {
        parsedData = JSON.parse(eventData);
      } catch {
        alert('Invalid JSON in event data');
        return;
      }

      await raiseEvent.mutateAsync({
        instanceId,
        eventName,
        eventData: parsedData,
      });

      setShowEventModal(false);
      setEventName('');
      setEventData('{}');
    } catch (err) {
      console.error('Failed to raise event:', err);
    }
  };

  if (isLoading) {
    return (
      <div className="flex justify-center py-12">
        <LoadingSpinner />
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-900/30 text-red-400 p-4 rounded-lg border border-red-800">
        Error loading workflow: {error.message}
      </div>
    );
  }

  if (!data) {
    return (
      <div className="bg-yellow-900/30 text-yellow-400 p-4 rounded-lg border border-yellow-800">
        Workflow not found
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <Link
          to="/workflows"
          className="text-gray-400 hover:text-gray-200"
        >
          ← Back to list
        </Link>
      </div>

      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-100 font-mono">
            {data.InstanceId}
          </h1>
          <p className="text-gray-400">{data.WorkflowType}</p>
        </div>
        <StatusBadge status={data.Status} size="lg" />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Info Card */}
        <div className="bg-dark-card rounded-lg border border-dark-border p-6">
          <h2 className="text-lg font-semibold text-gray-100 mb-4">Details</h2>
          <dl className="space-y-2">
            <div className="flex justify-between">
              <dt className="text-gray-400">Created</dt>
              <dd className="text-gray-200">
                {new Date(data.CreatedAt).toLocaleString()}
              </dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-gray-400">Last Updated</dt>
              <dd className="text-gray-200">
                {new Date(data.LastUpdatedAt).toLocaleString()}
              </dd>
            </div>
            {data.FailureDetails && (
              <div className="flex justify-between">
                <dt className="text-gray-400">Failure</dt>
                <dd className="text-red-400 text-sm">
                  {data.FailureDetails}
                </dd>
              </div>
            )}
          </dl>
        </div>

        {/* Actions Card */}
        <div className="bg-dark-card rounded-lg border border-dark-border p-6">
          <h2 className="text-lg font-semibold text-gray-100 mb-4">Actions</h2>
          <div className="space-y-2">
            {data.Status === 'Running' && (
              <button
                onClick={() => setShowTerminateConfirm(true)}
                disabled={terminateWorkflow.isPending}
                className="w-full px-4 py-2 border border-red-800 text-red-400 rounded-lg hover:bg-red-900/30 disabled:opacity-50"
              >
                {terminateWorkflow.isPending ? 'Terminating...' : 'Terminate Workflow'}
              </button>
            )}
            {data.Status === 'Running' && (
              <button
                onClick={() => setShowEventModal(true)}
                className="w-full px-4 py-2 border border-dark-border text-gray-300 rounded-lg hover:bg-dark-hover"
              >
                Raise Event
              </button>
            )}
            {data.Status !== 'Running' && (
              <p className="text-gray-500 text-sm text-center py-2">
                No actions available for {data.Status.toLowerCase()} workflows
              </p>
            )}
          </div>

          {terminateWorkflow.isError && (
            <div className="mt-3 text-red-400 text-sm">
              Error: {terminateWorkflow.error.message}
            </div>
          )}
        </div>
      </div>

      {/* Input */}
      <div className="bg-dark-card rounded-lg border border-dark-border p-6">
        <h2 className="text-lg font-semibold text-gray-100 mb-4">Input</h2>
        <JsonViewer data={data.Input} />
      </div>

      {/* Output */}
      {data.Output !== undefined && data.Output !== null && (
        <div className="bg-dark-card rounded-lg border border-dark-border p-6">
          <h2 className="text-lg font-semibold text-gray-100 mb-4">Output</h2>
          <JsonViewer data={data.Output} />
        </div>
      )}

      {/* Custom Status */}
      {data.CustomStatus !== undefined && data.CustomStatus !== null && (
        <div className="bg-dark-card rounded-lg border border-dark-border p-6">
          <h2 className="text-lg font-semibold text-gray-100 mb-4">Custom Status</h2>
          <JsonViewer data={data.CustomStatus} />
        </div>
      )}

      {/* Terminate Confirmation Modal */}
      {showTerminateConfirm && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-dark-card rounded-lg border border-dark-border p-6 max-w-md w-full mx-4">
            <h3 className="text-lg font-semibold text-gray-100 mb-4">Confirm Termination</h3>
            <p className="text-gray-400 mb-6">
              Are you sure you want to terminate this workflow? This action cannot be undone.
            </p>
            <div className="flex gap-3 justify-end">
              <button
                onClick={() => setShowTerminateConfirm(false)}
                className="px-4 py-2 border border-dark-border text-gray-300 rounded-lg hover:bg-dark-hover"
              >
                Cancel
              </button>
              <button
                onClick={handleTerminate}
                disabled={terminateWorkflow.isPending}
                className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50"
              >
                {terminateWorkflow.isPending ? 'Terminating...' : 'Terminate'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Raise Event Modal */}
      {showEventModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-dark-card rounded-lg border border-dark-border p-6 max-w-lg w-full mx-4">
            <h3 className="text-lg font-semibold text-gray-100 mb-4">Raise Event</h3>

            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-1">
                  Event Name *
                </label>
                <input
                  type="text"
                  required
                  value={eventName}
                  onChange={(e) => setEventName(e.target.value)}
                  placeholder="e.g., approval, payment_received"
                  className="w-full px-3 py-2 bg-dark-bg border border-dark-border text-gray-200 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 placeholder-gray-500"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-300 mb-1">
                  Event Data (JSON)
                </label>
                <textarea
                  value={eventData}
                  onChange={(e) => setEventData(e.target.value)}
                  rows={4}
                  className="w-full px-3 py-2 bg-dark-bg border border-dark-border text-gray-200 rounded-lg font-mono text-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                />
              </div>

              {raiseEvent.isError && (
                <div className="text-red-400 text-sm">
                  Error: {raiseEvent.error.message}
                </div>
              )}
            </div>

            <div className="flex gap-3 justify-end mt-6">
              <button
                onClick={() => {
                  setShowEventModal(false);
                  setEventName('');
                  setEventData('{}');
                }}
                className="px-4 py-2 border border-dark-border text-gray-300 rounded-lg hover:bg-dark-hover"
              >
                Cancel
              </button>
              <button
                onClick={handleRaiseEvent}
                disabled={raiseEvent.isPending || !eventName.trim()}
                className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
              >
                {raiseEvent.isPending ? 'Sending...' : 'Send Event'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
