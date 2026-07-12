import type { GameEntry, ModEntry, SaveFile } from '../types';
import ModCard from './ModCard';
import DetectSavesDialog from './DetectSavesDialog';
import { useEffect, useState, useMemo, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';

type SortBy = 'priority' | 'name' | 'status' | 'source';
type SortDir = 'asc' | 'desc';

interface BatchToggleResult {
  succeeded: string[];
  failed: { modId: string; modName: string; reason: string; conflictingMods: string[] }[];
}

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
  onBatchToggleMod: (modIds: string[], enabled: boolean) => Promise<BatchToggleResult>;
  onBatchDeleteMod: (modIds: string[]) => Promise<void>;
  onReloadConfig: () => void;
  onOpenProfiles: () => void;
  onModInfo: (modId: string) => void;
}

function sortMods(mods: ModEntry[], by: SortBy, dir: SortDir): ModEntry[] {
  const sorted = [...mods];
  const cmp = (a: string, b: string) => a.localeCompare(b, undefined, { sensitivity: 'base' });
  const dirMul = dir === 'asc' ? 1 : -1;

  switch (by) {
    case 'priority':
      sorted.sort((a, b) => (a.priority - b.priority) * dirMul);
      break;
    case 'name':
      sorted.sort((a, b) => cmp(a.name, b.name) * dirMul);
      break;
    case 'status': {
      const statusOrd = (m: ModEntry) => (m.enabled ? 0 : 1);
      sorted.sort((a, b) => {
        const d = (statusOrd(a) - statusOrd(b)) * dirMul;
        return d !== 0 ? d : cmp(a.name, b.name);
      });
      break;
    }
    case 'source':
      sorted.sort((a, b) => cmp(a.archiveSource, b.archiveSource) * dirMul);
      break;
  }
  return sorted;
}

