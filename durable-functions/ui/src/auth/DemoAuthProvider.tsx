import { createContext, useState, useEffect, type ReactNode } from 'react';

export interface DemoUser {
  id: string;
  name: string;
  email: string;
  role: 'admin' | 'viewer';
}

export interface AuthContextType {
  user: DemoUser | null;
  isAuthenticated: boolean;
  login: (userId: string) => void;
  logout: () => void;
}

export const AuthContext = createContext<AuthContextType | null>(null);

export const DEMO_USERS: DemoUser[] = [
  { id: '1', name: 'Demo Admin', email: 'admin@demo.local', role: 'admin' },
  { id: '2', name: 'Demo Viewer', email: 'viewer@demo.local', role: 'viewer' },
];

interface DemoAuthProviderProps {
  children: ReactNode;
}

export function DemoAuthProvider({ children }: DemoAuthProviderProps) {
  const [user, setUser] = useState<DemoUser | null>(null);

  const login = (userId: string) => {
    const demoUser = DEMO_USERS.find((u) => u.id === userId);
    if (demoUser) {
      setUser(demoUser);
      sessionStorage.setItem('demo-user', JSON.stringify(demoUser));
    }
  };

  const logout = () => {
    setUser(null);
    sessionStorage.removeItem('demo-user');
  };

  // Restore user on mount
  useEffect(() => {
    const stored = sessionStorage.getItem('demo-user');
    if (stored) {
      try {
        setUser(JSON.parse(stored));
      } catch {
        sessionStorage.removeItem('demo-user');
      }
    }
  }, []);

  return (
    <AuthContext.Provider value={{ user, login, logout, isAuthenticated: !!user }}>
      {children}
    </AuthContext.Provider>
  );
}
