import { useState, useEffect } from 'react';
import { useShortcutsHelp } from '../../hooks/useKeyboardShortcuts';
import styles from './KeyboardShortcutsHelp.module.css';

export function KeyboardShortcutsHelp() {
  const [isOpen, setIsOpen] = useState(false);
  const shortcuts = useShortcutsHelp();

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === '?' && !e.ctrlKey && !e.altKey && !e.metaKey) {
        // Check if not in input
        if (
          !(e.target instanceof HTMLInputElement) &&
          !(e.target instanceof HTMLTextAreaElement)
        ) {
          e.preventDefault();
          setIsOpen(prev => !prev);
        }
      }
      if (e.key === 'Escape' && isOpen) {
        setIsOpen(false);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen]);

  if (!isOpen) return null;

  return (
    <div className={styles.overlay} onClick={() => setIsOpen(false)}>
      <div className={styles.modal} onClick={e => e.stopPropagation()} role="dialog" aria-modal="true" aria-labelledby="shortcuts-title">
        <div className={styles.header}>
          <h2 id="shortcuts-title">Keyboard Shortcuts</h2>
          <button
            onClick={() => setIsOpen(false)}
            className={styles.closeButton}
            aria-label="Close keyboard shortcuts"
          >
            ✕
          </button>
        </div>
        <div className={styles.content}>
          <table className={styles.table}>
            <thead>
              <tr>
                <th>Keys</th>
                <th>Action</th>
              </tr>
            </thead>
            <tbody>
              {shortcuts.map((shortcut, index) => (
                <tr key={index}>
                  <td>
                    <kbd className={styles.kbd}>{shortcut.keys}</kbd>
                  </td>
                  <td>{shortcut.description}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className={styles.footer}>
          Press <kbd className={styles.kbd}>?</kbd> to toggle this dialog
        </div>
      </div>
    </div>
  );
}
