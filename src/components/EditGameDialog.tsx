import { useState } from 'react';

interface Props {
  currentPath: string;
  currentLaunchPath?: string;
  gameName: string;
  onPickDirectory: () => Promise<string | null>;
  onPickExecutable: () => Promise<string | null>;
  onSave: (newPath: string, launchPath: string) => void;
  onClose: () => void;
}

export default function EditGameDialog({ currentPath, currentLaunchPath, gameName, onPickDirectory, onPickExecutable, onSave, onClose }: Props) {
  const [path, setPath] = useState(currentPath);
  const [launchPath, setLaunchPath] = useState(currentLaunchPath ?? '');
  const [error, setError] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    if (!path.trim()) { setError('Game path is required'); return; }
    if (!path.startsWith('/')) { setError('Path must be absolute'); return; }
    if (launchPath.trim() && !launchPath.startsWith('/')) { setError('Launch path must be absolute'); return; }
    if (path.trim() === currentPath && launchPath.trim() === (currentLaunchPath ?? '')) { onClose(); return; }
    onSave(path.trim().replace(/\/+$/, ''), launchPath.trim());
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={e => e.stopPropagation()} role="dialog" aria-modal="true">
        <div className="dialog-header">
          <h2 className="dialog-title">Edit {gameName}</h2>
          <button className="btn btn-icon" onClick={onClose} aria-label="Close">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>
        <form onSubmit={handleSubmit}>
          <div className="dialog-body">
            <div className="input-group">
              <label className="input-label" htmlFor="edit-path">Install Path</label>
              <div style={{ display: 'flex', gap: 'var(--space-2)' }}>
                <input id="edit-path" className="input input-mono" type="text" value={path} onChange={e => setPath(e.target.value)} style={{ flex: 1 }} autoFocus />
                <button type="button" className="btn btn-secondary btn-sm" onClick={onPickDirectory}>Browse</button>
              </div>
              <div className="input-hint">Changing this path redeploys all enabled mods to the new location.</div>
            </div>
            <div className="input-group">
              <label className="input-label" htmlFor="edit-launch">Launch Executable</label>
              <div style={{ display: 'flex', gap: 'var(--space-2)' }}>
                <input id="edit-launch" className="input input-mono" type="text" value={launchPath} onChange={e => setLaunchPath(e.target.value)} placeholder="e.g. /usr/bin/steam steam://rungameid/292030" style={{ flex: 1 }} />
                <button type="button" className="btn btn-secondary btn-sm" onClick={onPickExecutable}>Browse</button>
              </div>
              <div className="input-hint">Optional. The command to launch the game.</div>
            </div>
            {error && <div className="input-error" role="alert">{error}</div>}
          </div>
          <div className="dialog-footer">
            <button type="button" className="btn btn-secondary" onClick={onClose}>Cancel</button>
            <button type="submit" className="btn btn-primary">Save</button>
          </div>
        </form>
      </div>
    </div>
  );
}
