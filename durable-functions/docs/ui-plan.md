# UI Demo Application Plan

## Overview

A React + TanStack-based demo UI to showcase the Azure Durable Functions orchestration system. Focus on security and demonstrating system capabilities rather than performance optimization.

---

## Technology Stack

| Category | Technology | Rationale |
|----------|------------|-----------|
| Framework | React 18+ | Industry standard, component-based |
| Build Tool | Vite | Fast dev server, optimized builds |
| Routing | TanStack Router | Type-safe routing, modern approach |
| Data Fetching | TanStack Query | Caching, auto-refresh, loading states |
| Forms | TanStack Form | Type-safe forms with validation |
| UI Components | shadcn/ui + Tailwind CSS | Clean, accessible components |
| Diagrams | React Flow | Interactive state machine visualization |
| Real-time | SignalR (@microsoft/signalr) | WebSocket-based real-time updates |
| HTTP Client | Axios | Interceptors, error handling |
| State | TanStack Query (server) + Zustand (client) | Separation of concerns |
| Auth | Demo mode (sessionStorage) | Simple auth for demo environment |

---

## Security Requirements

### Authentication
- Azure AD / Entra ID integration via MSAL.js
- JWT token validation
- Secure token storage (memory, not localStorage)
- Auto-refresh of tokens before expiry

### Authorization
- Role-based access control (RBAC)
- API calls include Bearer token
- Protected routes require authentication

### API Security
- HTTPS only (except local dev)
- CORS configuration
- Request/response sanitization
- No sensitive data in URL parameters

---

## Application Structure

```
ui/
├── src/
│   ├── main.tsx                 # Entry point
│   ├── App.tsx                  # Root component with providers
│   ├── auth/
│   │   ├── DemoAuthProvider.tsx # Demo authentication context
│   │   ├── ProtectedRoute.tsx   # Route guard component
│   │   └── useAuth.ts           # Auth hook
│   ├── api/
│   │   ├── client.ts            # Axios instance with interceptors
│   │   ├── workflows.ts         # Workflow API functions
│   │   ├── definitions.ts       # Definition API functions
│   │   ├── activities.ts        # Activity API functions
│   │   └── types.ts             # API type definitions
│   ├── routes/
│   │   ├── __root.tsx           # Root layout
│   │   ├── index.tsx            # Dashboard/Home
│   │   ├── login.tsx            # Login page
│   │   ├── workflows/
│   │   │   ├── index.tsx        # Workflow list
│   │   │   ├── $instanceId.tsx  # Workflow detail
│   │   │   └── new.tsx          # Start new workflow
│   │   ├── designer/
│   │   │   ├── index.tsx        # Designer list (recent/drafts)
│   │   │   ├── new.tsx          # Create new workflow
│   │   │   └── $workflowType.tsx # Edit existing workflow
│   │   ├── definitions/
│   │   │   ├── index.tsx        # Definitions list
│   │   │   └── $workflowType.tsx # Definition detail
│   │   └── activities/
│   │       ├── index.tsx        # Activity registry
│   │       └── $name.tsx        # Activity detail
│   ├── components/
│   │   ├── layout/
│   │   │   ├── Header.tsx
│   │   │   ├── Sidebar.tsx
│   │   │   └── MainLayout.tsx
│   │   ├── workflows/
│   │   │   ├── WorkflowList.tsx
│   │   │   ├── WorkflowCard.tsx
│   │   │   ├── WorkflowDetail.tsx
│   │   │   ├── WorkflowDiagram.tsx    # Read-only state machine
│   │   │   ├── StartWorkflowForm.tsx
│   │   │   └── RaiseEventForm.tsx
│   │   ├── designer/
│   │   │   ├── WorkflowDesigner.tsx   # Main designer canvas
│   │   │   ├── StateToolbox.tsx       # Draggable state types
│   │   │   ├── StateConfigPanel.tsx   # State property editor
│   │   │   ├── WorkflowConfigPanel.tsx # Global workflow settings
│   │   │   ├── ConditionBuilder.tsx   # Choice condition editor
│   │   │   ├── InputMappingEditor.tsx # JSONPath input mapping
│   │   │   ├── JsonPreview.tsx        # Live JSON output
│   │   │   ├── VersionHistory.tsx     # Definition versions
│   │   │   └── nodes/                 # Custom React Flow nodes
│   │   │       ├── TaskNode.tsx
│   │   │       ├── ChoiceNode.tsx
│   │   │       ├── WaitNode.tsx
│   │   │       ├── ParallelNode.tsx
│   │   │       ├── SucceedNode.tsx
│   │   │       └── FailNode.tsx
│   │   ├── definitions/
│   │   │   ├── DefinitionList.tsx
│   │   │   └── DefinitionViewer.tsx
│   │   ├── activities/
│   │   │   ├── ActivityList.tsx
│   │   │   ├── ActivityDetail.tsx
│   │   │   ├── ActivitySelector.tsx   # Dropdown for designer
│   │   │   └── ActivityTester.tsx     # Test execution form
│   │   └── common/
│   │       ├── LoadingSpinner.tsx
│   │       ├── ErrorBoundary.tsx
│   │       ├── StatusBadge.tsx
│   │       ├── JsonViewer.tsx
│   │       ├── JsonEditor.tsx         # Monaco-based editor
│   │       └── ConfirmDialog.tsx
│   ├── hooks/
│   │   ├── useWorkflows.ts
│   │   ├── useWorkflowDetail.ts
│   │   ├── useDefinitions.ts
│   │   ├── useActivities.ts
│   │   ├── useSignalR.ts
│   │   └── useDesigner.ts             # Designer state management
│   ├── stores/
│   │   └── designerStore.ts           # Zustand store for designer
│   └── lib/
│       ├── utils.ts
│       ├── constants.ts
│       ├── workflowGraph.ts           # Definition <-> ReactFlow conversion
│       └── validation.ts              # Workflow definition validation
├── public/
├── index.html
├── package.json
├── vite.config.ts
├── tailwind.config.js
└── tsconfig.json
```

