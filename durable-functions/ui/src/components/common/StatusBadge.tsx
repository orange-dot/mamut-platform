interface StatusBadgeProps {
  status: string;
  size?: 'sm' | 'md' | 'lg';
}

export function StatusBadge({ status, size = 'md' }: StatusBadgeProps) {
  const statusConfig: Record<string, { bg: string; text: string; dot: string }> = {
    Running: { bg: 'bg-blue-100', text: 'text-blue-800', dot: 'bg-blue-500' },
    Pending: { bg: 'bg-yellow-100', text: 'text-yellow-800', dot: 'bg-yellow-500' },
    Completed: { bg: 'bg-green-100', text: 'text-green-800', dot: 'bg-green-500' },
    Failed: { bg: 'bg-red-100', text: 'text-red-800', dot: 'bg-red-500' },
    Terminated: { bg: 'bg-gray-100', text: 'text-gray-800', dot: 'bg-gray-500' },
    Suspended: { bg: 'bg-orange-100', text: 'text-orange-800', dot: 'bg-orange-500' },
  };

  const config = statusConfig[status] || statusConfig.Pending;

  const sizeClasses = {
    sm: 'px-2 py-0.5 text-xs',
    md: 'px-2.5 py-1 text-sm',
    lg: 'px-3 py-1.5 text-base',
  };

  const dotSizes = {
    sm: 'w-1.5 h-1.5',
    md: 'w-2 h-2',
    lg: 'w-2.5 h-2.5',
  };

  return (
    <span
      className={`inline-flex items-center gap-1.5 font-medium rounded-full ${config.bg} ${config.text} ${sizeClasses[size]}`}
      role="status"
      aria-label={`Workflow status: ${status}`}
    >
      <span
        className={`${dotSizes[size]} rounded-full ${config.dot} ${status === 'Running' ? 'animate-pulse' : ''}`}
        aria-hidden="true"
      />
      {status}
    </span>
  );
}
