import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { BackupInfo } from '../types';

interface Props {
  onClose: () => void;
  onRestored: () => void;
}

export default function BackupRestoreDialog({ onClose, onRestored }: Props) {
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [restoring, setRestoring] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmRestore, setConfirmRestore] = useState<string | null>(null);

  const loadBackups = () => {
    setLoading(true);
    setError(null);
    invoke<BackupInfo[]>('list_backups')
      .then(setBackups)
      .catch(e => setError(`${e}`))
      .finally(() => setLoading(false));
  };

  useEffect(loadBackups, []);

  const handleBackup = async () => {
    try {
      await invoke<BackupInfo>('backup_config');
      loadBackups();
    } catch (e) {
      setError(`${e}`);
    }
  };

  const handleRestore = (filename: string) => {
    setConfirmRestore(filename);
  };

  const confirmAndRestore = async () => {
    if (!confirmRestore) return;
    setRestoring(confirmRestore);
    setError(null);
    setConfirmRestore(null);
    try {
      await invoke('restore_config', { backupFilename: confirmRestore });
      onRestored();
      onClose();
    } catch (e) {
      setError(`${e}`);
      setRestoring(null);
    }
  };

  const formatDisplayTime = (ts: string) => {
    // ts is YYYYMMDD-HHMMSS
    if (ts.length < 15) return ts;
    const d = ts.substring(0, 4) + '-' + ts.substring(4, 6) + '-' + ts.substring(6, 8);
    const t = ts.substring(9, 11) + ':' + ts.substring(11, 13) + ':' + ts.substring(13, 15);
    return d + ' ' + t;
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={e => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="Backup & Restore">
        <div className="dialog-header">
          <h2 className="dialog-title">Backup & Restore</h2>
          <button className="btn btn-icon" onClick={onClose} aria-label="Close">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <div className="dialog-body">
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 'var(--space-4)' }}>
            <span style={{ fontSize: 'var(--text-sm)', color: 'var(--text-secondary)' }}>
              Save or restore your full mod setup.
            </span>
            <button className="btn btn-primary btn-sm" onClick={handleBackup}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14">
                <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
                <polyline points="17 21 17 13 7 13 7 21" />
                <polyline points="7 3 7 8 15 8" />
              </svg>
              Backup Now
            </button>
          </div>

          {error && (
            <div className="safety-note" role="alert" style={{ marginBottom: 'var(--space-3)' }}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="10" />
                <line x1="15" y1="9" x2="9" y2="15" />
                <line x1="9" y1="9" x2="15" y2="15" />
              </svg>
              <span>{error}</span>
            </div>
          )}

          {loading ? (
            <div style={{ textAlign: 'center', padding: 'var(--space-5)', color: 'var(--text-muted)' }}>Loading backups...</div>
          ) : backups.length === 0 ? (
            <div style={{ textAlign: 'center', padding: 'var(--space-5)', color: 'var(--text-muted)', fontSize: 'var(--text-sm)' }}>
              No backups yet. Create one with the button above.
            </div>
          ) : (
            <div style={{ maxHeight: '300px', overflowY: 'auto' }}>
              {backups.map(b => (
                <div
                  key={b.filename}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    padding: 'var(--space-2) var(--space-3)',
                    marginBottom: 'var(--space-1)',
                    background: 'var(--void)',
                    borderRadius: 'var(--radius-md)',
                    border: '1px solid var(--border-subtle)',
                  }}
                >
                  <div>
                    <div style={{ fontSize: 'var(--text-sm)', fontFamily: 'var(--font-mono)', color: 'var(--text-primary)' }}>
                      {formatDisplayTime(b.timestamp)}
                    </div>
                    <div style={{ fontSize: 'var(--text-xs)', color: 'var(--text-muted)', marginTop: '2px' }}>
                      {b.game_count} game{b.game_count !== 1 ? 's' : ''} · {b.mod_count} mod{b.mod_count !== 1 ? 's' : ''}
                    </div>
                  </div>
                  <button
                    className="btn btn-ghost btn-sm"
                    onClick={() => handleRestore(b.filename)}
                    disabled={restoring !== null}
                  >
                    {restoring === b.filename ? 'Restoring...' : 'Restore'}
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="dialog-footer">
          {confirmRestore ? (
            <>
              <div style={{ flex: 1, textAlign: 'left', fontSize: 'var(--text-xs)', color: 'var(--signal-amber)' }}>
                ⚠ Restoring will replace your current config and redeploy all mods.
              </div>
              <button className="btn btn-secondary btn-sm" onClick={() => setConfirmRestore(null)}>Cancel</button>
              <button className="btn btn-destructive btn-sm" onClick={confirmAndRestore}>Confirm Restore</button>
            </>
          ) : (
            <button className="btn btn-secondary" onClick={onClose}>Close</button>
          )}
        </div>
      </div>
    </div>
  );
}