---

## Pages & Features

### 1. Dashboard (Home)

**Purpose:** Overview of system status and recent activity

**Components:**
- System health indicators (API, Service Bus, Storage)
- Recent workflows summary (last 10)
- Quick actions (Start Workflow, View All)
- Statistics cards (Total, Running, Completed, Failed)

**Data:**
```typescript
// GET /api/workflows?limit=10
// GET /api/health (new endpoint needed)
```

### 2. Workflow List

**Purpose:** Browse and filter all workflow instances

**Features:**
- Paginated table with sorting
- Filter by status (Running, Completed, Failed, etc.)
- Search by instance ID or entity ID
- Auto-refresh toggle (polling every 5s)
- Click to view details

**Components:**
- `WorkflowList` - Main table component
- `WorkflowCard` - Card view option
- `StatusBadge` - Visual status indicator
- Filter/search controls

**Data:**
```typescript
// GET /api/workflows?status=Running&limit=20&offset=0
interface WorkflowListItem {
  instanceId: string;
  status: string;
  workflowType: string;
  createdAt: string;
  lastUpdatedAt: string;
}
```

### 3. Workflow Detail

**Purpose:** Deep dive into a specific workflow instance

**Features:**
- Current status and metadata
- Input/output JSON viewer
- Execution timeline (step-by-step)
- Step results with expandable details
- Raise event form (for waiting workflows)
- Terminate/Purge actions

**Components:**
- `WorkflowDetail` - Main detail view
- `WorkflowTimeline` - Visual step progression
- `JsonViewer` - Formatted JSON display
- `RaiseEventForm` - Send events to workflow

**Data:**
```typescript
// GET /api/workflows/{instanceId}
interface WorkflowDetail {
  instanceId: string;
  status: string;
  input: WorkflowInput;
  output: object | null;
  createdAt: string;
  lastUpdatedAt: string;
  stepResults: Record<string, object>;
}
```

### 4. Start New Workflow

**Purpose:** Initiate a new workflow execution

**Features:**
- Select workflow type from definitions
- Form with validation
- Entity ID input (required)
- Idempotency key (auto-generated or custom)
- Custom data JSON editor
- Submit and redirect to detail view

**Components:**
- `StartWorkflowForm` - Main form
- Workflow type selector
- JSON editor for custom data

**Data:**
```typescript
// POST /api/workflows
interface StartWorkflowRequest {
  workflowType: string;
  entityId: string;
  idempotencyKey?: string;
  data?: Record<string, unknown>;
}
```

### 5. Workflow Definitions (View)

**Purpose:** View available workflow definitions

**Features:**
- List all available workflow types
- View definition JSON
- Visual state machine diagram
- Version information

