import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { SaveFile } from '../types';

interface Props {
  gameId: string;
  gameName: string;
}

export default function SaveScanner({ gameId, gameName }: Props) {
  const [saves, setSaves] = useState<SaveFile[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState(false);

  const scan = () => {
    setLoading(true);
    setError(null);
    invoke<SaveFile[]>('scan_saves', { gameId })
      .then(s => {
        setSaves(s);
        setExpanded(true);
      })
      .catch(e => {
        // Only show error if expanded; don't spam on first load
        setError(`${e}`);
      })
      .finally(() => setLoading(false));
  };

  // Auto-scan on mount
  useEffect(() => { scan(); }, [gameId]);

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const formatTime = (ts: string) => {
    if (ts.length < 15) return ts;
    return ts.substring(0, 4) + '-' + ts.substring(4, 6) + '-' + ts.substring(6, 8)
      + ' ' + ts.substring(9, 11) + ':' + ts.substring(11, 13);
  };

  const saveCount = saves.filter(s => s.isAutosave).length;
  const manualCount = saves.length - saveCount;

  return (
    <div style={{
      borderBottom: '1px solid var(--border-subtle)',
      padding: expanded ? 'var(--space-3) var(--space-5)' : 'var(--space-1) var(--space-5)',
    }}>
      <button
        onClick={() => {
          if (!expanded) scan();
          else setExpanded(false);
        }}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-2)',
          width: '100%',
          padding: 'var(--space-1) 0',
          cursor: 'pointer',
          background: 'none',
          border: 'none',
          color: 'var(--text-secondary)',
          font: 'inherit',
          fontSize: 'var(--text-xs)',
        }}
      >
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          width="14"
          height="14"
          style={{ flexShrink: 0, transition: 'transform var(--motion-fast)', transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)' }}
        >
          <polyline points="9 18 15 12 9 6" />
        </svg>
        <span>
          {loading ? 'Scanning saves...' :
           error ? `Save scanner: ${error}` :
           saves.length === 0 ? `No saves found for ${gameName}` :
           `${saves.length} save${saves.length !== 1 ? 's' : ''}` + (saveCount > 0 ? ` (${manualCount} manual, ${saveCount} auto)` : '')}
        </span>
        {!loading && (
          <button
            onClick={e => { e.stopPropagation(); scan(); }}
            className="btn btn-ghost btn-sm"
            style={{ marginLeft: 'auto' }}
            title="Refresh"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="12" height="12">
              <polyline points="23 4 23 10 17 10" />
              <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
            </svg>
          </button>
        )}
      </button>

      {expanded && saves.length > 0 && (
        <div style={{ marginTop: 'var(--space-2)', maxHeight: '200px', overflowY: 'auto' }}>
          {saves.map(s => (
            <div
              key={s.path}
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                padding: 'var(--space-1) var(--space-2)',
                fontSize: 'var(--text-xs)',
                fontFamily: 'var(--font-mono)',
                borderBottom: '1px solid var(--border-subtle)',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)', overflow: 'hidden' }}>
                <span style={{
                  display: 'inline-block',
                  width: '6px',
                  height: '6px',
                  borderRadius: '50%',
                  background: s.isAutosave ? 'var(--signal-amber)' : 'var(--signal-green)',
                  flexShrink: 0,
                }} />
                <span style={{
                  color: 'var(--text-primary)',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                  maxWidth: '260px',
                }}>
                  {s.name}
                </span>
                {s.isQuicksave && (
                  <span style={{ color: 'var(--signal-blue)', fontSize: '10px', flexShrink: 0 }}>Quick</span>
                )}
                {s.isAutosave && (
                  <span style={{ color: 'var(--signal-amber)', fontSize: '10px', flexShrink: 0 }}>Auto</span>
                )}
              </div>
              <div style={{ display: 'flex', gap: 'var(--space-3)', color: 'var(--text-muted)', flexShrink: 0 }}>
                <span>{formatSize(s.sizeBytes)}</span>
                <span>{formatTime(s.modified)}</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
