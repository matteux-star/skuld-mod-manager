interface Props {
  gameName: string;
  onConfirm: () => void;
  onClose: () => void;
}

export default function RemoveGameDialog({ gameName, onConfirm, onClose }: Props) {
  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={e => e.stopPropagation()} role="alertdialog" aria-modal="true" aria-label="Remove game">
        <div className="dialog-header">
          <h2 className="dialog-title">Remove Game</h2>
          <button className="btn btn-icon" onClick={onClose} aria-label="Close">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <div className="dialog-body">
          <p style={{ color: 'var(--text-secondary)', lineHeight: 'var(--leading-relaxed)' }}>
            Remove <strong style={{ color: 'var(--text-primary)' }}>{gameName}</strong> from the manager?
          </p>
          <p style={{ color: 'var(--text-muted)', fontSize: 'var(--text-xs)', marginTop: 'var(--space-2)' }}>
            All symlinks will be removed and every mod's entry in the list will be deleted.
            Extracted mod files stay on disk but become orphaned — re-add the game and use
            Diagnostics → Recover to bring them back.
          </p>
        </div>

        <div className="dialog-footer">
          <button className="btn btn-secondary" onClick={onClose}>Cancel</button>
          <button className="btn btn-destructive" onClick={onConfirm}>Remove</button>
        </div>
      </div>
    </div>
  );
}