**Components:**
- `DefinitionList` - Available workflows
- `DefinitionViewer` - JSON/diagram view

**Data:**
```typescript
// GET /api/definitions
// GET /api/definitions/{workflowType}
```

---

### 6. Workflow Designer (Create/Edit)

**Purpose:** Visual workflow modeling and configuration

**Features:**
- Drag-and-drop state creation
- Visual connection of states (transitions)
- State configuration panel
- Activity selection and mapping
- Condition builder for Choice states
- JSON preview (live sync)
- Save/publish workflow definitions
- Version management

**Components:**
- `WorkflowDesigner` - Main canvas with React Flow
- `StateToolbox` - Draggable state types
- `StateConfigPanel` - Right sidebar for state properties
- `ActivitySelector` - Available activities dropdown
- `ConditionBuilder` - Visual condition editor
- `JsonPreview` - Live JSON output
- `VersionHistory` - Definition versions

**State Configuration Options:**
```typescript
interface StateConfig {
  // Common
  name: string;
  comment?: string;

  // Task State
  activity?: string;
  input?: Record<string, string>;  // JSONPath mappings
  resultPath?: string;
  retryPolicy?: RetryConfig;
  catch?: CatchConfig[];
  compensateWith?: string;

  // Choice State
  choices?: ChoiceRule[];
  default?: string;

  // Wait State
  waitType: 'seconds' | 'timestamp' | 'event';
  seconds?: number;
  timestamp?: string;
  eventName?: string;

  // Parallel State
  branches?: Branch[];

  // Succeed/Fail State
  output?: Record<string, string>;
  error?: string;
  cause?: string;
}
```

### 7. Workflow Configuration

**Purpose:** Configure workflow behavior and settings

**Features:**
- Timeout settings
- Default retry policies
- Notification settings
- Metadata and tags
- Environment-specific overrides

**Components:**
- `WorkflowConfigForm` - Main configuration form
- `RetryPolicyEditor` - Retry settings
- `TimeoutEditor` - Timeout configuration
- `MetadataEditor` - Tags and metadata

**Configuration Schema:**
```typescript
interface WorkflowConfig {
  id: string;
  name: string;
  description?: string;
  version: string;

  config: {
    timeoutSeconds: number;
    defaultRetryPolicy: {
      maxAttempts: number;
      initialIntervalSeconds: number;
      backoffCoefficient: number;
      maxIntervalSeconds?: number;
    };
    notifications?: {
      onStart?: NotificationConfig;
      onComplete?: NotificationConfig;
      onFail?: NotificationConfig;
    };
  };

  metadata: {
    author?: string;
    tags?: string[];
    category?: string;
    createdAt: string;
    updatedAt: string;
  };
}
```

### 8. Activity Registry

**Purpose:** Browse and configure available activities

**Features:**
- List all registered activities
- View activity input/output schemas
- Activity documentation
- Test activity execution (dry run)

**Components:**
- `ActivityList` - Available activities
- `ActivityDetail` - Schema and docs
- `ActivityTester` - Test execution form

**Data:**
```typescript
// GET /api/activities
// GET /api/activities/{name}
// POST /api/activities/{name}/test
interface ActivityInfo {
  name: string;
  description?: string;
  inputSchema: JsonSchema;
  outputSchema: JsonSchema;
  category?: string;
  retryable: boolean;
  compensatable: boolean;
}
```

---

## API Endpoints Required

### Existing Endpoints
| Method | Endpoint | Purpose |
|--------|----------|---------|
| GET | `/api/workflows` | List workflows |
| GET | `/api/workflows/{id}` | Get workflow detail |
| POST | `/api/workflows` | Start new workflow |
| POST | `/api/workflows/{id}/events/{name}` | Raise event |

### New Endpoints Needed
| Method | Endpoint | Purpose |
|--------|----------|---------|
| GET | `/api/health` | System health check |
| GET | `/api/definitions` | List workflow definitions |
| GET | `/api/definitions/{type}` | Get definition detail |
| POST | `/api/definitions` | Create new definition |
| PUT | `/api/definitions/{type}` | Update definition |
| DELETE | `/api/definitions/{type}` | Delete definition |
| GET | `/api/definitions/{type}/versions` | List definition versions |
| POST | `/api/definitions/{type}/publish` | Publish draft to active |
| POST | `/api/definitions/validate` | Validate definition JSON |
| GET | `/api/activities` | List registered activities |
| GET | `/api/activities/{name}` | Get activity schema/docs |
| POST | `/api/activities/{name}/test` | Test activity (dry run) |
| DELETE | `/api/workflows/{id}` | Terminate workflow |
| DELETE | `/api/workflows/{id}/purge` | Purge workflow history |

