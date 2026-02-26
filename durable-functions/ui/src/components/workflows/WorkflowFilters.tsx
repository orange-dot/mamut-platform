import { useState, useCallback } from 'react';
import styles from './WorkflowFilters.module.css';

export interface WorkflowFilters {
  search: string;
  status: string[];
  workflowType: string;
  dateRange: {
    from: string;
    to: string;
  };
}

interface WorkflowFiltersProps {
  filters: WorkflowFilters;
  onChange: (filters: WorkflowFilters) => void;
  workflowTypes: string[];
}

const STATUS_OPTIONS = [
  { value: 'Running', label: 'Running', color: 'var(--info-color)' },
  { value: 'Completed', label: 'Completed', color: 'var(--success-color)' },
  { value: 'Failed', label: 'Failed', color: 'var(--error-color)' },
  { value: 'Pending', label: 'Pending', color: 'var(--warning-color)' },
  { value: 'Terminated', label: 'Terminated', color: 'var(--text-tertiary)' },
  { value: 'Canceled', label: 'Canceled', color: 'var(--text-tertiary)' },
];

export function WorkflowFiltersComponent({ filters, onChange, workflowTypes }: WorkflowFiltersProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  const handleSearchChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    onChange({ ...filters, search: e.target.value });
  }, [filters, onChange]);

  const handleStatusToggle = useCallback((status: string) => {
    const newStatuses = filters.status.includes(status)
      ? filters.status.filter(s => s !== status)
      : [...filters.status, status];
    onChange({ ...filters, status: newStatuses });
  }, [filters, onChange]);

  const handleWorkflowTypeChange = useCallback((e: React.ChangeEvent<HTMLSelectElement>) => {
    onChange({ ...filters, workflowType: e.target.value });
  }, [filters, onChange]);

  const handleDateChange = useCallback((field: 'from' | 'to', value: string) => {
    onChange({
      ...filters,
      dateRange: { ...filters.dateRange, [field]: value },
    });
  }, [filters, onChange]);

  const handleClear = useCallback(() => {
    onChange({
      search: '',
      status: [],
      workflowType: '',
      dateRange: { from: '', to: '' },
    });
  }, [onChange]);

  const hasActiveFilters =
    filters.search ||
    filters.status.length > 0 ||
    filters.workflowType ||
    filters.dateRange.from ||
    filters.dateRange.to;

  return (
    <div className={styles.container}>
      <div className={styles.mainRow}>
        <div className={styles.searchContainer}>
          <span className={styles.searchIcon}>🔍</span>
          <input
            type="text"
            placeholder="Search by instance ID, entity ID..."
            value={filters.search}
            onChange={handleSearchChange}
            className={styles.searchInput}
            aria-label="Search workflows"
          />
          {filters.search && (
            <button
              onClick={() => onChange({ ...filters, search: '' })}
              className={styles.clearButton}
              aria-label="Clear search"
            >
              ✕
            </button>
          )}
        </div>

        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className={`${styles.filterToggle} ${isExpanded ? styles.active : ''}`}
          aria-expanded={isExpanded}
        >
          <span>Filters</span>
          {hasActiveFilters && <span className={styles.filterBadge}>{
            (filters.status.length || 0) +
            (filters.workflowType ? 1 : 0) +
            (filters.dateRange.from || filters.dateRange.to ? 1 : 0)
          }</span>}
          <span className={styles.chevron}>{isExpanded ? '▲' : '▼'}</span>
        </button>

        {hasActiveFilters && (
          <button onClick={handleClear} className={styles.clearAllButton}>
            Clear all
          </button>
        )}
      </div>

      {isExpanded && (
        <div className={styles.expandedFilters}>
          <div className={styles.filterGroup}>
            <label className={styles.filterLabel}>Status</label>
            <div className={styles.statusFilters}>
              {STATUS_OPTIONS.map(option => (
                <button
                  key={option.value}
                  onClick={() => handleStatusToggle(option.value)}
                  className={`${styles.statusButton} ${filters.status.includes(option.value) ? styles.selected : ''}`}
                  style={{ '--status-color': option.color } as React.CSSProperties}
                >
                  <span className={styles.statusDot} />
                  {option.label}
                </button>
              ))}
            </div>
          </div>

          <div className={styles.filterGroup}>
            <label className={styles.filterLabel} htmlFor="workflow-type">Workflow Type</label>
            <select
              id="workflow-type"
              value={filters.workflowType}
              onChange={handleWorkflowTypeChange}
              className={styles.select}
            >
              <option value="">All types</option>
              {workflowTypes.map(type => (
                <option key={type} value={type}>{type}</option>
              ))}
            </select>
          </div>

          <div className={styles.filterGroup}>
            <label className={styles.filterLabel}>Date Range</label>
            <div className={styles.dateInputs}>
              <input
                type="datetime-local"
                value={filters.dateRange.from}
                onChange={e => handleDateChange('from', e.target.value)}
                className={styles.dateInput}
                aria-label="From date"
              />
              <span className={styles.dateSeparator}>to</span>
              <input
                type="datetime-local"
                value={filters.dateRange.to}
                onChange={e => handleDateChange('to', e.target.value)}
                className={styles.dateInput}
                aria-label="To date"
              />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// Hook for filtering workflows
export function useWorkflowFilters() {
  const [filters, setFilters] = useState<WorkflowFilters>({
    search: '',
    status: [],
    workflowType: '',
    dateRange: { from: '', to: '' },
  });

  const filterWorkflows = useCallback(<T extends {
    instanceId?: string;
    name?: string;
    runtimeStatus?: string;
    workflowType?: string;
    createdTime?: string;
  }>(workflows: T[]): T[] => {
    return workflows.filter(wf => {
      // Search filter
      if (filters.search) {
        const search = filters.search.toLowerCase();
        const matchesSearch =
          wf.instanceId?.toLowerCase().includes(search) ||
          wf.name?.toLowerCase().includes(search);
        if (!matchesSearch) return false;
      }

      // Status filter
      if (filters.status.length > 0) {
        if (!wf.runtimeStatus || !filters.status.includes(wf.runtimeStatus)) {
          return false;
        }
      }

      // Workflow type filter
      if (filters.workflowType) {
        if (wf.workflowType !== filters.workflowType) {
          return false;
        }
      }

      // Date range filter
      if (filters.dateRange.from || filters.dateRange.to) {
        const createdTime = wf.createdTime ? new Date(wf.createdTime) : null;
        if (createdTime) {
          if (filters.dateRange.from && createdTime < new Date(filters.dateRange.from)) {
            return false;
          }
          if (filters.dateRange.to && createdTime > new Date(filters.dateRange.to)) {
            return false;
          }
        }
      }

      return true;
    });
  }, [filters]);

  return { filters, setFilters, filterWorkflows };
}
