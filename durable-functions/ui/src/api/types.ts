export interface WorkflowListItem {
  InstanceId: string;
  Status: string;
  WorkflowType: string;
  CreatedAt: string;
  LastUpdatedAt: string;
}

export interface WorkflowListResponse {
  count: number;
  workflows: WorkflowListItem[];
}

export interface WorkflowInput {
  workflowType: string;
  entityId: string;
  idempotencyKey?: string;
  correlationId?: string;
  data?: Record<string, unknown>;
}

export interface WorkflowDetail {
  InstanceId: string;
  Status: string;
  WorkflowType: string;
  CreatedAt: string;
  LastUpdatedAt: string;
  Input?: unknown;
  Output?: unknown;
  CustomStatus?: unknown;
  FailureDetails?: string;
  runtimeStatus?: string; // Some API responses use this
}

export interface StartWorkflowRequest {
  workflowType: string;
  entityId: string;
  idempotencyKey?: string;
  data?: Record<string, unknown>;
}

export interface StartWorkflowResponse {
  InstanceId: string;
  StatusUri?: string;
  StartedAt?: string;
}

export interface WorkflowDefinition {
  id: string;
  version: string;
  name: string;
  description?: string;
  startAt: string;
  states: Record<string, WorkflowState>;
  config?: {
    timeoutSeconds?: number;
    defaultRetryPolicy?: RetryPolicy;
  };
  metadata?: {
    author?: string;
    tags?: string[];
  };
}

export interface WorkflowState {
  type: 'Task' | 'Choice' | 'Wait' | 'Parallel' | 'Succeed' | 'Fail';
  comment?: string;
  next?: string;
  end?: boolean;
  activity?: string;
  input?: Record<string, string>;
  resultPath?: string;
  choices?: ChoiceRule[];
  default?: string;
  output?: Record<string, string>;
}

export interface ChoiceRule {
  condition: {
    operator: string;
    variable: string;
    comparisonType: string;
    value: unknown;
  };
  next: string;
}

export interface RetryPolicy {
  maxAttempts: number;
  initialIntervalSeconds: number;
  backoffCoefficient: number;
}

// Workflow Definition Types (API uses PascalCase)
export interface WorkflowDefinitionSummary {
  Id: string;
  Name: string;
  Version: string;
  Description?: string;
  StateCount: number;
}

export interface WorkflowDefinitionListResponse {
  count: number;
  definitions: WorkflowDefinitionSummary[];
}

// Definition detail uses camelCase (serialized from C# model)
export interface WorkflowDefinitionDetail {
  id: string;
  name: string;
  version: string;
  description?: string;
  startAt: string;
  states: Record<string, WorkflowStateDetail>;
  config?: {
    timeoutSeconds?: number;
    compensationState?: string;
    defaultRetryPolicy?: RetryPolicy;
  };
  metadata?: {
    author?: string;
    tags?: string[];
  };
}

// State detail also uses camelCase
export interface WorkflowStateDetail {
  type?: string;
  comment?: string;
  next?: string;
  end?: boolean;
  activity?: string;
  input?: Record<string, unknown>;
  resultPath?: string;
  choices?: ChoiceRuleDetail[];
  default?: string;
  output?: Record<string, unknown>;
  externalEvent?: {
    eventName?: string;
    timeoutSeconds?: number;
    timeoutNext?: string;
    resultPath?: string;
  };
  compensateWith?: string;
  catch?: CatchDetail[];
  steps?: CompensationStepDetail[];
  finalState?: string;
  continueOnError?: boolean;
  error?: string;
  cause?: string;
}

export interface ChoiceRuleDetail {
  condition: {
    variable?: string;
    comparisonType?: string;
    value?: unknown;
    operator?: string;
  };
  next: string;
}

export interface CatchDetail {
  errors: string[];
  resultPath?: string;
  next: string;
}

export interface CompensationStepDetail {
  name: string;
  activity: string;
  input?: Record<string, unknown>;
  condition?: string;
}

export interface DefinitionVersionsResponse {
  definitionId: string;
  versions: string[];
}