---

## Demo Authentication

Simple demo-mode authentication without Azure AD:

### Demo Auth Provider
```typescript
// DemoAuthProvider.tsx
interface DemoUser {
  id: string;
  name: string;
  email: string;
  role: 'admin' | 'viewer';
}

const DEMO_USERS: DemoUser[] = [
  { id: '1', name: 'Demo Admin', email: 'admin@demo.local', role: 'admin' },
  { id: '2', name: 'Demo Viewer', email: 'viewer@demo.local', role: 'viewer' },
];

export function DemoAuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<DemoUser | null>(null);

  const login = (userId: string) => {
    const demoUser = DEMO_USERS.find(u => u.id === userId);
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
    if (stored) setUser(JSON.parse(stored));
  }, []);

  return (
    <AuthContext.Provider value={{ user, login, logout, isAuthenticated: !!user }}>
      {children}
    </AuthContext.Provider>
  );
}
```

### Login Page
```typescript
// LoginPage.tsx - Simple user selector
export function LoginPage() {
  const { login } = useAuth();

  return (
    <div className="login-container">
      <h1>Orchestration Demo</h1>
      <p>Select a demo user to continue:</p>
      <button onClick={() => login('1')}>Login as Admin</button>
      <button onClick={() => login('2')}>Login as Viewer</button>
    </div>
  );
}
```

### API Client (No Token Required)
```typescript
// client.ts
const apiClient = axios.create({
  baseURL: import.meta.env.VITE_API_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});

// Add demo user header for audit logging
apiClient.interceptors.request.use((config) => {
  const user = sessionStorage.getItem('demo-user');
  if (user) {
    config.headers['X-Demo-User'] = JSON.parse(user).email;
  }
  return config;
});
```

---

## SignalR Real-time Updates

### Backend SignalR Hub

New Azure Function with SignalR bindings for real-time workflow updates:

```csharp
// WorkflowHub.cs
public class WorkflowHub : ServerlessHub
{
    [FunctionName("negotiate")]
    public SignalRConnectionInfo Negotiate(
        [HttpTrigger(AuthorizationLevel.Anonymous, "post")] HttpRequest req,
        [SignalRConnectionInfo(HubName = "workflows")] SignalRConnectionInfo connectionInfo)
    {
        return connectionInfo;
    }

    [FunctionName("BroadcastWorkflowUpdate")]
    public static Task BroadcastWorkflowUpdate(
        [SignalRTrigger("workflows", "messages", "workflowUpdate")] InvocationContext context,
        [SignalR(HubName = "workflows")] IAsyncCollector<SignalRMessage> signalRMessages,
        WorkflowUpdateMessage message)
    {
        return signalRMessages.AddAsync(new SignalRMessage
        {
            Target = "workflowUpdated",
            Arguments = new object[] { message }
        });
    }
}

// Trigger from orchestrator completion
[FunctionName("OnWorkflowStatusChange")]
public static async Task OnWorkflowStatusChange(
    [DurableClient] IDurableOrchestrationClient client,
    [SignalR(HubName = "workflows")] IAsyncCollector<SignalRMessage> signalRMessages,
    string instanceId, string status)
{
    await signalRMessages.AddAsync(new SignalRMessage
    {
        Target = "workflowUpdated",
        Arguments = new object[] { new { instanceId, status, timestamp = DateTime.UtcNow } }
    });
}
```

### Frontend SignalR Client

```typescript
// hooks/useSignalR.ts
import * as signalR from '@microsoft/signalr';

export function useWorkflowSignalR(onWorkflowUpdate: (data: WorkflowUpdate) => void) {
  const connectionRef = useRef<signalR.HubConnection | null>(null);

  useEffect(() => {
    const connection = new signalR.HubConnectionBuilder()
      .withUrl(`${import.meta.env.VITE_API_URL}/api`)
      .withAutomaticReconnect()
      .build();

    connection.on('workflowUpdated', (data: WorkflowUpdate) => {
      onWorkflowUpdate(data);
    });

    connection.start().catch(console.error);
    connectionRef.current = connection;

    return () => {
      connection.stop();
    };
  }, [onWorkflowUpdate]);

  return connectionRef.current;
}
```

