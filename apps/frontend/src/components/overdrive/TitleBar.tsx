import { useCallback, useRef } from 'react';

async function getWindow() {
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    return getCurrentWindow();
  } catch {
    return null;
  }
}

export function TitleBar() {
  const winRef = useRef<Awaited<ReturnType<typeof getWindow>>>(null);
  const dragging = useRef(false);

  const getWin = useCallback(async () => {
    if (!winRef.current) {
      winRef.current = await getWindow();
    }
    return winRef.current;
  }, []);

  const minimize = async (e: React.MouseEvent) => {
    e.stopPropagation();
    (await getWin())?.minimize();
  };
  const toggleMaximize = async (e: React.MouseEvent) => {
    e.stopPropagation();
    (await getWin())?.toggleMaximize();
  };
  const close = async (e: React.MouseEvent) => {
    e.stopPropagation();
    (await getWin())?.close();
  };

  const handleMouseDown = useCallback(async (e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target.closest('.ac-titlebar__controls')) return;
    if (dragging.current) return;
    dragging.current = true;
    try {
      const win = await getWin();
      if (win) {
        win.startDragging();
      }
    } catch {
      // fallback: let native drag-region handle it
    } finally {
      dragging.current = false;
    }
  }, [getWin]);

  return (
    <div className="ac-titlebar" data-tauri-drag-region onMouseDown={handleMouseDown}>
      <svg className="ac-titlebar__logo" viewBox="0 0 64 64" fill="none" aria-hidden="true">
        <path d="M32 4L56 18V46L32 60L8 46V18L32 4Z" fill="#161620" stroke="#3a3a4a" strokeWidth="2" />
        <path d="M32 15L45 22.5V37.5L32 45L19 37.5V22.5L32 15Z" fill="#1e1e2c" stroke="#4a4a5e" strokeWidth="1.5" />
        <path d="M32 23L39 27V35L32 39L25 35V27L32 23Z" fill="none" stroke="#7aa8c8" strokeWidth="1.2" />
      </svg>
      <span className="ac-titlebar__text">Overdrive</span>

      <div className="ac-titlebar__controls">
        <button className="ac-titlebar__btn" onClick={minimize} title="Minimize" aria-label="Minimize">
          <svg width="11" height="11" viewBox="0 0 11 11" fill="none">
            <rect x="0" y="5" width="11" height="1" fill="currentColor" />
          </svg>
        </button>
        <button className="ac-titlebar__btn" onClick={toggleMaximize} title="Maximize" aria-label="Maximize">
          <svg width="11" height="11" viewBox="0 0 11 11" fill="none">
            <rect x="0.5" y="0.5" width="10" height="10" stroke="currentColor" fill="none" />
          </svg>
        </button>
        <button className="ac-titlebar__btn ac-titlebar__btn--close" onClick={close} title="Close" aria-label="Close">
          <svg width="11" height="11" viewBox="0 0 11 11" fill="none">
            <path d="M0.5 0.5L10.5 10.5M10.5 0.5L0.5 10.5" stroke="currentColor" strokeWidth="1.1" />
          </svg>
        </button>
      </div>
    </div>
  );
}
