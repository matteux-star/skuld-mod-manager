export type GameType = 'witcher3' | 'sod2';
export type SupportStatus = 'verified' | 'provisional';

export interface ModEntry {
  id: string;
  name: string;
  archiveSource: string;
  enabled: boolean;
  priority: number;
  installedFiles: string[];
  conflictState?: 'none' | 'warn' | 'block';
  conflictWith?: string[];
}

export interface GameEntry {
  id: string;
  type: GameType;
  name: string;
  path: string;
  launchPath?: string;
  supportStatus: SupportStatus;
  mods: ModEntry[];
}

export interface AppConfig {
  version: number;
  games: GameEntry[];
}

export interface Toast {
  id: string;
  type: 'success' | 'warning' | 'error';
  message: string;
}

export interface ImportResult {
  modId: string;
  modName: string;
  installedFiles: string[];
  warning: string | null;
}

export interface ConflictInfo {
  level: string;
  conflictsWith: string[];
  overlappingFiles: string[];
}

export interface ToggleResult {
  success: boolean;
  conflict: ConflictInfo | null;
  deployResults: [string, boolean, string][] | null;
}

export type DialogMode =
  | 'add-game'
  | 'edit-game'
  | 'remove-game'
  | 'import-mod'
  | 'update-mod'
  | 'delete-mod'
  | 'backup-restore'
  | null;

export interface DialogState {
  mode: DialogMode;
  gameId?: string;
  modId?: string;
}

export interface BackupInfo {
  filename: string;
  timestamp: string;
  game_count: number;
  mod_count: number;
}

export interface SaveFile {
  name: string;
  path: string;
  size_bytes: number;
  modified: string;
  is_autosave: boolean;
  is_quicksave: boolean;
}