### Integration with TanStack Query

```typescript
// hooks/useWorkflows.ts
export function useWorkflowsWithRealtime() {
  const queryClient = useQueryClient();

  // Subscribe to real-time updates
  useWorkflowSignalR((update) => {
    // Invalidate and refetch affected queries
    queryClient.invalidateQueries({ queryKey: ['workflows'] });
    queryClient.invalidateQueries({ queryKey: ['workflow', update.instanceId] });
  });

  return useQuery({
    queryKey: ['workflows'],
    queryFn: fetchWorkflows,
    staleTime: 30000, // 30 seconds - SignalR handles real-time
  });
}
```

### SignalR Events

| Event | Direction | Payload | Description |
|-------|-----------|---------|-------------|
| `workflowUpdated` | Server → Client | `{ instanceId, status, timestamp }` | Workflow status changed |
| `workflowStarted` | Server → Client | `{ instanceId, workflowType, entityId }` | New workflow started |
| `workflowCompleted` | Server → Client | `{ instanceId, output }` | Workflow finished |
| `activityCompleted` | Server → Client | `{ instanceId, activityName, result }` | Activity step done |

---

## Visual State Machine Diagram

### Component: WorkflowDiagram

Uses React Flow for interactive state machine visualization:

```typescript
// components/workflows/WorkflowDiagram.tsx
import ReactFlow, { Node, Edge, Background, Controls } from 'reactflow';
import 'reactflow/dist/style.css';

interface WorkflowDiagramProps {
  definition: WorkflowDefinition;
  currentState?: string;
  stepResults?: Record<string, unknown>;
}

export function WorkflowDiagram({ definition, currentState, stepResults }: WorkflowDiagramProps) {
  const { nodes, edges } = useMemo(() =>
    convertDefinitionToGraph(definition, currentState, stepResults),
    [definition, currentState, stepResults]
  );

  return (
    <div className="h-[500px] w-full border rounded-lg">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        fitView
        nodeTypes={customNodeTypes}
      >
        <Background />
        <Controls />
      </ReactFlow>
    </div>
  );
}
```

### State Node Types

```typescript
// components/workflows/nodes/StateNode.tsx
const nodeTypes = {
  task: TaskStateNode,      // Blue - Activity execution
  choice: ChoiceStateNode,  // Yellow - Decision point
  wait: WaitStateNode,      // Orange - Waiting for event
  parallel: ParallelStateNode, // Purple - Parallel execution
  succeed: SucceedStateNode,   // Green - Success terminal
  fail: FailStateNode,         // Red - Failure terminal
};

function TaskStateNode({ data }: NodeProps<TaskNodeData>) {
  const statusColor = {
    pending: 'bg-gray-100 border-gray-300',
    running: 'bg-blue-100 border-blue-500 animate-pulse',
    completed: 'bg-green-100 border-green-500',
    failed: 'bg-red-100 border-red-500',
  }[data.status];

  return (
    <div className={`px-4 py-2 rounded-lg border-2 ${statusColor}`}>
      <div className="font-medium">{data.label}</div>
      <div className="text-xs text-gray-500">{data.activity}</div>
      {data.result && (
        <div className="text-xs mt-1 text-green-600">✓ Completed</div>
      )}
    </div>
  );
}
```

### Definition to Graph Conversion

```typescript
// lib/workflowGraph.ts
export function convertDefinitionToGraph(
  definition: WorkflowDefinition,
  currentState?: string,
  stepResults?: Record<string, unknown>
): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = [];
  const edges: Edge[] = [];
  const stateNames = Object.keys(definition.states);

  // Layout: simple top-to-bottom
  stateNames.forEach((name, index) => {
    const state = definition.states[name];
    const isActive = name === currentState;
    const hasResult = stepResults && name in stepResults;

    nodes.push({
      id: name,
      type: state.type.toLowerCase(),
      position: { x: 250, y: index * 120 },
      data: {
        label: name,
        activity: state.activity,
        status: isActive ? 'running' : hasResult ? 'completed' : 'pending',
        result: stepResults?.[name],
        state,
      },
    });

    // Add edges
    if (state.next) {
      edges.push({
        id: `${name}-${state.next}`,
        source: name,
        target: state.next,
        animated: isActive,
      });
    }

    // Choice state edges
    if (state.type === 'Choice' && state.choices) {
      state.choices.forEach((choice, i) => {
        edges.push({
          id: `${name}-choice-${i}`,
          source: name,
          target: choice.next,
          label: `Condition ${i + 1}`,
        });
      });
      if (state.default) {
        edges.push({
          id: `${name}-default`,
          source: name,
          target: state.default,
          label: 'Default',
          style: { strokeDasharray: '5,5' },
        });
      }
    }
  });

  return { nodes, edges };
}
```

