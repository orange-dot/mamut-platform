import { useState, useEffect } from 'react';
import { useNavigate, Link } from '@tanstack/react-router';
import { useStartWorkflow } from '../../hooks/useWorkflows';
import { useDefinitions } from '../../hooks/useDefinitions';

export function StartWorkflow() {
  const navigate = useNavigate();
  const startWorkflow = useStartWorkflow();
  const { data: definitions, isLoading: loadingDefinitions } = useDefinitions();

  const [formData, setFormData] = useState({
    workflowType: '',
    entityId: '',
    idempotencyKey: '',
    data: '{}',
  });

  // Set default workflow type when definitions load
  useEffect(() => {
    if (definitions?.definitions.length && !formData.workflowType) {
      setFormData(prev => ({ ...prev, workflowType: definitions.definitions[0].Id }));
    }
  }, [definitions, formData.workflowType]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    try {
      let parsedData = {};
      try {
        parsedData = JSON.parse(formData.data);
      } catch {
        alert('Invalid JSON in data field');
        return;
      }

      const result = await startWorkflow.mutateAsync({
        workflowType: formData.workflowType,
        entityId: formData.entityId,
        idempotencyKey: formData.idempotencyKey || undefined,
        data: parsedData,
      });

      navigate({
        to: '/workflows/$instanceId',
        params: { instanceId: result.InstanceId },
      });
    } catch (error) {
      console.error('Failed to start workflow:', error);
    }
  };

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      <div className="flex items-center gap-4">
        <Link to="/workflows" className="text-gray-400 hover:text-gray-200">
          ← Back to list
        </Link>
      </div>

      <div>
        <h1 className="text-2xl font-bold text-gray-100">Start New Workflow</h1>
        <p className="text-gray-400">
          Configure and start a new workflow execution
        </p>
      </div>

      <form onSubmit={handleSubmit} className="bg-dark-card rounded-lg border border-dark-border p-6 space-y-6">
        <div>
          <label className="block text-sm font-medium text-gray-300 mb-1">
            Workflow Type
          </label>
          <select
            value={formData.workflowType}
            onChange={(e) => setFormData({ ...formData, workflowType: e.target.value })}
            disabled={loadingDefinitions}
            className="w-full px-3 py-2 bg-dark-bg border border-dark-border text-gray-200 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 disabled:opacity-50"
          >
            {loadingDefinitions && <option>Loading...</option>}
            {definitions?.definitions.map(def => (
              <option key={def.Id} value={def.Id}>
                {def.Name} ({def.Id})
              </option>
            ))}
            {!loadingDefinitions && !definitions?.definitions.length && (
              <option disabled>No workflow definitions available</option>
            )}
          </select>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-300 mb-1">
            Entity ID *
          </label>
          <input
            type="text"
            required
            value={formData.entityId}
            onChange={(e) => setFormData({ ...formData, entityId: e.target.value })}
            placeholder="e.g., device-001"
            className="w-full px-3 py-2 bg-dark-bg border border-dark-border text-gray-200 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 placeholder-gray-500"
          />
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-300 mb-1">
            Idempotency Key (optional)
          </label>
          <input
            type="text"
            value={formData.idempotencyKey}
            onChange={(e) => setFormData({ ...formData, idempotencyKey: e.target.value })}
            placeholder="Auto-generated if empty"
            className="w-full px-3 py-2 bg-dark-bg border border-dark-border text-gray-200 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 placeholder-gray-500"
          />
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-300 mb-1">
            Data (JSON)
          </label>
          <textarea
            value={formData.data}
            onChange={(e) => setFormData({ ...formData, data: e.target.value })}
            rows={6}
            className="w-full px-3 py-2 bg-dark-bg border border-dark-border text-gray-200 rounded-lg font-mono text-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
          />
        </div>

        <div className="flex gap-4">
          <button
            type="submit"
            disabled={startWorkflow.isPending}
            className="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
          >
            {startWorkflow.isPending ? 'Starting...' : 'Start Workflow'}
          </button>
          <Link
            to="/workflows"
            className="px-6 py-2 border border-dark-border text-gray-300 rounded-lg hover:bg-dark-hover"
          >
            Cancel
          </Link>
        </div>

        {startWorkflow.isError && (
          <div className="bg-red-900/30 text-red-400 p-4 rounded-lg border border-red-800">
            Error: {startWorkflow.error.message}
          </div>
        )}
      </form>
    </div>
  );
}
