interface Props {
  gameName: string;
  modName: string;
  onConfirm: () => void;
  onClose: () => void;
}

export default function DeleteModDialog({ gameName, modName, onConfirm, onClose }: Props) {
  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={e => e.stopPropagation()} role="alertdialog" aria-modal="true" aria-label="Delete mod">
        <div className="dialog-header">
          <h2 className="dialog-title">Delete Mod</h2>
          <button className="btn btn-icon" onClick={onClose} aria-label="Close">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <div className="dialog-body">
          <p style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-sm)', lineHeight: 'var(--leading-relaxed)' }}>
            Are you sure you want to delete <strong style={{ color: 'var(--text-primary)' }}>{modName}</strong> from{' '}
            <strong style={{ color: 'var(--text-primary)' }}>{gameName}</strong>?
          </p>
          <p style={{ color: 'var(--text-muted)', fontSize: 'var(--text-xs)', marginTop: 'var(--space-2)' }}>
            If enabled, symlinks will be removed. Library files stay on disk.
          </p>
        </div>

        <div className="dialog-footer">
          <button className="btn btn-secondary" onClick={onClose}>Cancel</button>
          <button className="btn btn-destructive" onClick={onConfirm}>
            Delete
          </button>
        </div>
      </div>
    </div>
  );
}
