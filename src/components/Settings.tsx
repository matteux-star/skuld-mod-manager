import type { AppSettings, DeployMethod } from '../types';

interface Props {
  settings: AppSettings;
  onChange: (patch: Partial<AppSettings>) => void;
  onClose: () => void;
  onClearCache: () => void;
  onResetSettings: () => void;
}

export default function Settings({ settings, onChange, onClose, onClearCache, onResetSettings }: Props) {
  return (
    <div className="settings-view">
      <div className="settings-header">
        <h2 className="settings-title">Settings</h2>
        <button className="btn btn-ghost btn-sm" onClick={onClose}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" width="14" height="14">
            <line x1="19" y1="12" x2="5" y2="12" /><polyline points="12 19 5 12 12 5" />
          </svg>
          Back
        </button>
      </div>

      <div className="settings-body">
        <div className="card">
          <div className="card-kicker">General</div>
          <div className="settings-row">
            <div>
              <div className="settings-row-label">Launch on startup</div>
              <div className="settings-row-desc">Open Skuld when you log in</div>
            </div>
            <label className="toggle">
              <input type="checkbox" checked={settings.launchOnStartup} onChange={e => onChange({ launchOnStartup: e.target.checked })} aria-label="Launch on startup" />
              <span className="toggle-track"><span className="toggle-knob" /></span>
            </label>
          </div>
          <div className="hr" />
          <div className="settings-row">
            <div>
              <div className="settings-row-label">Check for mod updates</div>
              <div className="settings-row-desc">Notify when a subscribed mod changes</div>
            </div>
            <label className="toggle">
              <input type="checkbox" checked={settings.checkUpdates} onChange={e => onChange({ checkUpdates: e.target.checked })} aria-label="Check for mod updates" />
              <span className="toggle-track"><span className="toggle-knob" /></span>
            </label>
          </div>
        </div>

        <div className="card">
          <div className="card-kicker">Deployment</div>
          <div className="input-group" style={{ margin: 0 }}>
            <label className="input-label">Method</label>
            <div className="seg">
              {(['symlink', 'hardlink', 'copy'] as DeployMethod[]).map(m => (
                <label key={m} className={`seg-opt${settings.deployMethod === m ? ' active' : ''}`}>
                  <input
                    type="radio"
                    name="deploy-method"
                    checked={settings.deployMethod === m}
                    onChange={() => onChange({ deployMethod: m })}
                  />
                  {m === 'symlink' ? 'Symlink' : m === 'hardlink' ? 'Hardlink' : 'Copy'}
                </label>
              ))}
            </div>
          </div>
          <div className="hr" />
          <div className="settings-row">
            <div>
              <div className="settings-row-label">Warn if game is running</div>
              <div className="settings-row-desc">Block deploy while the game process is active</div>
            </div>
            <label className="toggle">
              <input type="checkbox" checked={settings.warnIfRunning} onChange={e => onChange({ warnIfRunning: e.target.checked })} aria-label="Warn if game is running" />
              <span className="toggle-track"><span className="toggle-knob" /></span>
            </label>
          </div>
        </div>

        <div className="card">
          <div className="card-kicker">Backups</div>
          <div className="settings-row">
            <div className="settings-row-label">Auto-backup before deploy</div>
            <label className="toggle">
              <input type="checkbox" checked={settings.autoBackup} onChange={e => onChange({ autoBackup: e.target.checked })} aria-label="Auto-backup before deploy" />
              <span className="toggle-track"><span className="toggle-knob" /></span>
            </label>
          </div>
          <div className="hr" />
          <div>
            <div className="settings-row" style={{ marginBottom: 'var(--space-2)' }}>
              <span className="settings-row-label">Keep last backups</span>
              <span className="settings-slider-value">{settings.backupRetention}</span>
            </div>
            <input
              type="range"
              min={1}
              max={20}
              value={settings.backupRetention}
              onChange={e => onChange({ backupRetention: parseInt(e.target.value, 10) })}
              style={{ width: '100%' }}
              aria-label="Backups to keep"
            />
          </div>
        </div>

        <div className="card card-danger">
          <div className="card-kicker card-kicker-danger">Danger zone</div>
          <div style={{ display: 'flex', gap: 'var(--space-2)' }}>
            <button className="btn btn-secondary btn-sm" onClick={onClearCache}>Clear cache</button>
            <button className="btn btn-secondary btn-sm" onClick={onResetSettings}>Reset settings</button>
          </div>
        </div>
      </div>
    </div>
  );
}
