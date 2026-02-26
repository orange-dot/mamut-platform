# Session Journal - Workflow Playground Implementation

**Date:** 2025-11-26
**Purpose:** Implement a comprehensive Playground page for workflow definition, execution, and visualization

---

## Session Overview

This session focused on building a full-featured Playground page that allows users to:
- Define workflows visually or via JSON
- Execute workflows against the real backend (no mocks)
- Visualize execution progress in real-time
- Inspect workflow state, variables, and results
- Send external events to running workflows

---

## 1. Planning Phase

### User Requirements
- **Definition Mode:** Both (load existing AND create new)
- **MVP Scope:** Full Featured (all features from start)
- **Panel Layout:** Resizable panels with draggable dividers

### Architecture Decisions
- Zustand for playground-specific state management
- ReactFlow for workflow visualization
- 2-second polling for real-time execution updates
- Three-panel layout (Definition | Execution | Inspector) + bottom timeline

---

## 2. Implementation Steps

### Step 1: Core Infrastructure
Created foundational components:

| File | Purpose |
|------|---------|
| `ui/src/stores/usePlaygroundStore.ts` | Zustand store with Definition, Execution, UI slices |
| `ui/src/components/playground/ResizablePanels.tsx` | Draggable three-panel layout |
| `ui/src/components/playground/PlaygroundPage.tsx` | Main page component |
| `ui/src/router.tsx` | Added `/playground` route |
| `ui/src/components/layout/Sidebar.tsx` | Added Playground nav item |

### Step 2: Definition Panel
Built workflow definition components:

| File | Purpose |
|------|---------|
| `PlaygroundVisualizer.tsx` | ReactFlow-based workflow diagram |
| `JsonEditor.tsx` | JSON editing with format/validate |
| `TemplateList.tsx` | 4 pre-built workflow templates |

**Templates Included:**
1. Simple Task - Single activity execution
2. Choice Branch - Conditional branching
3. Error Handling - With catch and retry
4. Wait for Event - External event pattern

### Step 3: Execution Panel
Created execution visualization:

| File | Purpose |
|------|---------|
| `ExecutionProgress.tsx` | Status, progress bar, step timeline |

**Features:**
- Progress percentage with animated bar
- Color-coded status indicators (Pending, Running, Completed, Failed)
- Clickable step entries for inspection
- Real-time current step highlighting
- Error and completion messages

### Step 4: Inspector Panel
Enhanced state inspection:

| File | Purpose |
|------|---------|
| `InspectorPanel.tsx` | 4-tab inspector panel |

**Tabs:**
1. **State** - Workflow status, current step, selected step details, input/output
2. **Variables** - Workflow variables display
3. **Results** - Step results with expandable JSON
4. **Events** - Send external events to running workflows

### Step 5: Toolbar (Integrated in Center Panel)
- Definition selector dropdown
- Entity ID input
- Execute button (with loading state)
- Terminate button (for running workflows)
- Reset button
- Status badge

### Step 6: Polish
- Keyboard shortcuts (Ctrl+Enter to execute, Esc to reset)
- Panel size persistence
- Animated status indicators
- Proper TypeScript typing throughout

---

## 3. Technical Challenges & Solutions

### Challenge 1: TypeScript `unknown` type errors
**Problem:** JSX conditional rendering with `unknown` typed values caused TS2322 errors
**Solution:** Use `!= null` checks instead of truthy checks for `unknown` fields:
```typescript
// Before (error):
{workflowStatus?.Output && (...)}

// After (fixed):
{workflowStatus?.Output != null && (...)}
```

### Challenge 2: API Type Differences
**Problem:** List API returns PascalCase, Detail API returns camelCase
**Solution:** Used separate hooks and state to handle both:
```typescript
const [selectedDefId, setSelectedDefId] = useState<string>('');
const { data: definitionDetail } = useDefinitionDetail(selectedDefId);
```

### Challenge 3: Real-time Execution Updates
**Problem:** Need live updates during workflow execution
**Solution:** Conditional polling with TanStack Query:
```typescript
const { data: workflowStatus } = useQuery({
  queryKey: ['playground-workflow', instanceId],
  queryFn: () => fetchWorkflowDetail(instanceId!),
  enabled: !!instanceId,
  refetchInterval: executionData?.status === 'Running' ? 2000 : false,
});
```

---

## 4. Files Created/Modified

