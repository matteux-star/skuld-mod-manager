import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { AppConfig, Toast, DialogState } from './types';
import Sidebar from './components/Sidebar';
import ModList from './components/ModList';
import AddGameDialog from './components/AddGameDialog';
import EditGameDialog from './components/EditGameDialog';
import ImportModDialog from './components/ImportModDialog';
import DeleteModDialog from './components/DeleteModDialog';
import RemoveGameDialog from './components/RemoveGameDialog';
import BackupRestoreDialog from './components/BackupRestoreDialog';
import ToastContainer from './components/ToastContainer';

interface ImportResult { mod_id: string; mod_name: string; installed_files: string[]; warning: string | null; }
interface ToggleResult { success: boolean; conflict: { level: string; conflicts_with: string[]; overlapping_files: string[] } | null; deploy_results: [string, boolean, string][] | null; }

export default function App() {
  const [config, setConfig] = useState<AppConfig>({ version: 1, games: [] });
  const [selectedGameId, setSelectedGameId] = useState<string | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [dialog, setDialog] = useState<DialogState>({ mode: null });
  const [sevenZAvailable, setSevenZAvailable] = useState<boolean>(true);
  const selectedGame = config.games.find(g => g.id === selectedGameId) ?? null;

  const addToast = useCallback((type: Toast['type'], message: string) => {
    const id = crypto.randomUUID();
    setToasts(prev => [...prev, { id, type, message }]);
    setTimeout(() => setToasts(prev => prev.filter(t => t.id !== id)), 5000);
  }, []);

  useEffect(() => {
    invoke<AppConfig>('get_config').then(setConfig).catch(e => addToast('error', `Config: ${e}`));
    invoke<string>('check_7z_available').catch(() => { setSevenZAvailable(false); addToast('warning', '7z not found. Install p7zip-full.'); });
  }, []);

  const dismissToast = useCallback((id: string) => setToasts(prev => prev.filter(t => t.id !== id)), []);
  const openDialog = useCallback((mode: DialogState['mode'], gameId?: string, modId?: string) => setDialog({ mode, gameId, modId }), []);
  const closeDialog = useCallback(() => setDialog({ mode: null }), []);

  const pickDirectory = useCallback(async () => { try { return await open({ directory: true, multiple: false, title: 'Select directory' }) as string | null; } catch { addToast('error', 'File picker failed'); return null; } }, [addToast]);
  const pickArchive = useCallback(async () => { try { return await open({ multiple: false, title: 'Select archive', filters: [{ name: 'Archives', extensions: ['zip', '7z', 'rar'] }] }) as string | null; } catch { addToast('error', 'File picker failed'); return null; } }, [addToast]);
  const pickExecutable = useCallback(async () => { try { return await open({ multiple: false, title: 'Select executable' }) as string | null; } catch { addToast('error', 'File picker failed'); return null; } }, [addToast]);

  const handleAddGame = useCallback(async (name: string, path: string, gt: string) => {
    try { const u = await invoke<AppConfig>('add_game', { name, path, gameType: gt }); setConfig(u); setSelectedGameId(u.games[u.games.length-1]?.id??null); addToast('success', `Added ${name}`); closeDialog(); } catch (e) { addToast('error', `${e}`); }
  }, [addToast, closeDialog]);

  const handleEditGame = useCallback(async (gameId: string, newPath: string, launchPath: string) => {
    try {
      const u = await invoke<AppConfig>('edit_game_path', { gameId, newPath });
      if (launchPath) {
        const u2 = await invoke<AppConfig>('set_launch_path', { gameId, launchPath });
        setConfig(u2);
      } else {
        setConfig(u);
      }
      addToast('success', 'Updated');
      closeDialog();
    } catch (e) { addToast('error', `${e}`); }
  }, [addToast, closeDialog]);

  const handleRemoveGame = useCallback(async (gameId: string) => {
    const g = config.games.find(x => x.id === gameId);
    try { const u = await invoke<AppConfig>('remove_game', { gameId }); setConfig(u); if (selectedGameId === gameId) setSelectedGameId(null); addToast('success', `Removed ${g?.name ?? 'game'}`); closeDialog(); } catch (e) { addToast('error', `${e}`); }
  }, [config, selectedGameId, addToast, closeDialog]);

  const handleImportMod = useCallback(async (gameId: string, ap: string, mn: string) => {
    try { const r = await invoke<ImportResult>('import_mod', { gameId, archivePath: ap, modName: mn }); const u = await invoke<AppConfig>('get_config'); setConfig(u); addToast('success', `Imported ${r.mod_name} (${r.installed_files.length} files)`); closeDialog(); } catch (e) { addToast(String(e).includes('already exists') ? 'warning' : 'error', `${e}`); }
  }, [addToast, closeDialog]);

  const handleToggleMod = useCallback(async (gameId: string, modId: string) => {
    try { const r = await invoke<ToggleResult>('toggle_mod', { gameId, modId }); const u = await invoke<AppConfig>('get_config'); setConfig(u); const m = u.games.find(g => g.id === gameId)?.mods.find(x => x.id === modId); addToast('success', `${m?.name ?? 'Mod'} ${m?.enabled ? 'enabled' : 'disabled'}`); if (r.conflict?.level === 'warn') addToast('warning', `Conflict: ${r.conflict.conflicts_with.join(', ')}`); if (r.conflict?.level === 'block') addToast('error', `Blocked: ${r.conflict.conflicts_with.join(', ')}`); r.deploy_results?.forEach(([n, ok, msg]) => { if (!ok) addToast('error', `${n}: ${msg}`); }); } catch (e) { addToast('error', `${e}`); }
  }, [addToast]);

  const handleDeleteMod = useCallback(async (gameId: string, modId: string) => {
    const g = config.games.find(x => x.id === gameId); const m = g?.mods.find(x => x.id === modId);
    try { const u = await invoke<AppConfig>('delete_mod', { gameId, modId }); setConfig(u); addToast('success', `Deleted ${m?.name ?? 'mod'}`); closeDialog(); } catch (e) { addToast('error', `${e}`); }
  }, [config, addToast, closeDialog]);

  const handleReorder = useCallback(async (gameId: string, modIds: string[]) => {
    try { setConfig(await invoke<AppConfig>('reorder_mods', { gameId, modIds })); } catch (e) { addToast('error', `Reorder: ${e}`); }
  }, [addToast]);

  const handleDeployAll = useCallback(async () => {
    if (!selectedGameId) return;
    try { const results = await invoke<[string, boolean, string][]>('deploy_all', { gameId: selectedGameId }); results.forEach(([n, ok, msg]) => addToast(ok ? 'success' : 'error', `${n}: ${msg}`)); } catch (e) { addToast('error', `${e}`); }
  }, [selectedGameId, addToast]);

  const handlePurgeAll = useCallback(async () => {
    if (!selectedGameId) return;
    try { const results = await invoke<[string, boolean, string][]>('purge_all', { gameId: selectedGameId }); results.forEach(([n, ok, msg]) => addToast(ok ? 'success' : 'error', `${n}: ${msg}`)); } catch (e) { addToast('error', `${e}`); }
  }, [selectedGameId, addToast]);

  const handleLaunch = useCallback(async () => {
    if (!selectedGameId) return;
    try { const msg = await invoke<string>('launch_game', { gameId: selectedGameId }); addToast('success', msg); } catch (e) { addToast('error', `${e}`); }
  }, [selectedGameId, addToast]);

  const handleRestored = useCallback(async () => {
    try { setConfig(await invoke<AppConfig>('get_config')); addToast('success', 'Config restored'); } catch (e) { addToast('error', `${e}`); }
  }, [addToast]);

  return (
    <div className="app">
      <Sidebar games={config.games} selectedGameId={selectedGameId} onSelectGame={setSelectedGameId} onAddGame={() => openDialog('add-game')} />
      <div className="main-panel">
        {selectedGame ? (
          <ModList game={selectedGame}
            onToggleMod={(mid) => handleToggleMod(selectedGame.id, mid)}
            onImportMod={() => openDialog('import-mod', selectedGame.id)}
            onEditGame={() => openDialog('edit-game', selectedGame.id)}
            onDeleteGame={() => openDialog('remove-game', selectedGame.id)}
            onDeleteMod={(mid) => openDialog('delete-mod', selectedGame.id, mid)}
            onReorderMods={(mods) => handleReorder(selectedGame.id, mods.map(m => m.id))}
            onDeployAll={handleDeployAll} onPurgeAll={handlePurgeAll} onLaunchGame={handleLaunch}
            onBackupRestore={() => openDialog('backup-restore')} />
        ) : (
          <div className="mod-list-empty"><div className="mod-list-empty-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><line x1="3" y1="9" x2="21" y2="9"/><line x1="9" y1="21" x2="9" y2="9"/></svg></div><div className="mod-list-empty-title">Select a game</div><div className="mod-list-empty-desc">Choose a game from the sidebar to manage its mods.</div></div>
        )}
      </div>

      {dialog.mode === 'add-game' && <AddGameDialog onPickDirectory={pickDirectory} onAdd={handleAddGame} onClose={closeDialog} />}
      {dialog.mode === 'edit-game' && dialog.gameId && (
        <EditGameDialog currentPath={selectedGame?.path ?? ''} currentLaunchPath={selectedGame?.launchPath} gameName={selectedGame?.name ?? ''}
          onPickDirectory={pickDirectory} onPickExecutable={pickExecutable}
          onSave={(np, lp) => handleEditGame(dialog.gameId!, np, lp)} onClose={closeDialog} />
      )}
      {dialog.mode === 'remove-game' && dialog.gameId && <RemoveGameDialog gameName={selectedGame?.name ?? ''} onConfirm={() => handleRemoveGame(dialog.gameId!)} onClose={closeDialog} />}
      {dialog.mode === 'import-mod' && dialog.gameId && <ImportModDialog existingMods={selectedGame?.mods ?? []} onPickArchive={pickArchive} onImport={(ap, mn) => handleImportMod(dialog.gameId!, ap, mn)} onClose={closeDialog} sevenZAvailable={sevenZAvailable} />}
      {dialog.mode === 'delete-mod' && dialog.gameId && dialog.modId && <DeleteModDialog gameName={selectedGame?.name ?? ''} modName={selectedGame?.mods.find(m => m.id === dialog.modId)?.name ?? ''} onConfirm={() => handleDeleteMod(dialog.gameId!, dialog.modId!)} onClose={closeDialog} />}
      {dialog.mode === 'backup-restore' && <BackupRestoreDialog onClose={closeDialog} onRestored={handleRestored} />}
      <ToastContainer toasts={toasts} onDismiss={dismissToast} />
    </div>
  );
}
