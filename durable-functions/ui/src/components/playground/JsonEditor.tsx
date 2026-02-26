import { useCallback, useState, useEffect } from 'react';

interface JsonEditorProps {
  value: string;
  onChange: (value: string) => void;
  readOnly?: boolean;
  placeholder?: string;
}

export function JsonEditor({ value, onChange, readOnly = false, placeholder }: JsonEditorProps) {
  const [localValue, setLocalValue] = useState(value);
  const [error, setError] = useState<string | null>(null);

  // Sync external value changes
  useEffect(() => {
    setLocalValue(value);
  }, [value]);

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      const newValue = e.target.value;
      setLocalValue(newValue);

      // Validate JSON
      if (newValue.trim()) {
        try {
          JSON.parse(newValue);
          setError(null);
          onChange(newValue);
        } catch (err) {
          setError((err as Error).message);
        }
      } else {
        setError(null);
        onChange(newValue);
      }
    },
    [onChange]
  );

  const handleFormat = useCallback(() => {
    try {
      const parsed = JSON.parse(localValue);
      const formatted = JSON.stringify(parsed, null, 2);
      setLocalValue(formatted);
      setError(null);
      onChange(formatted);
    } catch (err) {
      setError((err as Error).message);
    }
  }, [localValue, onChange]);

  const handleMinify = useCallback(() => {
    try {
      const parsed = JSON.parse(localValue);
      const minified = JSON.stringify(parsed);
      setLocalValue(minified);
      setError(null);
      onChange(minified);
    } catch (err) {
      setError((err as Error).message);
    }
  }, [localValue, onChange]);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(localValue);
  }, [localValue]);

  return (
    <div className="h-full flex flex-col">
      {/* Toolbar */}
      <div className="flex items-center justify-between p-2 bg-dark-card border-b border-dark-border">
        <div className="flex items-center gap-2">
          {!readOnly && (
            <>
              <button
                onClick={handleFormat}
                className="px-2 py-1 text-xs bg-dark-bg border border-dark-border rounded hover:bg-dark-hover text-gray-300"
                title="Format JSON"
              >
                Format
              </button>
              <button
                onClick={handleMinify}
                className="px-2 py-1 text-xs bg-dark-bg border border-dark-border rounded hover:bg-dark-hover text-gray-300"
                title="Minify JSON"
              >
                Minify
              </button>
            </>
          )}
          <button
            onClick={handleCopy}
            className="px-2 py-1 text-xs bg-dark-bg border border-dark-border rounded hover:bg-dark-hover text-gray-300"
            title="Copy to clipboard"
          >
            Copy
          </button>
        </div>
        {error && (
          <span className="text-xs text-red-400 truncate max-w-[200px]" title={error}>
            ⚠️ {error}
          </span>
        )}
      </div>

      {/* Editor */}
      <div className="flex-1 relative overflow-hidden">
        <textarea
          value={localValue}
          onChange={handleChange}
          readOnly={readOnly}
          placeholder={placeholder}
          className={`w-full h-full p-4 bg-dark-bg text-gray-200 font-mono text-sm resize-none focus:outline-none focus:ring-1 ${
            error ? 'focus:ring-red-500' : 'focus:ring-blue-500'
          } ${readOnly ? 'cursor-default opacity-80' : ''}`}
          spellCheck={false}
        />

        {/* Line numbers overlay (simplified) */}
        <div className="absolute top-0 left-0 w-10 h-full bg-dark-card border-r border-dark-border pointer-events-none overflow-hidden">
          <div className="p-4 font-mono text-xs text-gray-500 select-none">
            {localValue.split('\n').map((_, i) => (
              <div key={i} className="leading-5">
                {i + 1}
              </div>
            ))}
          </div>
        </div>

        {/* Adjust textarea padding for line numbers */}
        <style>{`
          textarea {
            padding-left: 48px !important;
          }
        `}</style>
      </div>
    </div>
  );
}
