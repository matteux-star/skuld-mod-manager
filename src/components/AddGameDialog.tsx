import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface DetectedGame {
  gameType: string;
  displayName: string;
  installPath: string;
  source: string;
  sourceDetail: string;
}

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
  const [scanning, setScanning] = useState(false);
  const [detected, setDetected] = useState<DetectedGame[]>([]);

  const handleScan = async () => {
    setScanning(true);
    try {
      const results = await invoke<DetectedGame[]>('scan_for_games');
      setDetected(results);
    } catch (e) { setError(String(e)); }
    setScanning(false);
  };

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

  const handleDetectedAdd = (d: DetectedGame) => {
    onAdd(d.displayName, d.installPath, d.gameType);
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
            {/* ── Auto-detection ── */}
            <div style={{ marginBottom: 'var(--space-4)' }}>
              <button type="button" className="btn btn-secondary btn-sm" onClick={handleScan} disabled={scanning} style={{ width: '100%' }}>
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14">
                  <circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
                </svg>
                {scanning ? 'Scanning Steam...' : 'Scan for Games'}
              </button>
              {detected.length > 0 && (
                <div className="detected-list">
                  {detected.map(d => (
                    <div key={d.gameType} className="detected-item" onClick={() => handleDetectedAdd(d)}>
                      <div className="detected-item-left">
                        <span className="detected-name">{d.displayName}</span>
                        <span className="detected-path">{d.installPath}</span>
                        <span className="detected-source">{d.sourceDetail}</span>
                      </div>
                      <button type="button" className="btn btn-primary btn-sm">Add</button>
                    </div>
                  ))}
                </div>
              )}
            </div>

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
