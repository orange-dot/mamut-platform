import { Link } from '@tanstack/react-router';
import { useDefinitions } from '../../hooks/useDefinitions';
import { LoadingSpinner } from '../common/LoadingSpinner';

export function DefinitionList() {
  const { data, isLoading, error, refetch } = useDefinitions();

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-100">Workflow Definitions</h1>
          <p className="text-gray-400">
            {data?.count ?? 0} workflow types available
          </p>
        </div>
        <button
          onClick={() => refetch()}
          className="px-4 py-2 border border-dark-border text-gray-300 rounded-lg hover:bg-dark-hover"
        >
          Refresh
        </button>
      </div>

      {isLoading && (
        <div className="flex justify-center py-12">
          <LoadingSpinner />
        </div>
      )}

      {error && (
        <div className="bg-red-900/30 text-red-400 p-4 rounded-lg border border-red-800">
          Error loading definitions: {error.message}
        </div>
      )}

      {data && data.definitions.length > 0 && (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {data.definitions.map((definition) => (
            <Link
              key={definition.Id}
              to="/definitions/$definitionId"
              params={{ definitionId: definition.Id }}
              className="block bg-dark-card rounded-lg border border-dark-border p-6 hover:border-blue-800 transition-colors"
            >
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <h3 className="text-lg font-semibold text-gray-100 truncate">
                    {definition.Name}
                  </h3>
                  <p className="text-sm text-gray-500 font-mono">{definition.Id}</p>
                </div>
                <span className="ml-2 px-2 py-1 bg-blue-900/50 text-blue-400 text-xs rounded-full">
                  v{definition.Version}
                </span>
              </div>

              {definition.Description && (
                <p className="mt-3 text-sm text-gray-400 line-clamp-2">
                  {definition.Description}
                </p>
              )}

              <div className="mt-4 flex items-center text-sm text-gray-500">
                <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
                </svg>
                {definition.StateCount} states
              </div>
            </Link>
          ))}
        </div>
      )}

      {data && data.definitions.length === 0 && (
        <div className="bg-dark-card rounded-lg border border-dark-border p-12 text-center">
          <p className="text-gray-500">No workflow definitions found.</p>
        </div>
      )}
    </div>
  );
}