### Visual Legend

```
┌─────────────────────────────────────────┐
│  State Machine Legend                    │
├─────────────────────────────────────────┤
│  ▢ Gray     = Pending                   │
│  ▢ Blue     = Currently Running         │
│  ▢ Green    = Completed Successfully    │
│  ▢ Red      = Failed                    │
│  ◇ Yellow   = Choice/Decision           │
│  ⏸ Orange   = Waiting for Event         │
│  ═══►       = Transition                │
│  - - →      = Default Path              │
└─────────────────────────────────────────┘
```

---

## Workflow Designer (Detailed)

### Designer Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│  Header: Workflow Name ▼  |  Save Draft  |  Validate  |  Publish   │
├────────────┬────────────────────────────────────────┬───────────────┤
│            │                                        │               │
│  Toolbox   │         Designer Canvas               │  Properties   │
│            │         (React Flow)                  │    Panel      │
│  ┌──────┐  │                                        │               │
│  │ Task │  │    ┌─────────┐                        │  State: Task  │
│  └──────┘  │    │ Start   │                        │  ───────────  │
│  ┌──────┐  │    └────┬────┘                        │  Name: [    ] │
│  │Choice│  │         │                             │  Activity:    │
│  └──────┘  │    ┌────▼────┐                        │  [Select ▼]   │
│  ┌──────┐  │    │ Task 1  │                        │               │
│  │ Wait │  │    └────┬────┘                        │  Input:       │
│  └──────┘  │         │                             │  ┌──────────┐ │
│  ┌──────┐  │    ┌────▼────┐                        │  │ $.input. │ │
│  │Succeed│ │    │ Choice  │◇───► Branch A          │  └──────────┘ │
│  └──────┘  │    └────┬────┘                        │               │
│  ┌──────┐  │         │                             │  Result Path: │
│  │ Fail │  │    ┌────▼────┐                        │  [$.results. ]│
│  └──────┘  │    │ Succeed │                        │               │
│            │    └─────────┘                        │  Retry:       │
│            │                                        │  [Configure] │
├────────────┴────────────────────────────────────────┴───────────────┤
│  JSON Preview  │  Workflow Config  │  Validation Errors             │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │ {                                                               ││
│  │   "id": "my-workflow",                                         ││
│  │   "startAt": "Task1",                                          ││
│  │   "states": { ... }                                            ││
│  │ }                                                               ││
│  └─────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
```

### Designer Features

#### 1. State Toolbox
Drag-and-drop states onto canvas:
- **Task** - Execute an activity
- **Choice** - Decision branch
- **Wait** - Wait for time/event
- **Parallel** - Concurrent branches
- **Succeed** - Success terminal
- **Fail** - Failure terminal

#### 2. Canvas Interactions
- Drag to move states
- Connect states by dragging from output to input
- Click state to select and edit properties
- Double-click to rename
- Delete key to remove
- Undo/Redo support (Ctrl+Z / Ctrl+Y)
- Zoom and pan controls
- Auto-layout button

#### 3. State Configuration Panel
Dynamic form based on state type:

**Task State:**
```
┌─────────────────────────────────────┐
│ Task Configuration                  │
├─────────────────────────────────────┤
│ Name:     [GetUserData           ]  │
│ Comment:  [Fetch user from API   ]  │
│                                     │
│ Activity: [GetUserActivity      ▼]  │
│                                     │
│ Input Mapping:                      │
│ ┌─────────────────────────────────┐ │
│ │ userId: $.input.userId          │ │
│ │ + Add mapping                   │ │
│ └─────────────────────────────────┘ │
│                                     │
│ Result Path: [$.results.userData ]  │
│                                     │
│ ☑ Enable retry                      │
│   Max attempts: [3 ]                │
│   Initial delay: [1 ] seconds       │
│   Backoff: [2.0 ]                   │
│                                     │
│ Error Handling:                     │
│ [+ Add catch block               ]  │
│                                     │
│ Compensation:                       │
│ [Select activity              ▼]    │
└─────────────────────────────────────┘
```

**Choice State:**
```
┌─────────────────────────────────────┐
│ Choice Configuration                │
├─────────────────────────────────────┤
│ Name:     [CheckStatus           ]  │
│                                     │
│ Conditions:                         │
│ ┌─────────────────────────────────┐ │
│ │ Rule 1:                         │ │
│ │ Variable: [$.results.status   ] │ │
│ │ Operator: [Equals            ▼] │ │
│ │ Value:    [active              ] │ │
│ │ → Next:   [ProcessActive     ▼] │ │
│ │                          [Delete]│ │
│ └─────────────────────────────────┘ │
│ [+ Add condition                  ] │
│                                     │
│ Default: [HandleDefault        ▼]   │
└─────────────────────────────────────┘
```

#### 4. Condition Builder
Visual builder for choice conditions:
```typescript
// Supported operators
type ComparisonOperator =
  | 'Equals' | 'NotEquals'
  | 'GreaterThan' | 'GreaterThanOrEquals'
  | 'LessThan' | 'LessThanOrEquals'
  | 'Contains' | 'StartsWith' | 'EndsWith'
  | 'Exists' | 'NotExists'
  | 'IsNull' | 'IsNotNull';

