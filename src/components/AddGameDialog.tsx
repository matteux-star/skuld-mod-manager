import { useState } from 'react';

interface Props {
  onPickDirectory: () => Promise<string | null>;
  onAdd: (name: string, path: string, gameType: string) => void;
  onClose: () => void;
}

export default function AddGameDialog({ onPickDirectory, onAdd, onClose }: Props) {
  const [name, setName] = useState('');
  const [path, setPath] = useState('');
  const [type, setType] = useState<'witcher3' | 'sod2'>('witcher3');
  const [error, setError] = useState('');
  const [picking, setPicking] = useState(false);

  const handlePick = async () => {
    setPicking(true);
    const dir = await onPickDirectory();
    setPicking(false);
    if (dir) {
      setPath(dir);
      if (!name.trim()) {
        const folderName = dir.split('/').filter(Boolean).pop() || '';
        if (folderName) setName(folderName);
      }
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    if (!name.trim()) { setError('Game name is required'); return; }
    if (!path.trim()) { setError('Game path is required'); return; }
    if (!path.startsWith('/')) { setError('Path must be absolute'); return; }
    onAdd(name.trim(), path.trim().replace(/\/+$/, ''), type);
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={e => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="Add game">
        <div className="dialog-header">
          <h2 className="dialog-title">Add Game</h2>
          <button className="btn btn-icon" onClick={onClose} aria-label="Close">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="dialog-body">
            <div className="input-group">
              <label className="input-label" htmlFor="game-type">Game</label>
              <select id="game-type" className="input" value={type} onChange={e => setType(e.target.value as 'witcher3' | 'sod2')} style={{ appearance: 'none', cursor: 'pointer' }}>
                <option value="witcher3">The Witcher 3 (load order, verified)</option>
                <option value="sod2">State of Decay 2 (Proton, provisional)</option>
              </select>
            </div>
            <div className="input-group">
              <label className="input-label" htmlFor="game-name">Display Name</label>
              <input id="game-name" className="input" type="text" value={name} onChange={e => setName(e.target.value)} placeholder="The Witcher 3" autoFocus />
            </div>
            <div className="input-group">
              <label className="input-label" htmlFor="game-path">Install Path</label>
              <div style={{ display: 'flex', gap: 'var(--space-2)' }}>
                <input id="game-path" className="input input-mono" type="text" value={path} onChange={e => setPath(e.target.value)} placeholder={type === 'sod2' ? '/home/user/.steam/steam/steamapps/common/StateOfDecay2' : '/home/user/.steam/steam/steamapps/common/The Witcher 3'} style={{ flex: 1 }} />
                <button type="button" className="btn btn-secondary btn-sm" onClick={handlePick} disabled={picking}>{picking ? '...' : 'Browse'}</button>
              </div>
              <div className="input-hint">
                {type === 'sod2'
                  ? 'Steam install directory (under steamapps/common). Mods deploy into the Proton prefix — launch the game once through Steam first.'
                  : "Absolute path to the game's install directory."}
              </div>
            </div>
            {error && <div className="input-error" role="alert">{error}</div>}
          </div>
          <div className="dialog-footer">
            <button type="button" className="btn btn-secondary" onClick={onClose}>Cancel</button>
            <button type="submit" className="btn btn-primary">Add Game</button>
          </div>
        </form>
      </div>
    </div>
  );
}
