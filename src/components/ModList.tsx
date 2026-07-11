import type { GameEntry, ModEntry } from '../types';
import ModCard from './ModCard';
import SaveScanner from './SaveScanner';
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface Props {
  game: GameEntry;
  onToggleMod: (modId: string) => void;
  onImportMod: () => void;
  onEditGame: () => void;
  onDeleteGame: () => void;
  onDeleteMod: (modId: string) => void;
  onReorderMods: (mods: ModEntry[]) => void;
  onDeployAll: () => void;
  onPurgeAll: () => void;
  onLaunchGame: () => void;
  onBackupRestore: () => void;
}

export default function ModList({
  game,
  onToggleMod,
  onImportMod,
  onEditGame,
  onDeleteGame,
  onDeleteMod,
  onReorderMods,
  onDeployAll,
  onPurgeAll,
  onLaunchGame,
  onBackupRestore,
}: Props) {
  const hasPriority = game.type === 'witcher3';
  const sortedMods = [...game.mods].sort((a, b) => a.priority - b.priority);
  const enabledCount = game.mods.filter(m => m.enabled).length;

  const [conflictMap, setConflictMap] = useState<Record<string, { level: string; conflictsWith: string[] }>>({});

  useEffect(() => {
    invoke<[string, { level: string; conflictsWith: string[]; overlappingFiles: string[] }][]>('check_conflicts', { gameId: game.id })
      .then(results => {
        const map: Record<string, { level: string; conflictsWith: string[] }> = {};
        results.forEach(([id, info]) => {
          if (info.level !== 'none') map[id] = { level: info.level, conflictsWith: info.conflictsWith };
        });
        setConflictMap(map);
      }).catch(() => {});
  }, [game.id, game.mods]);

  const handleDragStart = (e: React.DragEvent, index: number) => {
    e.dataTransfer.setData('text/plain', String(index));
    (e.currentTarget as HTMLElement).classList.add('dragging');
  };
  const handleDragEnd = (e: React.DragEvent) => {
    (e.currentTarget as HTMLElement).classList.remove('dragging');
    // Remove drag-over from all mod cards
    document.querySelectorAll('.mod-card.drag-over').forEach(el => el.classList.remove('drag-over'));
  };
  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    // Add visual feedback on the drop target
    const card = (e.currentTarget as HTMLElement).closest('.mod-card');
    if (card && !card.classList.contains('dragging')) {
      card.classList.add('drag-over');
    }
  };
  const handleDragLeave = (e: React.DragEvent) => {
    (e.currentTarget as HTMLElement).closest('.mod-card')?.classList.remove('drag-over');
  };
  const handleDrop = (e: React.DragEvent, toIndex: number) => {
    e.preventDefault();
    (e.currentTarget as HTMLElement).closest('.mod-card')?.classList.remove('drag-over');
    const fromIndex = parseInt(e.dataTransfer.getData('text/plain'));
    if (fromIndex === toIndex || isNaN(fromIndex)) return;
    const reordered = [...sortedMods];
    const [moved] = reordered.splice(fromIndex, 1);
    reordered.splice(toIndex, 0, moved);
    onReorderMods(reordered);
  };

  return (
    <>
      <div className="panel-header">
        <div className="panel-header-left">
          <div>
            <div className="panel-title">{game.name}</div>
            <div className="panel-subtitle">{game.path}</div>
          </div>
        </div>
        <div className="panel-header-right">
          {game.launchPath && (
            <button className="btn btn-secondary btn-sm" onClick={onLaunchGame} title={game.launchPath}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14">
                <polygon points="5 3 19 12 5 21 5 3" fill="currentColor" />
              </svg>
              Launch
            </button>
          )}
          <button className="btn btn-ghost btn-sm" onClick={onEditGame}>Edit</button>
          <button className="btn btn-ghost btn-sm" onClick={onDeleteGame} style={{ color: 'var(--signal-red)' }}>Remove</button>
        </div>
      </div>

      <div className="toolbar">
        <span style={{ fontSize: 'var(--text-xs)', color: 'var(--text-muted)' }}>
          {game.mods.length} mod{game.mods.length !== 1 ? 's' : ''} · {enabledCount} enabled
        </span>
        <div className="toolbar-spacer" />
        <button className="btn btn-ghost btn-sm" onClick={onDeployAll} disabled={enabledCount === 0} title="Deploy all enabled mods">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14">
            <polyline points="4 17 10 11 4 5" /><line x1="12" y1="19" x2="20" y2="19" />
          </svg>
          Deploy All
        </button>
        <button className="btn btn-ghost btn-sm" onClick={onPurgeAll} title="Remove all symlinks">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14">
            <polyline points="3 6 5 6 21 6" /><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
          </svg>
          Purge All
        </button>
        <button className="btn btn-primary btn-sm" onClick={onImportMod}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="17 8 12 3 7 8" /><line x1="12" y1="3" x2="12" y2="15" />
          </svg>
          Import Mod
        </button>
        <button className="btn btn-ghost btn-sm" onClick={onBackupRestore} title="Backup & Restore" style={{ marginLeft: 'var(--space-1)' }}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14">
            <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
            <polyline points="17 21 17 13 7 13 7 21" />
            <polyline points="7 3 7 8 15 8" />
          </svg>
          Backups
        </button>
      </div>

      <SaveScanner gameId={game.id} gameName={game.name} />

      {game.mods.length > 0 && (
        <div className="safety-note" role="alert">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
            <line x1="12" y1="9" x2="12" y2="13" /><line x1="12" y1="17" x2="12.01" y2="17" />
          </svg>
          <span>Close the game before enabling, disabling, or reordering mods.</span>
        </div>
      )}

      <div className="mod-list">
        {sortedMods.map((mod, i) => (
          <ModCard key={mod.id} mod={mod} showPriority={hasPriority}
            onToggle={() => onToggleMod(mod.id)} onDelete={() => onDeleteMod(mod.id)}
            onDragStart={(e) => hasPriority && mod.enabled && handleDragStart(e, i)}
            onDragEnd={handleDragEnd} onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
            onDrop={(e) => handleDrop(e, i)} conflictInfo={conflictMap[mod.id] ?? null} />
        ))}
        {game.mods.length === 0 && (
          <div className="mod-list-empty">
            <div className="mod-list-empty-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                <polyline points="14 2 14 8 20 8" /><line x1="12" y1="18" x2="12" y2="12" /><line x1="9" y1="15" x2="15" y2="15" />
              </svg>
            </div>
            <div className="mod-list-empty-title">No mods yet</div>
            <div className="mod-list-empty-desc">Import a mod archive to get started.</div>
          </div>
        )}
      </div>
    </>
  );
}
