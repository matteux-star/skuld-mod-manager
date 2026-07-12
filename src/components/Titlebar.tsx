import { getCurrentWindow } from '@tauri-apps/api/window';

const win = getCurrentWindow();

export default function Titlebar() {
  return (
    <div className="titlebar" data-tauri-drag-region>
      <div className="titlebar-spacer" data-tauri-drag-region />
      <span className="titlebar-title" data-tauri-drag-region>Skuld Mod Manager</span>
      <div className="titlebar-controls">
        <button type="button" className="titlebar-btn" aria-label="Minimize" onClick={() => win.minimize()}>
          <svg width="12" height="12" viewBox="0 0 12 12"><rect x="1" y="5.5" width="10" height="1.2" fill="currentColor" /></svg>
        </button>
        <button type="button" className="titlebar-btn" aria-label="Maximize" onClick={() => win.toggleMaximize()}>
          <svg width="11" height="11" viewBox="0 0 11 11"><rect x="0.6" y="0.6" width="9.8" height="9.8" fill="none" stroke="currentColor" strokeWidth="1.1" /></svg>
        </button>
        <button type="button" className="titlebar-btn titlebar-btn-close" aria-label="Close" onClick={() => win.close()}>
          <svg width="12" height="12" viewBox="0 0 12 12"><path d="M1 1L11 11M11 1L1 11" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" /></svg>
        </button>
      </div>
    </div>
  );
}
