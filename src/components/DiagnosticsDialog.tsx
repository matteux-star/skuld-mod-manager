import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { DiagnosticReport, ImportResult } from '../types';

interface Props {
  gameId: string;
  gameName: string;
  onClose: () => void;
  onRecovered: () => void;
}

const severityColor = { info: 'var(--text-muted)', warning: 'var(--signal-amber)', error: 'var(--signal-red)' };

export default function DiagnosticsDialog({ gameId, gameName, onClose, onRecovered }: Props) {
  const [report, setReport] = useState<DiagnosticReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [recovering, setRecovering] = useState<string | null>(null);

  const runDiagnostics = () => {
    setLoading(true);
    setError(null);
    invoke<DiagnosticReport>('diagnose_game', { gameId })
      .then(setReport)
      .catch(e => setError(`${e}`))
      .finally(() => setLoading(false));
  };

  useEffect(runDiagnostics, [gameId]);

  const handleRecover = async (folderName: string) => {
    setRecovering(folderName);
    setError(null);
    try {
      await invoke<ImportResult>('adopt_orphaned_mod', { gameId, folderName });
      onRecovered();
      runDiagnostics();
    } catch (e) {
      setError(`${folderName}: ${e}`);
    } finally {
      setRecovering(null);
    }
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={e => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="Diagnostics">
        <div className="dialog-header">
          <h2 className="dialog-title">Diagnostics — {gameName}</h2>
          <button className="btn btn-icon" onClick={onClose} aria-label="Close">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <div className="dialog-body">
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
            <div style={{ textAlign: 'center', padding: 'var(--space-5)', color: 'var(--text-muted)' }}>Running diagnostics...</div>
          ) : (
            <>
              <div style={{ maxHeight: '260px', overflowY: 'auto', marginBottom: 'var(--space-4)' }}>
                {report?.findings.map((f, i) => (
                  <div key={i} style={{
                    display: 'flex', gap: 'var(--space-2)', padding: 'var(--space-2) var(--space-3)',
                    marginBottom: 'var(--space-1)', background: 'var(--void)', borderRadius: 'var(--radius-md)',
                    border: '1px solid var(--border-subtle)', fontSize: 'var(--text-sm)',
                  }}>
                    <span style={{ color: severityColor[f.severity], fontWeight: 600, textTransform: 'uppercase', fontSize: 'var(--text-xs)', flexShrink: 0 }}>
                      {f.severity}
                    </span>
                    <span style={{ color: 'var(--text-primary)' }}>{f.message}</span>
                  </div>
                ))}
              </div>

              {report && report.orphanedFolders.length > 0 && (
                <div>
                  <div style={{ fontSize: 'var(--text-xs)', color: 'var(--text-muted)', marginBottom: 'var(--space-2)' }}>
                    Orphaned library folders — extracted on disk but not registered as mods:
                  </div>
                  {report.orphanedFolders.map(folder => (
                    <div key={folder} style={{
                      display: 'flex', alignItems: 'center', justifyContent: 'space-between',
                      padding: 'var(--space-2) var(--space-3)', marginBottom: 'var(--space-1)',
                      background: 'var(--void)', borderRadius: 'var(--radius-md)', border: '1px solid var(--border-subtle)',
                    }}>
                      <span style={{ fontSize: 'var(--text-sm)', fontFamily: 'var(--font-mono)', color: 'var(--text-primary)' }}>{folder}</span>
                      <button className="btn btn-ghost btn-sm" onClick={() => handleRecover(folder)} disabled={recovering !== null}>
                        {recovering === folder ? 'Recovering...' : 'Recover'}
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </>
          )}
        </div>

        <div className="dialog-footer">
          <button className="btn btn-secondary" onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
}
