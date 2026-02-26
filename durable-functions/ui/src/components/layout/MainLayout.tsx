import { Outlet } from '@tanstack/react-router';
import { Header } from './Header';
import { Sidebar } from './Sidebar';

export function MainLayout() {
  return (
    <div className="min-h-screen flex flex-col">
      <Header />
      <div className="flex flex-1">
        <Sidebar />
        <main
          className="flex-1 p-6 overflow-auto"
          role="main"
          aria-label="Main content"
          id="main-content"
        >
          <Outlet />
        </main>
      </div>
    </div>
  );
}
