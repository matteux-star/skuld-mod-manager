import { useState } from 'react';
import type { ModEntry } from '../types';

interface Props {
  existingMods: ModEntry[];
  onPickArchive: () => Promise<string | null>;
  onImport: (archivePath: string, modName: string) => void;
  onClose: () => void;
  sevenZAvailable: boolean;
}

export default function ImportModDialog({ existingMods, onPickArchive, onImport, onClose, sevenZAvailable }: Props) {
  const [name, setName] = useState('');
  const [archive, setArchive] = useState('');
  const [error, setError] = useState('');
  const [picking, setPicking] = useState(false);

  const handlePick = async () => {
    setPicking(true);
    const file = await onPickArchive();
    setPicking(false);
    if (file) {
      setArchive(file);
      if (!name.trim()) {
        const fileName = file.split('/').pop()?.replace(/\.(zip|7z|rar)$/i, '')?.replace(/[_-]/g, ' ') || '';
        if (fileName) setName(fileName);
      }
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    if (!name.trim()) { setError('Mod name is required'); return; }
    if (!archive.trim()) { setError('Archive file is required'); return; }
    if (!archive.startsWith('/')) { setError('Please select a file using Browse'); return; }
    onImport(archive.trim(), name.trim());
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={e => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="Import mod">
        <div className="dialog-header">
          <h2 className="dialog-title">Import Mod</h2>
          <button className="btn btn-icon" onClick={onClose} aria-label="Close">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="dialog-body">
            {!sevenZAvailable && (
              <div className="safety-note" style={{ margin: '0 0 var(--space-4)' }}>
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="16" height="16">
                  <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
                  <line x1="12" y1="9" x2="12" y2="13" /><line x1="12" y1="17" x2="12.01" y2="17" />
                </svg>
                <span>7z not found. Only .zip imports will work. Install p7zip-full for .7z/.rar support.</span>
              </div>
            )}

            <div className="input-group">
              <label className="input-label" htmlFor="mod-name">Mod Name</label>
              <input id="mod-name" className="input" type="text" value={name} onChange={e => setName(e.target.value)} placeholder="Better Textures" autoFocus />
            </div>
            <div className="input-group">
              <label className="input-label" htmlFor="mod-archive">Archive File</label>
              <div style={{ display: 'flex', gap: 'var(--space-2)' }}>
                <input id="mod-archive" className="input input-mono" type="text" value={archive} onChange={e => setArchive(e.target.value)} placeholder="Select a .zip, .7z, or .rar file" style={{ flex: 1 }} readOnly onClick={handlePick} />
                <button type="button" className="btn btn-secondary btn-sm" onClick={handlePick} disabled={picking}>{picking ? '...' : 'Browse'}</button>
              </div>
              <div className="input-hint">Extracted mod files go to ~/.config/linux-mod-manager/library/</div>
            </div>
            {existingMods.length > 0 && (
              <div className="input-hint" style={{ color: 'var(--text-secondary)' }}>
                {existingMods.length} mod(s) already imported. Duplicate names trigger an update prompt.
              </div>
            )}
            {error && <div className="input-error" role="alert">{error}</div>}
          </div>

          <div className="dialog-footer">
            <button type="button" className="btn btn-secondary" onClick={onClose}>Cancel</button>
            <button type="submit" className="btn btn-primary">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="16" height="16">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="17 8 12 3 7 8" /><line x1="12" y1="3" x2="12" y2="15" />
              </svg>
              Import
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
