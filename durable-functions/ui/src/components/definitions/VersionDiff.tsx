import { useState, useMemo } from 'react';
import { useDefinitionDetail, useDefinitionVersions } from '../../hooks/useDefinitions';
import styles from './VersionDiff.module.css';

interface VersionDiffProps {
  definitionId: string;
  currentVersion: string;
}

interface DiffLine {
  type: 'unchanged' | 'added' | 'removed';
  lineNumber: { left?: number; right?: number };
  content: string;
}

export function VersionDiff({ definitionId, currentVersion }: VersionDiffProps) {
  const [leftVersion, setLeftVersion] = useState<string>('');
  const [rightVersion, setRightVersion] = useState<string>(currentVersion);
  const [isExpanded, setIsExpanded] = useState(false);

  const { data: versionsData } = useDefinitionVersions(definitionId);
  const { data: leftData, isLoading: leftLoading } = useDefinitionDetail(
    definitionId,
    leftVersion || undefined
  );
  const { data: rightData, isLoading: rightLoading } = useDefinitionDetail(
    definitionId,
    rightVersion || undefined
  );

  const versions = versionsData?.versions || [];

  // Simple diff algorithm for JSON comparison
  const diffResult = useMemo(() => {
    if (!leftData || !rightData) return [];

    const leftJson = JSON.stringify(leftData, null, 2).split('\n');
    const rightJson = JSON.stringify(rightData, null, 2).split('\n');

    return computeDiff(leftJson, rightJson);
  }, [leftData, rightData]);

  const stats = useMemo(() => {
    const added = diffResult.filter(d => d.type === 'added').length;
    const removed = diffResult.filter(d => d.type === 'removed').length;
    const unchanged = diffResult.filter(d => d.type === 'unchanged').length;
    return { added, removed, unchanged };
  }, [diffResult]);

  if (versions.length < 2) {
    return null; // No point in showing diff with only one version
  }

  return (
    <div className={styles.container}>
      <button
        className={styles.toggleButton}
        onClick={() => setIsExpanded(!isExpanded)}
        aria-expanded={isExpanded}
        aria-controls="version-diff-panel"
      >
        <span className={styles.toggleIcon}>{isExpanded ? '▼' : '▶'}</span>
        <span>Compare Versions</span>
        {isExpanded && stats.added + stats.removed > 0 && (
          <span className={styles.statsInline}>
            <span className={styles.addedStat}>+{stats.added}</span>
            <span className={styles.removedStat}>-{stats.removed}</span>
          </span>
        )}
      </button>

      {isExpanded && (
        <div id="version-diff-panel" className={styles.panel}>
          <div className={styles.versionSelectors}>
            <div className={styles.selector}>
              <label htmlFor="left-version">Base version:</label>
              <select
                id="left-version"
                value={leftVersion}
                onChange={(e) => setLeftVersion(e.target.value)}
                aria-label="Select base version to compare"
              >
                <option value="">Select version...</option>
                {versions.map((v) => (
                  <option key={v} value={v}>
                    v{v} {v === currentVersion ? '(current)' : ''}
                  </option>
                ))}
              </select>
            </div>
            <span className={styles.arrow}>→</span>
            <div className={styles.selector}>
              <label htmlFor="right-version">Compare to:</label>
              <select
                id="right-version"
                value={rightVersion}
                onChange={(e) => setRightVersion(e.target.value)}
                aria-label="Select version to compare against"
              >
                {versions.map((v) => (
                  <option key={v} value={v}>
                    v{v} {v === currentVersion ? '(current)' : ''}
                  </option>
                ))}
              </select>
            </div>
          </div>

          {!leftVersion && (
            <div className={styles.hint}>
              Select a base version to compare against {rightVersion}
            </div>
          )}

          {leftVersion && (leftLoading || rightLoading) && (
            <div className={styles.loading}>
              <div className={styles.spinner} aria-label="Loading versions" />
              Loading versions...
            </div>
          )}

          {leftVersion && !leftLoading && !rightLoading && diffResult.length > 0 && (
            <>
              <div className={styles.stats}>
                <span className={styles.statItem}>
                  <span className={styles.addedBadge}>{stats.added} additions</span>
                </span>
                <span className={styles.statItem}>
                  <span className={styles.removedBadge}>{stats.removed} deletions</span>
                </span>
                <span className={styles.statItem}>
                  {stats.unchanged} unchanged
                </span>
              </div>

              <div className={styles.diffContainer} role="region" aria-label="Version differences">
                <div className={styles.diffHeader}>
                  <span className={styles.diffHeaderLeft}>v{leftVersion}</span>
                  <span className={styles.diffHeaderRight}>v{rightVersion}</span>
                </div>
                <div className={styles.diffBody}>
                  {diffResult.map((line, index) => (
                    <div
                      key={index}
                      className={`${styles.diffLine} ${styles[line.type]}`}
                    >
                      <span className={styles.lineNumber}>
                        {line.lineNumber.left || ''}
                      </span>
                      <span className={styles.lineNumber}>
                        {line.lineNumber.right || ''}
                      </span>
                      <span className={styles.diffIndicator}>
                        {line.type === 'added' ? '+' : line.type === 'removed' ? '-' : ' '}
                      </span>
                      <pre className={styles.lineContent}>{line.content}</pre>
                    </div>
                  ))}
                </div>
              </div>
            </>
          )}

          {leftVersion && !leftLoading && !rightLoading && stats.added === 0 && stats.removed === 0 && (
            <div className={styles.noChanges}>
              No differences found between v{leftVersion} and v{rightVersion}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// Simple line-by-line diff algorithm
function computeDiff(leftLines: string[], rightLines: string[]): DiffLine[] {
  const result: DiffLine[] = [];

  let leftIndex = 0;
  let rightIndex = 0;
  let leftLineNum = 1;
  let rightLineNum = 1;

  // Use LCS-based approach for better diffs
  const lcs = longestCommonSubsequence(leftLines, rightLines);
  let lcsIndex = 0;

  while (leftIndex < leftLines.length || rightIndex < rightLines.length) {
    const leftLine = leftLines[leftIndex];
    const rightLine = rightLines[rightIndex];
    const lcsLine = lcs[lcsIndex];

    if (leftIndex < leftLines.length && leftLine === lcsLine && rightIndex < rightLines.length && rightLine === lcsLine) {
      // Line is in both - unchanged
      result.push({
        type: 'unchanged',
        lineNumber: { left: leftLineNum++, right: rightLineNum++ },
        content: leftLine,
      });
      leftIndex++;
      rightIndex++;
      lcsIndex++;
    } else if (leftIndex < leftLines.length && leftLine !== lcsLine) {
      // Line removed from left
      result.push({
        type: 'removed',
        lineNumber: { left: leftLineNum++ },
        content: leftLine,
      });
      leftIndex++;
    } else if (rightIndex < rightLines.length && rightLine !== lcsLine) {
      // Line added to right
      result.push({
        type: 'added',
        lineNumber: { right: rightLineNum++ },
        content: rightLine,
      });
      rightIndex++;
    } else if (leftIndex < leftLines.length) {
      result.push({
        type: 'removed',
        lineNumber: { left: leftLineNum++ },
        content: leftLine,
      });
      leftIndex++;
    } else if (rightIndex < rightLines.length) {
      result.push({
        type: 'added',
        lineNumber: { right: rightLineNum++ },
        content: rightLine,
      });
      rightIndex++;
    }
  }

  return result;
}

// LCS algorithm for better diff quality
function longestCommonSubsequence(a: string[], b: string[]): string[] {
  const m = a.length;
  const n = b.length;
  const dp: number[][] = Array(m + 1).fill(null).map(() => Array(n + 1).fill(0));

  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (a[i - 1] === b[j - 1]) {
        dp[i][j] = dp[i - 1][j - 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
      }
    }
  }

  // Backtrack to find LCS
  const result: string[] = [];
  let i = m, j = n;
  while (i > 0 && j > 0) {
    if (a[i - 1] === b[j - 1]) {
      result.unshift(a[i - 1]);
      i--;
      j--;
    } else if (dp[i - 1][j] > dp[i][j - 1]) {
      i--;
    } else {
      j--;
    }
  }

  return result;
}
