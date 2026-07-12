import type { GameEntry } from '../types';

interface Props {
  games: GameEntry[];
  selectedGameId: string | null;
  onSelectGame: (id: string) => void;
  onAddGame: () => void;
  onOpenSettings: () => void;
  settingsActive: boolean;
}

const GAME_ICONS: Record<string, string> = {
  witcher3: '⚔️',
  sod2: '🧟',
};

export default function Sidebar({ games, selectedGameId, onSelectGame, onAddGame, onOpenSettings, settingsActive }: Props) {
  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <div className="sidebar-brand">
          <div className="sidebar-brand-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M15 6v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3V6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3" />
            </svg>
          </div>
          <span className="sidebar-brand-text">Skuld</span>
          <button
            type="button"
            className={`sidebar-settings-btn${settingsActive ? ' active' : ''}`}
            aria-label="Settings"
            onClick={onOpenSettings}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" width="16" height="16">
              <path d="M10.3 2h3.4l.5 2.4a7.9 7.9 0 0 1 1.8 1l2.3-.9 1.7 3-1.7 1.6c.1.4.1.9.1 1.3s0 .9-.1 1.3l1.7 1.6-1.7 3-2.3-.9a7.9 7.9 0 0 1-1.8 1l-.5 2.4h-3.4l-.5-2.4a7.9 7.9 0 0 1-1.8-1l-2.3.9-1.7-3 1.7-1.6a6.9 6.9 0 0 1 0-2.6L4 8.5l1.7-3 2.3.9a7.9 7.9 0 0 1 1.8-1z" />
              <circle cx="12" cy="12" r="3" />
            </svg>
          </button>
        </div>

        {games.length > 0 && (
          <div className="sidebar-section-label">Games</div>
        )}
      </div>

      <nav className="game-list">
        {games.map(game => (
          <button
            key={game.id}
            className={`game-item${!settingsActive && game.id === selectedGameId ? ' active' : ''}`}
            onClick={() => onSelectGame(game.id)}
          >
            <span className="game-item-icon" aria-hidden="true">
              {GAME_ICONS[game.type] ?? '🎮'}
            </span>
            <span className="game-item-name">{game.name}</span>
            <span className={`badge-support ${game.supportStatus}`}>
              {game.supportStatus}
            </span>
          </button>
        ))}

        {games.length === 0 && (
          <div style={{ padding: 'var(--space-4)', color: 'var(--text-muted)', fontSize: 'var(--text-sm)', textAlign: 'center' }}>
            No games added yet.
          </div>
        )}
      </nav>

      <div className="sidebar-footer">
        <button className="btn-add-game" onClick={onAddGame}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
          Add Game
        </button>
      </div>
    </aside>
  );
}
