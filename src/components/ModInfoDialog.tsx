import { useState } from 'react';
import type { ModEntry } from '../types';
import { invoke } from '@tauri-apps/api/core';

interface Props {
  gameId: string;
  mod: ModEntry;
  onClose: () => void;
  onSaved: () => void;
}

const CATEGORIES = ['', 'Gameplay', 'Graphics', 'UI', 'Audio', 'Patches', 'Overhaul', 'Utilities', 'Other'];

export default function ModInfoDialog({ gameId, mod, onClose, onSaved }: Props) {
  const [name, setName] = useState(mod.name);
  const [version, setVersion] = useState(mod.version ?? '');
  const [author, setAuthor] = useState(mod.author ?? '');
  const [description, setDescription] = useState(mod.description ?? '');
  const [sourceUrl, setSourceUrl] = useState(mod.sourceUrl ?? '');
  const [category, setCategory] = useState(mod.category ?? '');
  const [tagsInput, setTagsInput] = useState((mod.tags ?? []).join(', '));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  const handleSave = async () => {
    setSaving(true);
    setError('');
    try {
      const tags = tagsInput.split(',').map(t => t.trim()).filter(Boolean);
      await invoke('update_mod_metadata', {
        gameId,
        modId: mod.id,
        name: name || null,
        version: version || null,
        author: author || null,
        description: description || null,
        sourceUrl: sourceUrl || null,
        category: category || null,
        tags: tags.length > 0 ? tags : null,
      });
      onSaved();
      onClose();
    } catch (e) {
      setError(String(e));
      setSaving(false);
    }
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" style={{ maxWidth: 520 }} onClick={(e) => e.stopPropagation()}>
        <div className="dialog-header">
          <h2 className="dialog-title">Mod Info: {mod.name}</h2>
          <button className="btn btn-icon" onClick={onClose} aria-label="Close">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <div className="dialog-body">
          <div className="input-group">
            <label className="input-label">Name</label>
            <input className="input" value={name} onChange={(e) => setName(e.target.value)} />
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 'var(--space-4)' }}>
            <div className="input-group">
              <label className="input-label">Version</label>
              <input className="input input-mono" value={version} onChange={(e) => setVersion(e.target.value)} placeholder="e.g. 1.4.2" />
            </div>
            <div className="input-group">
              <label className="input-label">Author</label>
              <input className="input" value={author} onChange={(e) => setAuthor(e.target.value)} placeholder="e.g. modder123" />
            </div>
          </div>
          <div className="input-group">
            <label className="input-label">Category</label>
            <select className="filter-sort-select" style={{ width: '100%', height: 36 }} value={category} onChange={(e) => setCategory(e.target.value)}>
              <option value="">None</option>
              {CATEGORIES.filter(c => c).map(c => <option key={c} value={c}>{c}</option>)}
            </select>
          </div>
          <div className="input-group">
            <label className="input-label">Tags (comma separated)</label>
            <input className="input" value={tagsInput} onChange={(e) => setTagsInput(e.target.value)} placeholder="e.g. lore-friendly, performance" />
          </div>
          <div className="input-group">
            <label className="input-label">Source URL</label>
            <input className="input input-mono" value={sourceUrl} onChange={(e) => setSourceUrl(e.target.value)} placeholder="https://www.nexusmods.com/..." />
          </div>
          <div className="input-group">
            <label className="input-label">Description</label>
            <textarea
              className="input"
              style={{ height: 80, resize: 'vertical', padding: 'var(--space-2) var(--space-3)', fontFamily: 'var(--font-ui)' }}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Brief description of what this mod does..."
              maxLength={500}
            />
            <div className="input-hint">{description.length}/500</div>
          </div>
          {error && <div className="input-error">{error}</div>}
        </div>

        <div className="dialog-footer">
          <button className="btn btn-secondary" onClick={onClose}>Cancel</button>
          <button className="btn btn-primary" onClick={handleSave} disabled={saving}>
            {saving ? 'Saving...' : 'Save'}
          </button>
        </div>
      </div>
    </div>
  );
}
