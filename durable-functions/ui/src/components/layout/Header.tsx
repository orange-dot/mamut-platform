import { Link } from '@tanstack/react-router';
import { useAuth } from '../../auth/useAuth';

export function Header() {
  const { user, logout, isAuthenticated } = useAuth();

  return (
    <header className="bg-dark-card border-b border-dark-border px-6 py-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Link to="/" className="text-xl font-bold text-gray-100">
            Orchestration Studio
          </Link>
          <span className="text-xs bg-blue-900 text-blue-300 px-2 py-1 rounded">
            Demo
          </span>
        </div>

        <div className="flex items-center gap-4">
          {isAuthenticated && user && (
            <>
              <div className="text-sm text-gray-400">
                <span className="font-medium text-gray-200">{user.name}</span>
                <span className="mx-2">•</span>
                <span className="capitalize">{user.role}</span>
              </div>
              <button
                onClick={logout}
                className="text-sm text-gray-400 hover:text-gray-200"
              >
                Logout
              </button>
            </>
          )}
        </div>
      </div>
    </header>
  );
}
