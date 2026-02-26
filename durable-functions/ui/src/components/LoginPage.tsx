import { useNavigate } from '@tanstack/react-router';
import { useAuth } from '../auth/useAuth';
import { DEMO_USERS } from '../auth/DemoAuthProvider';

export function LoginPage() {
  const { login } = useAuth();
  const navigate = useNavigate();

  // Note: Redirect is now handled by router beforeLoad

  const handleLogin = (userId: string) => {
    login(userId);
    navigate({ to: '/' });
  };

  return (
    <main
      className="min-h-screen flex items-center justify-center bg-dark-bg"
      role="main"
      aria-labelledby="login-heading"
    >
      <div className="bg-dark-card p-8 rounded-xl border border-dark-border max-w-md w-full">
        <header className="text-center mb-8">
          <h1 id="login-heading" className="text-2xl font-bold text-gray-100">
            Orchestration Studio
          </h1>
          <p className="text-gray-400 mt-2">
            Azure Durable Functions Demo
          </p>
        </header>

        <section aria-label="User selection">
          <p id="user-selection-label" className="text-sm text-gray-500 text-center mb-4">
            Select a demo user to continue:
          </p>

          <div className="space-y-4" role="group" aria-labelledby="user-selection-label">
            {DEMO_USERS.map((user) => (
              <button
                key={user.id}
                onClick={() => handleLogin(user.id)}
                className="w-full flex items-center gap-4 p-4 border border-dark-border rounded-lg hover:bg-dark-hover transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-dark-bg"
                aria-label={`Sign in as ${user.name}, ${user.role}`}
              >
                <div
                  className="w-10 h-10 bg-blue-900/50 rounded-full flex items-center justify-center"
                  aria-hidden="true"
                >
                  <span className="text-blue-400 font-semibold">
                    {user.name.charAt(0)}
                  </span>
                </div>
                <div className="text-left">
                  <div className="font-medium text-gray-200">{user.name}</div>
                  <div className="text-sm text-gray-500">
                    {user.email} • <span className="capitalize">{user.role}</span>
                  </div>
                </div>
              </button>
            ))}
          </div>
        </section>

        <aside className="mt-8 p-4 bg-blue-900/30 rounded-lg border border-blue-800" role="note">
          <p className="text-xs text-blue-300">
            <strong>Note:</strong> This is a demo authentication system.
            In production, this would integrate with Azure AD / Entra ID.
          </p>
        </aside>
      </div>
    </main>
  );
}
