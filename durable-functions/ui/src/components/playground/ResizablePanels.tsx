import { useCallback, useRef, useState, useEffect } from 'react';

interface ResizablePanelsProps {
  leftPanel: React.ReactNode;
  centerPanel: React.ReactNode;
  rightPanel: React.ReactNode;
  bottomPanel?: React.ReactNode;
  leftWidth: number;
  rightWidth: number;
  bottomHeight: number;
  onLeftWidthChange: (width: number) => void;
  onRightWidthChange: (width: number) => void;
  onBottomHeightChange: (height: number) => void;
  bottomExpanded: boolean;
  onBottomExpandedChange: (expanded: boolean) => void;
  minLeftWidth?: number;
  minRightWidth?: number;
  minCenterWidth?: number;
  minBottomHeight?: number;
}

export function ResizablePanels({
  leftPanel,
  centerPanel,
  rightPanel,
  bottomPanel,
  leftWidth,
  rightWidth,
  bottomHeight,
  onLeftWidthChange,
  onRightWidthChange,
  onBottomHeightChange,
  bottomExpanded,
  onBottomExpandedChange,
  minLeftWidth = 280,
  minRightWidth = 280,
  minCenterWidth = 400,
  minBottomHeight = 80,
}: ResizablePanelsProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = useState<'left' | 'right' | 'bottom' | null>(null);
  const [startX, setStartX] = useState(0);
  const [startY, setStartY] = useState(0);
  const [startWidth, setStartWidth] = useState(0);
  const [startHeight, setStartHeight] = useState(0);

  const handleMouseDown = useCallback(
    (e: React.MouseEvent, divider: 'left' | 'right' | 'bottom') => {
      e.preventDefault();
      setDragging(divider);
      setStartX(e.clientX);
      setStartY(e.clientY);
      if (divider === 'left') {
        setStartWidth(leftWidth);
      } else if (divider === 'right') {
        setStartWidth(rightWidth);
      } else {
        setStartHeight(bottomHeight);
      }
    },
    [leftWidth, rightWidth, bottomHeight]
  );

  useEffect(() => {
    if (!dragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (!containerRef.current) return;

      const containerRect = containerRef.current.getBoundingClientRect();

      if (dragging === 'left') {
        const deltaX = e.clientX - startX;
        const newWidth = Math.max(
          minLeftWidth,
          Math.min(startWidth + deltaX, containerRect.width - minCenterWidth - rightWidth - 16)
        );
        onLeftWidthChange(newWidth);
      } else if (dragging === 'right') {
        const deltaX = startX - e.clientX;
        const newWidth = Math.max(
          minRightWidth,
          Math.min(startWidth + deltaX, containerRect.width - minCenterWidth - leftWidth - 16)
        );
        onRightWidthChange(newWidth);
      } else if (dragging === 'bottom') {
        const deltaY = startY - e.clientY;
        const maxHeight = containerRect.height - 200; // Leave space for main content
        const newHeight = Math.max(minBottomHeight, Math.min(startHeight + deltaY, maxHeight));
        onBottomHeightChange(newHeight);
      }
    };

    const handleMouseUp = () => {
      setDragging(null);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [
    dragging,
    startX,
    startY,
    startWidth,
    startHeight,
    leftWidth,
    rightWidth,
    minLeftWidth,
    minRightWidth,
    minCenterWidth,
    minBottomHeight,
    onLeftWidthChange,
    onRightWidthChange,
    onBottomHeightChange,
  ]);

  return (
    <div
      ref={containerRef}
      className="flex flex-col h-full overflow-hidden"
      style={{ cursor: dragging ? (dragging === 'bottom' ? 'ns-resize' : 'ew-resize') : 'default' }}
    >
      {/* Main panels row */}
      <div className="flex flex-1 min-h-0">
        {/* Left Panel */}
        <div
          className="flex-shrink-0 overflow-hidden bg-dark-card border-r border-dark-border"
          style={{ width: leftWidth }}
        >
          {leftPanel}
        </div>

        {/* Left Divider */}
        <div
          className={`w-1 flex-shrink-0 cursor-ew-resize transition-colors hover:bg-blue-500 ${
            dragging === 'left' ? 'bg-blue-500' : 'bg-dark-border'
          }`}
          onMouseDown={(e) => handleMouseDown(e, 'left')}
        />

        {/* Center Panel */}
        <div className="flex-1 min-w-0 overflow-hidden bg-dark-bg">{centerPanel}</div>

        {/* Right Divider */}
        <div
          className={`w-1 flex-shrink-0 cursor-ew-resize transition-colors hover:bg-blue-500 ${
            dragging === 'right' ? 'bg-blue-500' : 'bg-dark-border'
          }`}
          onMouseDown={(e) => handleMouseDown(e, 'right')}
        />

        {/* Right Panel */}
        <div
          className="flex-shrink-0 overflow-hidden bg-dark-card border-l border-dark-border"
          style={{ width: rightWidth }}
        >
          {rightPanel}
        </div>
      </div>

      {/* Bottom Panel (Timeline) */}
      {bottomPanel && (
        <>
          {/* Bottom Divider with collapse toggle */}
          <div className="flex items-center bg-dark-card border-t border-dark-border">
            <div
              className={`flex-1 h-1 cursor-ns-resize transition-colors hover:bg-blue-500 ${
                dragging === 'bottom' ? 'bg-blue-500' : 'bg-dark-border'
              }`}
              onMouseDown={(e) => handleMouseDown(e, 'bottom')}
            />
            <button
              onClick={() => onBottomExpandedChange(!bottomExpanded)}
              className="px-3 py-1 text-xs text-gray-400 hover:text-white hover:bg-dark-hover transition-colors"
              title={bottomExpanded ? 'Collapse timeline' : 'Expand timeline'}
            >
              {bottomExpanded ? '▼ Timeline' : '▲ Timeline'}
            </button>
            <div
              className={`flex-1 h-1 cursor-ns-resize transition-colors hover:bg-blue-500 ${
                dragging === 'bottom' ? 'bg-blue-500' : 'bg-dark-border'
              }`}
              onMouseDown={(e) => handleMouseDown(e, 'bottom')}
            />
          </div>

          {/* Bottom Panel Content */}
          <div
            className="flex-shrink-0 overflow-hidden bg-dark-card transition-all duration-200"
            style={{ height: bottomExpanded ? bottomHeight : 0 }}
          >
            {bottomPanel}
          </div>
        </>
      )}
    </div>
  );
}
