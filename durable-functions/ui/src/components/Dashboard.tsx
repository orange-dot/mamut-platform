import { Link } from '@tanstack/react-router';
import { useAuth } from '../auth/useAuth';
import { useWorkflows } from '../hooks/useWorkflows';
import { StatusBadge } from './common/StatusBadge';
import { LoadingSpinner } from './common/LoadingSpinner';

export function Dashboard() {
  const { user } = useAuth();
  const { data, isLoading } = useWorkflows();

  // Calculate stats from workflows
  const stats = {
    total: data?.workflows?.length ?? 0,
    running: data?.workflows?.filter(w => w.Status === 'Running').length ?? 0,
    completed: data?.workflows?.filter(w => w.Status === 'Completed').length ?? 0,
    failed: data?.workflows?.filter(w => w.Status === 'Failed').length ?? 0,
  };

  const recentWorkflows = data?.workflows?.slice(0, 5) ?? [];

  return (
    <article className="space-y-6" aria-labelledby="dashboard-heading">
      <header>
        <h1 id="dashboard-heading" className="text-2xl font-bold text-gray-100">Dashboard</h1>
        <p className="text-gray-400">
          Welcome back{user ? `, ${user.name}` : ''}!
        </p>
      </header>

      {/* Stats Cards */}
      <section aria-label="Workflow statistics">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4" role="group">
          <StatCard title="Total Workflows" value={isLoading ? '...' : stats.total} color="blue" />
          <StatCard title="Running" value={isLoading ? '...' : stats.running} color="yellow" />
          <StatCard title="Completed" value={isLoading ? '...' : stats.completed} color="green" />
          <StatCard title="Failed" value={isLoading ? '...' : stats.failed} color="red" />
        </div>
      </section>

      {/* Quick Actions */}
      <section className="bg-dark-card rounded-lg border border-dark-border p-6" aria-labelledby="quick-actions-heading">
        <h2 id="quick-actions-heading" className="text-lg font-semibold text-gray-100 mb-4">
          Quick Actions
        </h2>
        <nav className="flex gap-4" aria-label="Quick action links">
          <Link
            to="/workflows/new"
            className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-dark-bg"
          >
            Start New Workflow
          </Link>
          <button
            disabled
            className="px-4 py-2 border border-dark-border text-gray-500 rounded-lg cursor-not-allowed"
            title="Coming soon"
            aria-disabled="true"
          >
            Open Designer
          </button>
          <Link
            to="/workflows"
            className="px-4 py-2 border border-dark-border text-gray-300 rounded-lg hover:bg-dark-hover transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-dark-bg"
          >
            View All Workflows
          </Link>
        </nav>
      </section>

      {/* Recent Workflows */}
      <section className="bg-dark-card rounded-lg border border-dark-border p-6" aria-labelledby="recent-workflows-heading">
        <div className="flex justify-between items-center mb-4">
          <h2 id="recent-workflows-heading" className="text-lg font-semibold text-gray-100">
            Recent Workflows
          </h2>
          <Link to="/workflows" className="text-sm text-blue-400 hover:underline">
            View all
          </Link>
        </div>

        {isLoading ? (
          <div className="flex justify-center py-8" aria-busy="true" aria-label="Loading workflows">
            <LoadingSpinner />
          </div>
        ) : recentWorkflows.length === 0 ? (
          <p className="text-gray-500 text-center py-8">
            No workflows yet. Start one to see activity here.
          </p>
        ) : (
          <ul className="space-y-2" role="list" aria-label="Recent workflow list">
            {recentWorkflows.map((workflow) => (
              <li key={workflow.InstanceId}>
                <Link
                  to="/workflows/$instanceId"
                  params={{ instanceId: workflow.InstanceId }}
                  className="block p-3 rounded-lg hover:bg-dark-hover transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
                  aria-label={`View workflow ${workflow.InstanceId}, type ${workflow.WorkflowType}, status ${workflow.Status}`}
                >
                  <div className="flex items-center justify-between">
                    <div>
                      <div className="font-mono text-sm text-gray-200">
                        {workflow.InstanceId.substring(0, 20)}...
                      </div>
                      <div className="text-xs text-gray-500">
                        {workflow.WorkflowType} • {new Date(workflow.CreatedAt).toLocaleString()}
                      </div>
                    </div>
                    <StatusBadge status={workflow.Status} />
                  </div>
                </Link>
              </li>
            ))}
          </ul>
        )}
      </section>
    </article>
  );
}

interface StatCardProps {
  title: string;
  value: string | number;
  color: 'blue' | 'green' | 'yellow' | 'red';
}

function StatCard({ title, value, color }: StatCardProps) {
  const colorClasses = {
    blue: 'bg-blue-900/30 text-blue-400',
    green: 'bg-green-900/30 text-green-400',
    yellow: 'bg-yellow-900/30 text-yellow-400',
    red: 'bg-red-900/30 text-red-400',
  };

  return (
    <div
      className="bg-dark-card rounded-lg border border-dark-border p-4"
      role="group"
      aria-label={`${title}: ${value}`}
    >
      <div className="text-sm text-gray-400" id={`stat-${title.toLowerCase().replace(' ', '-')}`}>
        {title}
      </div>
      <div
        className={`text-2xl font-bold mt-1 ${colorClasses[color]}`}
        aria-labelledby={`stat-${title.toLowerCase().replace(' ', '-')}`}
      >
        {value}
      </div>
    </div>
  );
}
