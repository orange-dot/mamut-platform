import { useState, useRef, useCallback } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useToast } from '../../contexts/ToastContext';
import styles from './ExportImport.module.css';

interface ExportImportProps {
  definitions: Array<{ id: string; name: string; version: string }>;
  onImport?: (definition: unknown) => Promise<void>;
}

export function ExportImport({ definitions, onImport }: ExportImportProps) {
  const [isImportModalOpen, setIsImportModalOpen] = useState(false);
  const [importData, setImportData] = useState('');
  const [importError, setImportError] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const toast = useToast();
  const queryClient = useQueryClient();

  const importMutation = useMutation({
    mutationFn: async (data: unknown) => {
      if (onImport) {
        await onImport(data);
      }
      return data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['definitions'] });
      toast.success('Import successful', 'Workflow definition imported');
      setIsImportModalOpen(false);
      setImportData('');
    },
    onError: (error: Error) => {
      toast.error('Import failed', error.message);
    },
  });

  const handleExportAll = useCallback(() => {
    const exportData = JSON.stringify(definitions, null, 2);
    const blob = new Blob([exportData], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `workflow-definitions-${new Date().toISOString().split('T')[0]}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    toast.success('Export successful', `Downloaded ${definitions.length} definitions`);
  }, [definitions, toast]);

  const handleFileSelect = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (event) => {
      const content = event.target?.result as string;
      setImportData(content);
      setImportError(null);
      try {
        JSON.parse(content);
      } catch {
        setImportError('Invalid JSON file');
      }
    };
    reader.readAsText(file);
  }, []);

  const handleImport = useCallback(() => {
    try {
      const parsed = JSON.parse(importData);
      importMutation.mutate(parsed);
    } catch {
      setImportError('Invalid JSON format');
    }
  }, [importData, importMutation]);

  return (
    <>
      <div className={styles.container}>
        <button onClick={handleExportAll} className={styles.button} title="Export all definitions">
          📤 Export All
        </button>
        <button onClick={() => setIsImportModalOpen(true)} className={styles.button} title="Import definition">
          📥 Import
        </button>
      </div>

      {isImportModalOpen && (
        <div className={styles.overlay} onClick={() => setIsImportModalOpen(false)}>
          <div className={styles.modal} onClick={e => e.stopPropagation()} role="dialog" aria-modal="true">
            <div className={styles.modalHeader}>
              <h3>Import Workflow Definition</h3>
              <button onClick={() => setIsImportModalOpen(false)} className={styles.closeButton} aria-label="Close">
                ✕
              </button>
            </div>

            <div className={styles.modalContent}>
              <div className={styles.fileUpload}>
                <input
                  ref={fileInputRef}
                  type="file"
                  accept=".json"
                  onChange={handleFileSelect}
                  className={styles.fileInput}
                  id="import-file"
                />
                <label htmlFor="import-file" className={styles.fileLabel}>
                  <span className={styles.uploadIcon}>📁</span>
                  <span>Choose a JSON file or drag & drop</span>
                </label>
              </div>

              <div className={styles.divider}>
                <span>or paste JSON</span>
              </div>

              <textarea
                value={importData}
                onChange={e => {
                  setImportData(e.target.value);
                  setImportError(null);
                }}
                className={styles.textarea}
                placeholder='{"id": "MyWorkflow", "name": "My Workflow", ...}'
                rows={10}
              />

              {importError && (
                <div className={styles.error} role="alert">
                  {importError}
                </div>
              )}
            </div>

            <div className={styles.modalFooter}>
              <button onClick={() => setIsImportModalOpen(false)} className={styles.cancelButton}>
                Cancel
              </button>
              <button
                onClick={handleImport}
                className={styles.importButton}
                disabled={!importData || !!importError || importMutation.isPending}
              >
                {importMutation.isPending ? 'Importing...' : 'Import'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Export buttons for individual definitions */}
      <style>{`
        .export-single-btn {
          padding: 0.25rem 0.5rem;
          background: var(--bg-secondary);
          border: 1px solid var(--border-color);
          border-radius: 0.25rem;
          cursor: pointer;
          font-size: 0.75rem;
        }
        .export-single-btn:hover {
          background: var(--bg-hover);
        }
      `}</style>
    </>
  );
}

// Hook for export/import functionality
export function useExportImport() {
  const toast = useToast();

  const exportToJson = useCallback((data: unknown, filename: string) => {
    const json = JSON.stringify(data, null, 2);
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    toast.success('Export successful', `Downloaded ${filename}`);
  }, [toast]);

  const importFromFile = useCallback((): Promise<unknown> => {
    return new Promise((resolve, reject) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = '.json';
      input.onchange = (e) => {
        const file = (e.target as HTMLInputElement).files?.[0];
        if (!file) {
          reject(new Error('No file selected'));
          return;
        }
        const reader = new FileReader();
        reader.onload = (event) => {
          try {
            const content = event.target?.result as string;
            const parsed = JSON.parse(content);
            resolve(parsed);
          } catch {
            reject(new Error('Invalid JSON file'));
          }
        };
        reader.onerror = () => reject(new Error('Failed to read file'));
        reader.readAsText(file);
      };
      input.click();
    });
  }, []);

  return { exportToJson, importFromFile };
}
