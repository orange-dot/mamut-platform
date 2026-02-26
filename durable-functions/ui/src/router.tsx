import { lazy, Suspense } from 'react';
import { createRouter, createRootRoute, createRoute, redirect } from '@tanstack/react-router';
import { MainLayout } from './components/layout/MainLayout';
import { LoginPage } from './components/LoginPage';

// Lazy load heavy components for code splitting
const Dashboard = lazy(() => import('./components/Dashboard').then(m => ({ default: m.Dashboard })));
const WorkflowList = lazy(() => import('./components/workflows/WorkflowList').then(m => ({ default: m.WorkflowList })));
const WorkflowDetail = lazy(() => import('./components/workflows/WorkflowDetail').then(m => ({ default: m.WorkflowDetail })));
const StartWorkflow = lazy(() => import('./components/workflows/StartWorkflow').then(m => ({ default: m.StartWorkflow })));
const DefinitionList = lazy(() => import('./components/definitions/DefinitionList').then(m => ({ default: m.DefinitionList })));
const DefinitionDetail = lazy(() => import('./components/definitions/DefinitionDetail').then(m => ({ default: m.DefinitionDetail })));
const ActivityList = lazy(() => import('./components/activities/ActivityList').then(m => ({ default: m.ActivityList })));
const WorkflowDesigner = lazy(() => import('./components/designer/WorkflowDesigner').then(m => ({ default: m.WorkflowDesigner })));
const PlaygroundPage = lazy(() => import('./components/playground/PlaygroundPage').then(m => ({ default: m.PlaygroundPage })));
const SimulatorPage = lazy(() => import('./components/simulator/SimulatorPage').then(m => ({ default: m.SimulatorPage })));

// Loading fallback component
function PageLoader() {
  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      height: '100%',
      minHeight: '200px',
      color: 'var(--text-secondary)',
    }}>
      <div style={{ textAlign: 'center' }}>
        <div style={{
          width: '32px',
          height: '32px',
          border: '3px solid var(--border-color)',
          borderTopColor: 'var(--primary-color)',
          borderRadius: '50%',
          animation: 'spin 0.8s linear infinite',
          margin: '0 auto 1rem',
        }} />
        <div>Loading...</div>
      </div>
      <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}

// Wrapper for lazy components with Suspense
function withSuspense(Component: React.ComponentType) {
  return function SuspenseWrapper() {
    return (
      <Suspense fallback={<PageLoader />}>
        <Component />
      </Suspense>
    );
  };
}

// Auth check helper
function isAuthenticated() {
  const user = sessionStorage.getItem('demo-user');
  return !!user;
}

// Root route - no layout, just outlet
const rootRoute = createRootRoute();

// Login route - standalone page
const loginRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/login',
  component: LoginPage,
  beforeLoad: () => {
    if (isAuthenticated()) {
      throw redirect({ to: '/' });
    }
  },
});

// Protected layout route
const protectedRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: 'protected',
  component: MainLayout,
  beforeLoad: () => {
    if (!isAuthenticated()) {
      throw redirect({ to: '/login' });
    }
  },
});

// Dashboard - index route under protected
const indexRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/',
  component: withSuspense(Dashboard),
});

// Workflows routes
const workflowsRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/workflows',
  component: withSuspense(WorkflowList),
});

const startWorkflowRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/workflows/new',
  component: withSuspense(StartWorkflow),
});

const workflowDetailRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/workflows/$instanceId',
  component: withSuspense(WorkflowDetail),
});

// Definitions routes
const definitionsRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/definitions',
  component: withSuspense(DefinitionList),
});

const definitionDetailRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/definitions/$definitionId',
  component: withSuspense(DefinitionDetail),
});

// Activities route
const activitiesRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/activities',
  component: withSuspense(ActivityList),
});

// Designer route
const designerRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/designer',
  component: withSuspense(WorkflowDesigner),
});

// Playground route
const playgroundRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/playground',
  component: withSuspense(PlaygroundPage),
});

// Simulator route
const simulatorRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/simulator',
  component: withSuspense(SimulatorPage),
});

// Route tree
const routeTree = rootRoute.addChildren([
  loginRoute,
  protectedRoute.addChildren([
    indexRoute,
    workflowsRoute,
    startWorkflowRoute,
    workflowDetailRoute,
    definitionsRoute,
    definitionDetailRoute,
    activitiesRoute,
    designerRoute,
    playgroundRoute,
    simulatorRoute,
  ]),
]);

// Create router
export const router = createRouter({ routeTree });

// Type declaration for router
declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
