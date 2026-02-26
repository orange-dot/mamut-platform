import type { WorkflowDefinitionDetail } from '../../api/types';

interface Template {
  id: string;
  name: string;
  description: string;
  icon: string;
  definition: WorkflowDefinitionDetail;
}

// Sample templates for quick start
const TEMPLATES: Template[] = [
  {
    id: 'simple-task',
    name: 'Simple Task',
    description: 'A single task state that executes an activity',
    icon: '⚙️',
    definition: {
      id: 'template-simple-task',
      name: 'Simple Task Workflow',
      version: '1.0.0',
      description: 'A workflow with a single task state',
      startAt: 'ExecuteTask',
      states: {
        ExecuteTask: {
          type: 'Task',
          activity: 'ProcessData',
          end: true,
        },
      },
    },
  },
  {
    id: 'choice-branch',
    name: 'Choice Branch',
    description: 'A workflow with conditional branching',
    icon: '🔀',
    definition: {
      id: 'template-choice-branch',
      name: 'Choice Branch Workflow',
      version: '1.0.0',
      description: 'A workflow with conditional branching',
      startAt: 'CheckCondition',
      states: {
        CheckCondition: {
          type: 'Choice',
          choices: [
            {
              condition: {
                variable: '$.data.approved',
                comparisonType: 'BooleanEquals',
                value: true,
              },
              next: 'ProcessApproved',
            },
          ],
          default: 'ProcessRejected',
        },
        ProcessApproved: {
          type: 'Task',
          activity: 'HandleApproval',
          next: 'Complete',
        },
        ProcessRejected: {
          type: 'Task',
          activity: 'HandleRejection',
          next: 'Complete',
        },
        Complete: {
          type: 'Succeed',
        },
      },
    },
  },
  {
    id: 'error-handling',
    name: 'Error Handling',
    description: 'A workflow with error catch and retry',
    icon: '🛡️',
    definition: {
      id: 'template-error-handling',
      name: 'Error Handling Workflow',
      version: '1.0.0',
      description: 'A workflow demonstrating error handling',
      startAt: 'TryOperation',
      states: {
        TryOperation: {
          type: 'Task',
          activity: 'RiskyOperation',
          catch: [
            {
              errors: ['OperationError', 'TimeoutError'],
              resultPath: '$.error',
              next: 'HandleError',
            },
          ],
          next: 'Success',
        },
        HandleError: {
          type: 'Task',
          activity: 'LogError',
          next: 'Failure',
        },
        Success: {
          type: 'Succeed',
        },
        Failure: {
          type: 'Fail',
          error: 'WorkflowFailed',
          cause: 'Operation failed after error handling',
        },
      },
    },
  },
  {
    id: 'wait-event',
    name: 'Wait for Event',
    description: 'A workflow that waits for an external event',
    icon: '⏳',
    definition: {
      id: 'template-wait-event',
      name: 'Wait Event Workflow',
      version: '1.0.0',
      description: 'A workflow that waits for an external event',
      startAt: 'InitialTask',
      states: {
        InitialTask: {
          type: 'Task',
          activity: 'PrepareForEvent',
          next: 'WaitForApproval',
        },
        WaitForApproval: {
          type: 'Wait',
          externalEvent: {
            eventName: 'approval',
            timeoutSeconds: 3600,
            timeoutNext: 'Timeout',
            resultPath: '$.approvalResult',
          },
          next: 'ProcessApproval',
        },
        ProcessApproval: {
          type: 'Task',
          activity: 'CompleteApproval',
          next: 'Success',
        },
        Timeout: {
          type: 'Fail',
          error: 'TimeoutError',
          cause: 'Approval not received within timeout',
        },
        Success: {
          type: 'Succeed',
        },
      },
    },
  },
];

interface TemplateListProps {
  onSelect: (definition: WorkflowDefinitionDetail) => void;
}

export function TemplateList({ onSelect }: TemplateListProps) {
  return (
    <div className="h-full overflow-auto p-3">
      <p className="text-xs text-gray-400 mb-3">
        Select a template to start with a pre-defined workflow
      </p>
      <div className="space-y-2">
        {TEMPLATES.map((template) => (
          <button
            key={template.id}
            onClick={() => onSelect(template.definition)}
            className="w-full p-3 bg-dark-bg border border-dark-border rounded-lg hover:border-blue-500 hover:bg-dark-hover transition-colors text-left group"
          >
            <div className="flex items-start gap-3">
              <span className="text-2xl">{template.icon}</span>
              <div className="flex-1 min-w-0">
                <h4 className="font-medium text-white group-hover:text-blue-400 transition-colors">
                  {template.name}
                </h4>
                <p className="text-xs text-gray-400 mt-1">{template.description}</p>
                <div className="flex items-center gap-2 mt-2">
                  <span className="text-xs text-gray-500">
                    {Object.keys(template.definition.states).length} states
                  </span>
                  <span className="text-xs text-gray-600">•</span>
                  <span className="text-xs text-gray-500">
                    Start: {template.definition.startAt}
                  </span>
                </div>
              </div>
            </div>
          </button>
        ))}
      </div>

      <div className="mt-4 p-3 bg-dark-bg border border-dark-border rounded-lg">
        <p className="text-xs text-gray-400">
          💡 <strong>Tip:</strong> Templates are starting points. After selecting one, you can modify
          the JSON or use the visual editor to customize the workflow.
        </p>
      </div>
    </div>
  );
}