function filterMods(mods: ModEntry[], query: string, category: string, tags: Set<string>): ModEntry[] {
  let result = mods;

  if (category) {
    result = result.filter(m => (m.category || 'Other') === category);
  }

  if (tags.size > 0) {
    result = result.filter(m => {
      const modTags = m.tags ?? [];
      return Array.from(tags).every(t => modTags.includes(t));
    });
  }

  if (query.trim()) {
    const q = query.toLowerCase();
    result = result.filter(m =>
      m.name.toLowerCase().includes(q) ||
      m.archiveSource.toLowerCase().includes(q) ||
      m.installedFiles.some(f => f.toLowerCase().includes(q)) ||
      (m.author ?? '').toLowerCase().includes(q) ||
      (m.description ?? '').toLowerCase().includes(q)
    );
  }

  return result;
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
  onBatchToggleMod,
  onBatchDeleteMod,
  onReloadConfig,
  onOpenProfiles,
  onModInfo,
}: Props) {
  const hasPriority = game.type === 'witcher3';
  const enabledCount = game.mods.filter(m => m.enabled).length;

  const [profileMenuOpen, setProfileMenuOpen] = useState(false);
  const activeProfile = game.profiles?.find(p => p.id === game.activeProfileId);

  const [searchQuery, setSearchQuery] = useState('');
  const [sortBy, setSortBy] = useState<SortBy>(hasPriority ? 'priority' : 'name');
  const [sortDir, setSortDir] = useState<SortDir>('asc');
  const debounceRef = useRef<ReturnType<typeof setTimeout>>();

  // Selection state
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const lastClickedRef = useRef<number>(-1);

  // Category + tag filter state
  const [categoryFilter, setCategoryFilter] = useState<string>('');
  const [tagFilter, setTagFilter] = useState<Set<string>>(new Set());

  // Saves detection
  const [detectOpen, setDetectOpen] = useState(false);
  const [savesResult, setSavesResult] = useState<SaveFile[] | null>(null);

  // Reset search/sort/selection/filters when game changes
  useEffect(() => {
    setSearchQuery('');
    setSortBy(hasPriority ? 'priority' : 'name');
    setSortDir('asc');
    setSelectedIds(new Set());
    setCategoryFilter('');
    setTagFilter(new Set());
    setSavesResult(null);
    lastClickedRef.current = -1;
  }, [game.id]);

  // Compute categories from mods
  const categories = useMemo(() => {
    const cats = new Map<string, number>();
    (game.mods ?? []).forEach(m => {
      const c = m.category || 'Other';
      cats.set(c, (cats.get(c) ?? 0) + 1);
    });
    return Array.from(cats.entries()).sort(([a], [b]) => a.localeCompare(b));
  }, [game.mods]);

  // Compute all tags
  const allTags = useMemo(() => {
    const tags = new Set<string>();
    (game.mods ?? []).forEach(m => (m.tags ?? []).forEach(t => tags.add(t)));
    return Array.from(tags).sort();
  }, [game.mods]);

  // Debounced search input
  const handleSearchInput = useCallback((value: string) => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => setSearchQuery(value), 150);
  }, []);

  // Filter and sort
  const visibleMods = useMemo(() => {
    const filtered = filterMods(game.mods, searchQuery, categoryFilter, tagFilter);
    return sortMods(filtered, sortBy, sortDir);
  }, [game.mods, searchQuery, sortBy, sortDir, categoryFilter, tagFilter]);

  const noResults = searchQuery.trim() !== '' && visibleMods.length === 0;

  // Refs for drag-and-drop indices
  const sortedForDrag = useMemo(
    () => sortMods(game.mods, 'priority', 'asc'),
    [game.mods]
  );

  // ── Selection handlers ──
  const handleSelect = useCallback((modId: string, event: React.MouseEvent) => {
    const idx = visibleMods.findIndex(m => m.id === modId);
    setSelectedIds(prev => {
      const next = new Set(prev);
      if (event.shiftKey && lastClickedRef.current >= 0) {
        // Range select
        const [lo, hi] = [Math.min(lastClickedRef.current, idx), Math.max(lastClickedRef.current, idx)];
        const selecting = !prev.has(modId);
        for (let i = lo; i <= hi; i++) {
          const mid = visibleMods[i]?.id;
          if (mid) selecting ? next.add(mid) : next.delete(mid);
        }
      } else if (event.ctrlKey || event.metaKey) {
        // Toggle single
        next.has(modId) ? next.delete(modId) : next.add(modId);
      } else {
        // Replace selection
        next.clear();
        next.has(modId) ? next.delete(modId) : next.add(modId);
      }
      return next;
    });
    lastClickedRef.current = idx;
  }, [visibleMods]);

  const handleSelectAll = useCallback(() => {
    setSelectedIds(prev => {
      if (prev.size === visibleMods.length) {
        return new Set();
      }
      return new Set(visibleMods.map(m => m.id));
    });
  }, [visibleMods]);

  const allSelected = visibleMods.length > 0 && selectedIds.size === visibleMods.length;

  // ── Batch operations ──
  const handleBatchEnable = useCallback(async () => {
    const ids = Array.from(selectedIds);
    if (ids.length === 0) return;
    const result = await onBatchToggleMod(ids, true);
    setSelectedIds(new Set());
    onReloadConfig();
    return result;
  }, [selectedIds, onBatchToggleMod, onReloadConfig]);

  const handleBatchDisable = useCallback(async () => {
    const ids = Array.from(selectedIds);
    if (ids.length === 0) return;
    const result = await onBatchToggleMod(ids, false);
    setSelectedIds(new Set());
    onReloadConfig();
    return result;
  }, [selectedIds, onBatchToggleMod, onReloadConfig]);

  const handleBatchDelete = useCallback(async () => {
    const ids = Array.from(selectedIds);
    if (ids.length === 0) return;
    await onBatchDeleteMod(ids);
    setSelectedIds(new Set());
    onReloadConfig();
  }, [selectedIds, onBatchDeleteMod, onReloadConfig]);

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

  const handleDragStart = (e: React.DragEvent, modId: string) => {
    const idx = sortedForDrag.findIndex(m => m.id === modId);
    e.dataTransfer.setData('text/plain', String(idx));
    (e.currentTarget as HTMLElement).classList.add('dragging');
  };
  const handleDragEnd = (e: React.DragEvent) => {
    (e.currentTarget as HTMLElement).classList.remove('dragging');
    document.querySelectorAll('.mod-card.drag-over').forEach(el => el.classList.remove('drag-over'));
  };
  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    const card = (e.currentTarget as HTMLElement).closest('.mod-card');
    if (card && !card.classList.contains('dragging')) {
      card.classList.add('drag-over');
    }
  };
  const handleDragLeave = (e: React.DragEvent) => {
    (e.currentTarget as HTMLElement).closest('.mod-card')?.classList.remove('drag-over');
  };
  const handleDrop = (e: React.DragEvent, toModId: string) => {
    e.preventDefault();
    (e.currentTarget as HTMLElement).closest('.mod-card')?.classList.remove('drag-over');
    const fromIndex = parseInt(e.dataTransfer.getData('text/plain'));
    const reordered = [...sortedForDrag];
    const toIndex = reordered.findIndex(m => m.id === toModId);
    if (fromIndex === toIndex || isNaN(fromIndex) || toIndex === -1) return;
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
          {(game.profiles?.length ?? 0) > 0 && (
            <div className="profile-dropdown">
              <button
                className={`profile-dropdown-toggle${activeProfile ? ' active-profile' : ''}`}
                onClick={() => setProfileMenuOpen(o => !o)}
                onBlur={() => setTimeout(() => setProfileMenuOpen(false), 200)}
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="12" height="12">
                  <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                  <polyline points="14 2 14 8 20 8" />
                </svg>
                {activeProfile ? activeProfile.name : 'Profile'}
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="10" height="10">
                  <polyline points="6 9 12 15 18 9" />
                </svg>
              </button>
              {profileMenuOpen && (
                <div className="profile-dropdown-menu">
                  {(game.profiles ?? []).map(p => (
                    <button key={p.id} className="profile-dropdown-item" onClick={() => { invoke('apply_profile', { gameId: game.id, profileId: p.id }).then(() => onReloadConfig()); setProfileMenuOpen(false); }}>
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="12" height="12">
                        <polyline points="20 6 9 17 4 12" />
                      </svg>
                      {p.name}
                      {p.id === game.activeProfileId && <span style={{ color: 'var(--signal-blue)', fontSize: '10px' }}>active</span>}
                    </button>
                  ))}
                  <div className="profile-dropdown-sep" />
                  <button className="profile-dropdown-item" onClick={() => { setProfileMenuOpen(false); onOpenProfiles(); }}>
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="12" height="12">
                      <circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
                    </svg>
                    Manage Profiles
                  </button>
                </div>
              )}
            </div>
          )}
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

      <div className="saves-row">
        <span className="settings-row-desc">
          {savesResult
            ? `${savesResult.length} save${savesResult.length !== 1 ? 's' : ''} found for ${game.name}`
            : `No saves detected for ${game.name}`}
        </span>
        <button type="button" className="btn btn-ghost btn-sm" onClick={() => setDetectOpen(true)}>Detect Saves</button>
      </div>

      {/* ── Filter bar ── */}
      {game.mods.length > 0 && (
        <div className="filter-bar">
          <div className="filter-bar-left">
            <div
              className={`mod-select-checkbox${allSelected ? ' selected' : ''}`}
              onClick={handleSelectAll}
              role="checkbox"
              aria-checked={allSelected}
              aria-label="Select all mods"
              tabIndex={0}
              onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleSelectAll(); } }}
            >
              {allSelected && (
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" width="14" height="14">
                  <polyline points="20 6 9 17 4 12" />
                </svg>
              )}
            </div>
            <div className="filter-search">
              <svg className="filter-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
              </svg>
              <input
                className="filter-search-input"
                type="text"
                placeholder={`Search ${game.mods.length} mods...`}
                onChange={(e) => handleSearchInput(e.target.value)}
                aria-label="Search mods"
              />
              {searchQuery && (
                <button className="filter-search-clear" onClick={() => { setSearchQuery(''); const el = document.querySelector('.filter-search-input') as HTMLInputElement; if (el) el.value = ''; }} aria-label="Clear search">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
                  </svg>
                </button>
              )}
            </div>
          </div>
          <div className="filter-sort">
            <label className="filter-sort-label" htmlFor="sort-by">Sort</label>
            <select
              id="sort-by"
              className="filter-sort-select"
              value={sortBy}
              onChange={(e) => setSortBy(e.target.value as SortBy)}
            >
              {hasPriority && <option value="priority">Priority</option>}
              <option value="name">Name</option>
              <option value="status">Status</option>
              <option value="source">Source</option>
            </select>
            <button
              className="btn btn-icon"
              onClick={() => setSortDir(d => d === 'asc' ? 'desc' : 'asc')}
              title={sortDir === 'asc' ? 'Ascending — click for descending' : 'Descending — click for ascending'}
              aria-label={`Sort direction: ${sortDir}`}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"
                style={{ transform: sortDir === 'desc' ? 'rotate(180deg)' : undefined }}>
                <line x1="12" y1="5" x2="12" y2="19" /><polyline points="19 12 12 19 5 12" />
              </svg>
            </button>
          </div>
        </div>
      )}

      {/* ── Category filter pills ── */}
      {(game.mods.length > 0 && categories.length > 1) && (
        <div className="category-bar">
          <button
            className={`category-pill${categoryFilter === '' ? ' active' : ''}`}
            onClick={() => setCategoryFilter('')}
          >
            All
          </button>
          {categories.map(([cat, count]) => (
            <button
              key={cat}
              className={`category-pill${categoryFilter === cat ? ' active' : ''}`}
              onClick={() => setCategoryFilter(cat === categoryFilter ? '' : cat)}
            >
              {cat} <span className="category-count">{count}</span>
            </button>
          ))}
        </div>
      )}

      {/* ── Tag filter (if tags exist) ── */}
      {allTags.length > 0 && (
        <div className="tag-bar">
          {allTags.map(t => (
            <button
              key={t}
              className={`tag-chip${tagFilter.has(t) ? ' active' : ''}`}
              onClick={() => setTagFilter(prev => {
                const next = new Set(prev);
                next.has(t) ? next.delete(t) : next.add(t);
                return next;
              })}
            >
              {t}
            </button>
          ))}
        </div>
      )}

      {/* ── Batch toolbar ── */}
      {selectedIds.size > 0 && (
        <div className="batch-bar">
          <span className="batch-count">{selectedIds.size} mod{selectedIds.size !== 1 ? 's' : ''} selected</span>
          <div className="toolbar-spacer" />
          <button className="btn btn-ghost btn-sm" onClick={handleBatchEnable}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14">
              <polyline points="4 17 10 11 4 5" /><line x1="12" y1="19" x2="20" y2="19" />
            </svg>
            Enable
          </button>
          <button className="btn btn-ghost btn-sm" onClick={handleBatchDisable}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14">
              <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
            </svg>
            Disable
          </button>
          <button className="btn btn-ghost btn-sm" onClick={handleBatchDelete} style={{ color: 'var(--signal-red)' }}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14">
              <polyline points="3 6 5 6 21 6" /><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
            </svg>
            Delete
          </button>
        </div>
      )}

      <div className="mod-list">
        {visibleMods.map((mod) => (
          <ModCard key={mod.id} mod={mod} showPriority={hasPriority}
            onToggle={() => onToggleMod(mod.id)} onDelete={() => onDeleteMod(mod.id)}
            onDragStart={(e) => hasPriority && mod.enabled && handleDragStart(e, mod.id)}
            onDragEnd={handleDragEnd} onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
            onDrop={(e) => handleDrop(e, mod.id)} conflictInfo={conflictMap[mod.id] ?? null}
            selected={selectedIds.has(mod.id)}
            onSelect={(e) => handleSelect(mod.id, e)}
            onInfo={() => onModInfo(mod.id)} />
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
        {noResults && (
          <div className="mod-list-empty">
            <div className="mod-list-empty-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
              </svg>
            </div>
            <div className="mod-list-empty-title">No mods match "{searchQuery}"</div>
            <div className="mod-list-empty-desc">Try a different search term or clear the filter.</div>
          </div>
        )}
      </div>

      {detectOpen && (
        <DetectSavesDialog
          gameId={game.id}
          gameName={game.name}
          onScanned={setSavesResult}
          onClose={() => setDetectOpen(false)}
        />
      )}
    </>
  );
}
