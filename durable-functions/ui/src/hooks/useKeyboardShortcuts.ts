import { useEffect, useCallback } from 'react';
import { useNavigate } from '@tanstack/react-router';

interface ShortcutConfig {
  key: string;
  ctrl?: boolean;
  alt?: boolean;
  shift?: boolean;
  action: () => void;
  description: string;
}

const defaultShortcuts: ShortcutConfig[] = [];

export function useKeyboardShortcuts(customShortcuts: ShortcutConfig[] = []) {
  const navigate = useNavigate();

  const navigationShortcuts: ShortcutConfig[] = [
    { key: 'h', alt: true, action: () => navigate({ to: '/' }), description: 'Go to Dashboard' },
    { key: 'w', alt: true, action: () => navigate({ to: '/workflows' }), description: 'Go to Workflows' },
    { key: 'd', alt: true, action: () => navigate({ to: '/designer' }), description: 'Go to Designer' },
    { key: 'p', alt: true, action: () => navigate({ to: '/playground' }), description: 'Go to Playground' },
    { key: 'f', alt: true, action: () => navigate({ to: '/definitions' }), description: 'Go to Definitions' },
  ];

  const allShortcuts = [...defaultShortcuts, ...navigationShortcuts, ...customShortcuts];

  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    // Ignore if user is typing in an input
    if (
      event.target instanceof HTMLInputElement ||
      event.target instanceof HTMLTextAreaElement ||
      (event.target as HTMLElement)?.isContentEditable
    ) {
      return;
    }

    for (const shortcut of allShortcuts) {
      const keyMatch = event.key.toLowerCase() === shortcut.key.toLowerCase();
      const ctrlMatch = shortcut.ctrl ? event.ctrlKey || event.metaKey : !event.ctrlKey && !event.metaKey;
      const altMatch = shortcut.alt ? event.altKey : !event.altKey;
      const shiftMatch = shortcut.shift ? event.shiftKey : !event.shiftKey;

      if (keyMatch && ctrlMatch && altMatch && shiftMatch) {
        event.preventDefault();
        shortcut.action();
        return;
      }
    }
  }, [allShortcuts]);

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  return { shortcuts: allShortcuts };
}

// Hook to show keyboard shortcuts help
export function useShortcutsHelp() {
  const shortcuts = [
    { keys: 'Alt + H', description: 'Go to Dashboard' },
    { keys: 'Alt + W', description: 'Go to Workflows' },
    { keys: 'Alt + D', description: 'Go to Designer' },
    { keys: 'Alt + P', description: 'Go to Playground' },
    { keys: 'Alt + F', description: 'Go to Definitions' },
    { keys: '?', description: 'Show keyboard shortcuts' },
    { keys: 'Esc', description: 'Close modal/dialog' },
  ];

  return shortcuts;
}
