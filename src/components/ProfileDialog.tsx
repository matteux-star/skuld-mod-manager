import { useState } from 'react';
import type { Profile } from '../types';
import { invoke } from '@tauri-apps/api/core';

interface Props {
  gameId: string;
  profiles: Profile[];
  activeProfileId: string | undefined;
  onClose: () => void;
  onProfileChanged: () => void;
}

export default function ProfileDialog({ gameId, profiles, activeProfileId, onClose, onProfileChanged }: Props) {
  const [newName, setNewName] = useState('');
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');
  const [error, setError] = useState('');

  const handleCreate = async () => {
    const name = newName.trim();
    if (!name) { setError('Profile name is required'); return; }
    if (profiles.some(p => p.name === name)) { setError('A profile with this name already exists'); return; }

    try {
      await invoke('create_profile', { gameId, name });
      setNewName('');
      setError('');
      onProfileChanged();
    } catch (e) { setError(String(e)); }
  };

  const handleApply = async (profileId: string) => {
    try {
      await invoke('apply_profile', { gameId, profileId });
      onProfileChanged();
    } catch (e) { setError(String(e)); }
  };

  const handleDelete = async (profileId: string) => {
    try {
      await invoke('delete_profile', { gameId, profileId });
      onProfileChanged();
    } catch (e) { setError(String(e)); }
  };

  const handleStartRename = (profile: Profile) => {
    setRenamingId(profile.id);
    setRenameValue(profile.name);
  };

  const handleRenameConfirm = async (profileId: string) => {
    const name = renameValue.trim();
    if (!name) return;
    try {
      await invoke('rename_profile', { gameId, profileId, name });
      setRenamingId(null);
      onProfileChanged();
    } catch (e) { setError(String(e)); }
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={(e) => e.stopPropagation()}>
        <div className="dialog-header">
          <h2 className="dialog-title">Mod Profiles</h2>
          <button className="btn btn-icon" onClick={onClose} aria-label="Close">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <div className="dialog-body">
          {profiles.length === 0 && (
            <p style={{ color: 'var(--text-muted)', fontSize: 'var(--text-sm)', textAlign: 'center', padding: 'var(--space-4) 0' }}>
              No profiles yet. Save your current mod setup as a profile to switch between different playthroughs.
            </p>
          )}

          <div className="profile-list">
            {profiles.map(p => (
              <div key={p.id} className={`profile-item${p.id === activeProfileId ? ' active' : ''}`}>
                <div className="profile-item-left">
                  {renamingId === p.id ? (
                    <input
                      className="input"
                      style={{ height: 28, fontSize: 'var(--text-sm)' }}
                      value={renameValue}
                      onChange={(e) => setRenameValue(e.target.value)}
                      onBlur={() => handleRenameConfirm(p.id)}
                      onKeyDown={(e) => { if (e.key === 'Enter') handleRenameConfirm(p.id); if (e.key === 'Escape') setRenamingId(null); }}
                      autoFocus
                    />
                  ) : (
                    <>
                      <span className="profile-name">{p.name}</span>
                      {p.id === activeProfileId && <span className="badge-status enabled">Active</span>}
                      <span className="profile-meta">{p.modStates.length} mods · created {p.createdAt.slice(0, 10)}</span>
                    </>
                  )}
                </div>
                <div className="profile-item-right">
                  {p.id !== activeProfileId && (
                    <button className="btn btn-ghost btn-sm" onClick={() => handleApply(p.id)}>Activate</button>
                  )}
                  {renamingId !== p.id && (
                    <button className="btn btn-icon" onClick={() => handleStartRename(p)} aria-label={`Rename ${p.name}`}>
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14">
                        <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
                        <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
                      </svg>
                    </button>
                  )}
                  <button className="btn btn-icon" onClick={() => handleDelete(p.id)} aria-label={`Delete ${p.name}`} style={{ color: 'var(--signal-red)' }}>
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14">
                      <polyline points="3 6 5 6 21 6" /><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                    </svg>
                  </button>
                </div>
              </div>
            ))}
          </div>

          <div style={{ marginTop: 'var(--space-4)', display: 'flex', gap: 'var(--space-2)' }}>
            <input
              className="input"
              placeholder="New profile name..."
              value={newName}
              onChange={(e) => { setNewName(e.target.value); setError(''); }}
              onKeyDown={(e) => { if (e.key === 'Enter') handleCreate(); }}
              style={{ flex: 1 }}
            />
            <button className="btn btn-primary btn-sm" onClick={handleCreate} disabled={!newName.trim()}>
              Create
            </button>
          </div>
          {error && <div className="input-error" style={{ marginTop: 'var(--space-2)' }}>{error}</div>}
        </div>
      </div>
    </div>
  );
}