// Compound conditions
type LogicalOperator = 'and' | 'or' | 'not';
```

#### 5. JSON Preview (Live Sync)
- Real-time JSON generation from canvas
- Syntax highlighting
- Copy to clipboard
- Toggle between JSON and YAML
- Bidirectional: edit JSON to update canvas

#### 6. Validation
Real-time validation with error markers:
- Missing required fields
- Invalid JSONPath expressions
- Unreachable states
- Missing transitions
- Circular dependencies
- Unknown activities

#### 7. Version Management
```
┌─────────────────────────────────────┐
│ Version History                     │
├─────────────────────────────────────┤
│ ● v1.2.0 (active) - 2 hours ago    │
│   "Added retry to payment step"     │
│                                     │
│ ○ v1.1.0 - Yesterday               │
│   "Fixed validation bug"            │
│                                     │
│ ○ v1.0.0 - 3 days ago              │
│   "Initial release"                 │
│                                     │
│ [Compare versions] [Rollback]       │
└─────────────────────────────────────┘
```

### Designer State Management (Zustand)

```typescript
// stores/designerStore.ts
interface DesignerState {
  // Workflow metadata
  workflowId: string | null;
  workflowName: string;
  isDirty: boolean;

  // React Flow state
  nodes: Node[];
  edges: Edge[];
  selectedNode: string | null;

  // Actions
  addState: (type: StateType, position: XYPosition) => void;
  updateState: (id: string, config: Partial<StateConfig>) => void;
  deleteState: (id: string) => void;
  connectStates: (source: string, target: string) => void;
  setSelectedNode: (id: string | null) => void;

  // Serialization
  toDefinition: () => WorkflowDefinition;
  fromDefinition: (def: WorkflowDefinition) => void;

