import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { WorkflowDefinitionDetail } from '../api/types';

// Execution step tracking
export interface ExecutedStep {
  stepName: string;
  stepType: string;
  executedAt: string;
  duration?: number;
  status: 'completed' | 'failed' | 'running';
}

// Parsed execution data
export interface PlaygroundExecutionData {
  status: string;
  currentStep: string | null;
  executedSteps: ExecutedStep[];
  stepResults: Record<string, unknown>;
  variables: Record<string, unknown>;
  error: { message: string; code: string } | null;
}

// Inspector tab types
export type InspectorTab = 'state' | 'variables' | 'results' | 'events';

// Definition tab types
export type DefinitionTab = 'visual' | 'json' | 'templates';

// Panel sizes
export interface PanelSizes {
  left: number;
  right: number;
  bottom: number;
}

// Store state interface
interface PlaygroundState {
  // Definition state
  definition: WorkflowDefinitionDetail | null;
  definitionJson: string;
  definitionTab: DefinitionTab;
  isDirty: boolean;
  validationErrors: string[];

  // Execution state
  instanceId: string | null;
  entityId: string;
  inputData: string;
  executionData: PlaygroundExecutionData | null;
  isExecuting: boolean;
  executionHistory: Array<{
    instanceId: string;
    workflowName: string;
    startedAt: string;
    status: string;
  }>;

  // UI state
  inspectorTab: InspectorTab;
  panelSizes: PanelSizes;
  timelineExpanded: boolean;
  selectedStepName: string | null;

  // Definition actions
  setDefinition: (def: WorkflowDefinitionDetail | null) => void;
  setDefinitionJson: (json: string) => void;
  setDefinitionTab: (tab: DefinitionTab) => void;
  setValidationErrors: (errors: string[]) => void;
  markDirty: () => void;
  markClean: () => void;
  resetDefinition: () => void;

  // Execution actions
  setInstanceId: (id: string | null) => void;
  setEntityId: (id: string) => void;
  setInputData: (data: string) => void;
  setExecutionData: (data: PlaygroundExecutionData | null) => void;
  setIsExecuting: (executing: boolean) => void;
  addToHistory: (entry: { instanceId: string; workflowName: string; status: string }) => void;
  clearExecution: () => void;

  // UI actions
  setInspectorTab: (tab: InspectorTab) => void;
  setPanelSizes: (sizes: Partial<PanelSizes>) => void;
  setTimelineExpanded: (expanded: boolean) => void;
  setSelectedStepName: (name: string | null) => void;
}

const DEFAULT_PANEL_SIZES: PanelSizes = {
  left: 350,
  right: 300,
  bottom: 120,
};

const DEFAULT_INPUT_DATA = '{\n  \n}';

export const usePlaygroundStore = create<PlaygroundState>()(
  persist(
    (set, get) => ({
      // Initial definition state
      definition: null,
      definitionJson: '',
      definitionTab: 'visual',
      isDirty: false,
      validationErrors: [],

      // Initial execution state
      instanceId: null,
      entityId: '',
      inputData: DEFAULT_INPUT_DATA,
      executionData: null,
      isExecuting: false,
      executionHistory: [],

      // Initial UI state
      inspectorTab: 'state',
      panelSizes: DEFAULT_PANEL_SIZES,
      timelineExpanded: true,
      selectedStepName: null,

      // Definition actions
      setDefinition: (def) => {
        set({
          definition: def,
          isDirty: false,
          definitionJson: def ? JSON.stringify(def, null, 2) : '',
          validationErrors: [],
        });
      },

      setDefinitionJson: (json) => {
        set({ definitionJson: json, isDirty: true });
      },

      setDefinitionTab: (tab) => {
        set({ definitionTab: tab });
      },

      setValidationErrors: (errors) => {
        set({ validationErrors: errors });
      },

      markDirty: () => {
        set({ isDirty: true });
      },

      markClean: () => {
        set({ isDirty: false });
      },

      resetDefinition: () => {
        set({
          definition: null,
          definitionJson: '',
          isDirty: false,
          validationErrors: [],
        });
      },

      // Execution actions
      setInstanceId: (id) => {
        set({ instanceId: id });
      },

      setEntityId: (id) => {
        set({ entityId: id });
      },

      setInputData: (data) => {
        set({ inputData: data });
      },

      setExecutionData: (data) => {
        set({ executionData: data });
      },

      setIsExecuting: (executing) => {
        set({ isExecuting: executing });
      },

      addToHistory: (entry) => {
        const history = get().executionHistory;
        const newEntry = {
          ...entry,
          startedAt: new Date().toISOString(),
        };
        // Keep last 10 executions
        const newHistory = [newEntry, ...history].slice(0, 10);
        set({ executionHistory: newHistory });
      },

      clearExecution: () => {
        set({
          instanceId: null,
          executionData: null,
          isExecuting: false,
          selectedStepName: null,
        });
      },

      // UI actions
      setInspectorTab: (tab) => {
        set({ inspectorTab: tab });
      },

      setPanelSizes: (sizes) => {
        set((state) => ({
          panelSizes: { ...state.panelSizes, ...sizes },
        }));
      },

      setTimelineExpanded: (expanded) => {
        set({ timelineExpanded: expanded });
      },

      setSelectedStepName: (name) => {
        set({ selectedStepName: name });
      },
    }),
    {
      name: 'playground-storage',
      partialize: (state) => ({
        // Only persist UI preferences and entity ID
        panelSizes: state.panelSizes,
        timelineExpanded: state.timelineExpanded,
        entityId: state.entityId,
        executionHistory: state.executionHistory,
      }),
    }
  )
);

// Selector hooks for optimized re-renders
export const useDefinition = () => usePlaygroundStore((state) => state.definition);
export const useDefinitionJson = () => usePlaygroundStore((state) => state.definitionJson);
export const useDefinitionTab = () => usePlaygroundStore((state) => state.definitionTab);
export const useExecutionData = () => usePlaygroundStore((state) => state.executionData);
export const useInstanceId = () => usePlaygroundStore((state) => state.instanceId);
export const usePanelSizes = () => usePlaygroundStore((state) => state.panelSizes);
export const useInspectorTab = () => usePlaygroundStore((state) => state.inspectorTab);
export const useSelectedStepName = () => usePlaygroundStore((state) => state.selectedStepName);
