import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { SaveFile } from '../types';

interface Props {
  gameId: string;
  gameName: string;
  onScanned?: (saves: SaveFile[]) => void;
  onClose: () => void;
}

function formatTime(ts: string): string {
  if (ts.length < 15) return ts;
  return ts.substring(0, 4) + '-' + ts.substring(4, 6) + '-' + ts.substring(6, 8)
    + ' ' + ts.substring(9, 11) + ':' + ts.substring(11, 13);
}

export default function DetectSavesDialog({ gameId, gameName, onScanned, onClose }: Props) {
  const [scanning, setScanning] = useState(true);
  const [saves, setSaves] = useState<SaveFile[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setScanning(true);
    setError(null);
    invoke<SaveFile[]>('scan_saves', { gameId })
      .then(s => { setSaves(s); onScanned?.(s); })
      .catch(e => setError(String(e)))
      .finally(() => setScanning(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [gameId]);

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog dialog-sm" onClick={e => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="Detect Saves">
        <h2 className="dialog-title">Detect Saves</h2>

        {scanning ? (
          <div className="detect-saves-scanning">
            <div className="spinner">
              <svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="var(--color-primary)" strokeWidth="2" strokeLinecap="round"><path d="M12 3a9 9 0 1 0 9 9" /></svg>
            </div>
            <div className="settings-row-desc">Scanning save directories&hellip;</div>
          </div>
        ) : (
          <>
            <div className="dialog-body">
              {error ? `Couldn't scan saves: ${error}` : `${saves.length} save${saves.length !== 1 ? 's' : ''} found for ${gameName}.`}
            </div>
            {saves.length > 0 && (
              <div className="detect-saves-list">
                {saves.map(s => (
                  <div key={s.path} className="detect-saves-item">
                    <span className="detect-saves-name">{s.name}</span>
                    <span className="detect-saves-time">{formatTime(s.modified)}</span>
                  </div>
                ))}
              </div>
            )}
          </>
        )}

        <div className="dialog-footer">
          <button type="button" className="btn btn-primary" onClick={onClose}>Done</button>
        </div>
      </div>
    </div>
  );
}