  // History
  undo: () => void;
  redo: () => void;
  canUndo: boolean;
  canRedo: boolean;
}
```

---

## Development Phases

### Phase 1: Foundation (Frontend)
- [ ] Project setup (Vite + React + TypeScript)
- [ ] TanStack Router configuration
- [ ] TanStack Query setup
- [ ] Tailwind CSS + shadcn/ui installation
- [ ] Basic layout components (Header, Sidebar, MainLayout)
- [ ] API client setup

### Phase 2: Demo Authentication
- [ ] Demo auth provider and context
- [ ] Login page with user selector
- [ ] Protected route wrapper
- [ ] Session persistence
- [ ] Logout functionality

### Phase 3: Core Workflow Features
- [ ] Workflow list page with table
- [ ] Workflow detail page
- [ ] Start workflow form with validation
- [ ] Status badges component
- [ ] JSON viewer component

### Phase 4: Visual State Machine (Read-only)
- [ ] React Flow integration
- [ ] Custom node types (Task, Choice, Wait, Succeed, Fail)
- [ ] Definition to graph conversion
- [ ] Real-time state highlighting
- [ ] Interactive diagram controls (zoom, pan)

### Phase 5: Workflow Designer (Canvas)
- [ ] Designer page layout
- [ ] State toolbox (draggable items)
- [ ] React Flow drag-and-drop integration
- [ ] Node connection handling
- [ ] Auto-layout algorithm
- [ ] Undo/redo with history stack

### Phase 6: Workflow Designer (Configuration)
- [ ] State configuration panel
- [ ] Activity selector dropdown
- [ ] Input mapping editor (JSONPath)
- [ ] Condition builder for Choice states
- [ ] Retry policy editor
- [ ] Catch block configuration

### Phase 7: Workflow Designer (Persistence)
- [ ] JSON preview panel (live sync)
- [ ] Bidirectional JSON <-> Canvas sync
- [ ] Definition validation
- [ ] Save draft functionality
- [ ] Publish workflow
- [ ] Version history UI

### Phase 8: Activity Registry
- [ ] Activity list page
- [ ] Activity detail page (schema, docs)
- [ ] Activity tester form
- [ ] Activity selector integration in designer

### Phase 9: Backend API Endpoints
- [ ] GET /api/health - System health check
- [ ] GET /api/definitions - List workflow definitions
- [ ] GET /api/definitions/{type} - Get definition detail
- [ ] POST /api/definitions - Create definition
- [ ] PUT /api/definitions/{type} - Update definition
- [ ] POST /api/definitions/validate - Validate definition
- [ ] GET /api/definitions/{type}/versions - List versions
- [ ] GET /api/activities - List activities
- [ ] GET /api/activities/{name} - Get activity schema
- [ ] POST /api/activities/{name}/test - Test activity
- [ ] DELETE /api/workflows/{id} - Terminate workflow
- [ ] DELETE /api/workflows/{id}/purge - Purge workflow

### Phase 10: SignalR Real-time (Backend)
- [ ] Add SignalR NuGet package
- [ ] Create negotiate endpoint
- [ ] Workflow status change notifications
- [ ] Activity completion notifications
- [ ] Integration with orchestrator

### Phase 11: SignalR Real-time (Frontend)
- [ ] SignalR client setup
- [ ] useWorkflowSignalR hook
- [ ] TanStack Query invalidation on updates
- [ ] Connection status indicator
- [ ] Reconnection handling

### Phase 12: Polish & Integration
- [ ] Dashboard page with stats
- [ ] Raise event functionality
- [ ] Loading states and skeletons
- [ ] Error boundaries
- [ ] Keyboard shortcuts
- [ ] Docker integration for UI

---

## Docker Integration

Add UI service to `docker-compose.yml`:

```yaml
ui:
  build:
    context: ../ui
    dockerfile: Dockerfile
  ports:
    - "3000:80"
  environment:
    - VITE_API_URL=http://localhost:7071/api
    - VITE_AZURE_CLIENT_ID=${AZURE_CLIENT_ID:-demo}
    - VITE_AZURE_TENANT_ID=${AZURE_TENANT_ID:-demo}
  depends_on:
    - functions
  networks:
    - orchestration-network
```

For demo without Azure AD, include a "demo mode" that bypasses authentication.

---

## Decisions

| Question | Decision |
|----------|----------|
| Authentication | Demo mode without real Azure AD auth |
| Workflow Definitions | New API endpoints |
| Real-time Updates | SignalR WebSockets |
| State Machine Visualization | Visual diagram |
| Mobile Support | Desktop only |

---

## Estimated Effort

| Phase | Effort |
|-------|--------|
| Phase 1: Foundation (Frontend) | 3-4 hours |
| Phase 2: Demo Authentication | 2-3 hours |
| Phase 3: Core Workflow Features | 5-6 hours |
| Phase 4: Visual State Machine (Read-only) | 4-5 hours |
| Phase 5: Workflow Designer (Canvas) | 6-8 hours |
| Phase 6: Workflow Designer (Configuration) | 6-8 hours |
| Phase 7: Workflow Designer (Persistence) | 4-5 hours |
| Phase 8: Activity Registry | 3-4 hours |
| Phase 9: Backend API Endpoints | 6-8 hours |
| Phase 10: SignalR Backend | 3-4 hours |
| Phase 11: SignalR Frontend | 2-3 hours |
| Phase 12: Polish & Integration | 4-5 hours |
| **Total** | **48-63 hours** |

---

## Next Steps

1. Review and approve this plan
2. Answer open questions
3. Begin Phase 1 implementation
