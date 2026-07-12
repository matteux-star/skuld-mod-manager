export type GameType = 'witcher3' | 'sod2';
export type SupportStatus = 'verified' | 'provisional';

export interface ModEntry {
  id: string;
  name: string;
  archiveSource: string;
  enabled: boolean;
  priority: number;
  installedFiles: string[];
  version?: string;
  author?: string;
  description?: string;
  sourceUrl?: string;
  category?: string;
  tags: string[];
  installedAt?: string;
  updatedAt?: string;
  conflictState?: 'none' | 'warn' | 'block';
  conflictWith?: string[];
}

export interface ModState {
  modId: string;
  enabled: boolean;
  priority: number;
}

export interface Profile {
  id: string;
  name: string;
  gameId: string;
  modStates: ModState[];
  createdAt: string;
}

export interface GameEntry {
  id: string;
  type: GameType;
  name: string;
  path: string;
  launchPath?: string;
  supportStatus: SupportStatus;
  mods: ModEntry[];
  activeProfileId?: string;
  profiles: Profile[];
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
  | 'profiles'
  | 'mod-info'
  | null;

export interface DialogState {
  mode: DialogMode;
  gameId?: string;
  modId?: string;
}

export interface BackupInfo {
  filename: string;
  timestamp: string;
  gameCount: number;
  modCount: number;
}

export interface SaveFile {
  name: string;
  path: string;
  sizeBytes: number;
  modified: string;
  isAutosave: boolean;
  isQuicksave: boolean;
}