### New Files (8)
```
ui/src/stores/usePlaygroundStore.ts
ui/src/components/playground/PlaygroundPage.tsx
ui/src/components/playground/ResizablePanels.tsx
ui/src/components/playground/PlaygroundVisualizer.tsx
ui/src/components/playground/JsonEditor.tsx
ui/src/components/playground/TemplateList.tsx
ui/src/components/playground/ExecutionProgress.tsx
ui/src/components/playground/InspectorPanel.tsx
```

### Modified Files (3)
```
ui/src/router.tsx - Added playground route
ui/src/components/layout/Sidebar.tsx - Added nav item
ui/src/components/playground/index.ts - Added exports
```

### Documentation Updated
```
docs/session-journal-2025-11-25.md - Added video link, fixed duration
```

---

## 5. Playground Features Summary

### Left Panel: Definition
- **Visual Tab:** ReactFlow diagram showing workflow states and transitions
- **JSON Tab:** Full JSON editor with format/minify/copy
- **Templates Tab:** Quick-start templates for common patterns

### Center Panel: Execution
- Toolbar with definition selector, entity ID input, execute/terminate/reset
- Real-time execution progress with:
  - Progress bar and percentage
  - Current step indicator
  - Step timeline with status icons
  - Error display
  - Completion message

### Right Panel: Inspector
- **State Tab:** Overall workflow state, selected step details
- **Variables Tab:** Runtime variables
- **Results Tab:** Step-by-step results
- **Events Tab:** Send external events to running workflows

### Bottom Panel: Timeline
- Horizontal step timeline with clickable entries
- Current step indicator (animated)
- Collapsible with toggle button

---

## 6. Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Enter` | Execute workflow |
| `Esc` | Reset/clear execution |

---

## 7. MCP Server Configuration

Configured Microsoft Playwright MCP server for UI testing:

**File:** `~/.claude/mcp.json`
```json
{
  "mcpServers": {
    "playwright": {
      "command": "npx",
      "args": ["@playwright/mcp@latest", "--browser", "chrome"]
    }
  }
}
```

**Features:**
- Headed mode (visible browser)
- Chrome browser
- Full browser automation capabilities

---

## 8. Access URLs

| URL | Description |
|-----|-------------|
| http://localhost:3000/playground | Playground page |
| http://localhost:3000 | Dashboard |
| http://localhost:7071/api/workflows | Workflows API |
| http://localhost:7071/api/definitions | Definitions API |

---

## 9. Next Steps

- [ ] Restart Claude Code to activate Playwright MCP
- [ ] Test playground with real workflow execution
- [ ] Add more workflow templates
- [ ] Consider adding workflow definition saving/loading
- [ ] Add execution history panel

---

## 10. Session 3: UI Testing & Bug Fixes

### Playwright E2E Testing
Implemented comprehensive UI testing with Playwright:

| File | Purpose |
|------|---------|
| `ui/e2e/quick-full-test.spec.ts` | Full UI walkthrough test |
| `ui/e2e/visual-test.spec.ts` | Visual regression tests |
| `ui/e2e/error-check.spec.ts` | Console error detection |
| `ui/e2e/workflow-detail.spec.ts` | Workflow detail page tests |
| `ui/e2e/start-workflow-manual.spec.ts` | Start workflow flow |
| `ui/playwright.config.ts` | Playwright configuration |
| `.github/workflows/ui-tests.yml` | CI/CD workflow for UI tests |
| `docs/testing-guide.md` | Testing documentation |

### Bug Fixes
1. **StartWorkflowFunction.cs** - Fixed instance ID return (was returning generated ID instead of actual Durable Functions ID)
2. **UI types.ts** - Fixed `StartWorkflowResponse` to use PascalCase `InstanceId` matching API
3. **CI workflow** - Fixed multiline connection string in YAML (was breaking Azurite connection)
4. **Integration tests** - Fixed blob storage roundtrip test

### Test Results
- **120 tests pass** (115 unit + 5 integration)
- **9 E2E tests pass** (Playwright)

### Session Recording
**Video:** [Session 3 Recording](https://kisoft80-my.sharepoint.com/:v:/g/personal/bojan_ki-soft_com/IQCl1MgcyYtyS5WlRrnVIAUiAQNUBKqnO3b3Yh_JHfHg70A?e=K9mbkt)
**Password:** `teska-prica`

---

**Session Duration:** ~3 hours (Session 1: ~2 hours, Session 3: ~1 hour)
**Components Created:** 8 new files + 8 test files
**Build Status:** All TypeScript checks passing, 120 tests passing
**Deployment:** Docker container rebuilt and running
