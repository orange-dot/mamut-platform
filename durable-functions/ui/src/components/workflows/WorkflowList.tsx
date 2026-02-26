import { Link } from '@tanstack/react-router';
import { useWorkflows } from '../../hooks/useWorkflows';
import { StatusBadge } from '../common/StatusBadge';
import { LoadingSpinner } from '../common/LoadingSpinner';

export function WorkflowList() {
  const { data, isLoading, error, refetch } = useWorkflows();

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-100">Workflows</h1>
          <p className="text-gray-400">
            {data?.count ?? 0} total workflows
          </p>
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => refetch()}
            className="px-4 py-2 border border-dark-border text-gray-300 rounded-lg hover:bg-dark-hover"
          >
            Refresh
          </button>
          <Link
            to="/workflows/new"
            className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
          >
            Start Workflow
          </Link>
        </div>
      </div>

      {isLoading && (
        <div className="flex justify-center py-12">
          <LoadingSpinner />
        </div>
      )}

      {error && (
        <div className="bg-red-900/30 text-red-400 p-4 rounded-lg border border-red-800">
          Error loading workflows: {error.message}
        </div>
      )}

      {data && data.workflows.length > 0 && (
        <div className="bg-dark-card rounded-lg border border-dark-border overflow-hidden">
          <table className="w-full">
            <thead className="bg-dark-hover">
              <tr>
                <th className="px-4 py-3 text-left text-sm font-medium text-gray-400">
                  Instance ID
                </th>
                <th className="px-4 py-3 text-left text-sm font-medium text-gray-400">
                  Status
                </th>
                <th className="px-4 py-3 text-left text-sm font-medium text-gray-400">
                  Type
                </th>
                <th className="px-4 py-3 text-left text-sm font-medium text-gray-400">
                  Created
                </th>
                <th className="px-4 py-3 text-left text-sm font-medium text-gray-400">
                  Updated
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-dark-border">
              {data.workflows.map((workflow) => (
                <tr key={workflow.InstanceId} className="hover:bg-dark-hover">
                  <td className="px-4 py-3">
                    <Link
                      to="/workflows/$instanceId"
                      params={{ instanceId: workflow.InstanceId }}
                      className="text-blue-400 hover:underline font-mono text-sm"
                    >
                      {workflow.InstanceId.slice(0, 12)}...
                    </Link>
                  </td>
                  <td className="px-4 py-3">
                    <StatusBadge status={workflow.Status} />
                  </td>
                  <td className="px-4 py-3 text-sm text-gray-300">
                    {workflow.WorkflowType}
                  </td>
                  <td className="px-4 py-3 text-sm text-gray-400">
                    {new Date(workflow.CreatedAt).toLocaleString()}
                  </td>
                  <td className="px-4 py-3 text-sm text-gray-400">
                    {new Date(workflow.LastUpdatedAt).toLocaleString()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {data && data.workflows.length === 0 && (
        <div className="bg-dark-card rounded-lg border border-dark-border p-12 text-center">
          <p className="text-gray-500">No workflows found.</p>
          <Link
            to="/workflows/new"
            className="mt-4 inline-block text-blue-400 hover:underline"
          >
            Start your first workflow
          </Link>
        </div>
      )}
    </div>
  );
}
