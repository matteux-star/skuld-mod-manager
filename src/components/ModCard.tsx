import type { ModEntry } from '../types';

interface ConflictDisplay {
  level: string;
  conflictsWith: string[];
}

interface Props {
  mod: ModEntry;
  showPriority: boolean;
  onToggle: () => void;
  onDelete: () => void;
  onDragStart?: (e: React.DragEvent) => void;
  onDragEnd?: (e: React.DragEvent) => void;
  onDragOver?: (e: React.DragEvent) => void;
  onDragLeave?: (e: React.DragEvent) => void;
  onDrop?: (e: React.DragEvent) => void;
  conflictInfo: ConflictDisplay | null;
  selected?: boolean;
  onSelect?: (e: React.MouseEvent) => void;
  onInfo?: () => void;
}

export default function ModCard({
  mod,
  showPriority,
  onToggle,
  onDelete,
  onDragStart,
  onDragEnd,
  onDragOver,
  onDragLeave,
  onDrop,
  conflictInfo,
  selected,
  onSelect,
  onInfo,
}: Props) {
  const hasMetadata = !!(mod.version || mod.author || mod.category || mod.tags?.length);
  return (
    <div
      className={`mod-card${!mod.enabled ? ' disabled' : ''}`}
      draggable={showPriority && mod.enabled}
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      onDragOver={onDragOver}
      onDragLeave={onDragLeave}
      onDrop={onDrop}
      role="listitem"
      aria-label={`${mod.name}, ${mod.enabled ? 'enabled' : 'disabled'}${showPriority ? `, priority ${mod.priority}` : ''}`}
    >
      <div
        className={`mod-select-checkbox${selected ? ' selected' : ''}`}
        onClick={(e) => { e.stopPropagation(); onSelect?.(e); }}
        role="checkbox"
        aria-checked={selected ?? false}
        aria-label={`Select ${mod.name}`}
        tabIndex={0}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onSelect?.(e as unknown as React.MouseEvent); } }}
      >
        {selected && (
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" width="14" height="14">
            <polyline points="20 6 9 17 4 12" />
          </svg>
        )}
      </div>

      {showPriority && mod.enabled && (
        <div className="mod-priority-handle" aria-label="Drag to reorder priority">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
            <circle cx="9" cy="5" r="1" fill="currentColor" stroke="none" />
            <circle cx="15" cy="5" r="1" fill="currentColor" stroke="none" />
            <circle cx="9" cy="12" r="1" fill="currentColor" stroke="none" />
            <circle cx="15" cy="12" r="1" fill="currentColor" stroke="none" />
            <circle cx="9" cy="19" r="1" fill="currentColor" stroke="none" />
            <circle cx="15" cy="19" r="1" fill="currentColor" stroke="none" />
          </svg>
        </div>
      )}

      {showPriority && mod.enabled && (
        <div className="mod-priority-num">{mod.priority}</div>
      )}

      <div className="mod-info">
        <div className="mod-name">
          {mod.name}
          {onInfo && (
            <button className="btn btn-icon mod-info-btn" onClick={onInfo} aria-label={`Edit metadata for ${mod.name}`} title="Edit metadata" style={{ width: 20, height: 20, marginLeft: 'var(--space-1)' }}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="12" height="12">
                <circle cx="12" cy="12" r="10" /><line x1="12" y1="16" x2="12" y2="12" /><line x1="12" y1="8" x2="12.01" y2="8" />
              </svg>
            </button>
          )}
        </div>
        {hasMetadata && (
          <div className="mod-metadata-line">
            {mod.version && <span className="mod-metadata-tag version">v{mod.version}</span>}
            {mod.author && <span className="mod-metadata-tag author">by {mod.author}</span>}
            {mod.category && <span className="mod-metadata-tag category">{mod.category}</span>}
          </div>
        )}
        {mod.installedFiles.length > 0 && (
          <div className="mod-path">{mod.installedFiles[0]}{mod.installedFiles.length > 1 ? ` +${mod.installedFiles.length - 1} more` : ''}</div>
        )}
        <div className="mod-source">{mod.archiveSource}</div>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
        {conflictInfo?.level === 'warn' && (
          <span className="badge-status conflict-warn" title={`Conflicts with: ${conflictInfo.conflictsWith.join(', ')}`}>
            Conflict
          </span>
        )}
        {conflictInfo?.level === 'block' && (
          <span className="badge-status conflict-block" title={`Blocked by: ${conflictInfo.conflictsWith.join(', ')}`}>
            Blocked
          </span>
        )}
        {mod.enabled && !conflictInfo && (
          <span className="badge-status enabled">Enabled</span>
        )}

        <label className="toggle">
          <input
            type="checkbox"
            checked={mod.enabled}
            onChange={onToggle}
            disabled={conflictInfo?.level === 'block'}
            aria-label={`${mod.enabled ? 'Disable' : 'Enable'} ${mod.name}`}
          />
          <span className="toggle-track"><span className="toggle-knob" /></span>
        </label>

        <button className="btn btn-icon" onClick={onDelete} aria-label={`Delete ${mod.name}`} style={{ color: 'var(--text-muted)' }}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="3 6 5 6 21 6" />
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
          </svg>
        </button>
      </div>
    </div>
  );
}
