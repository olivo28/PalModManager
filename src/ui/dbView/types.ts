export type DbTable = 'mods' | 'profiles' | 'settings' | 'usmap';

export interface ModInfo {
  id: string;
  name: string;
  type: string;
  enabled: boolean;
  version: string;
  installDate: string;
  gamePath: string;
  nexusAuthor: string | null;
}

export interface Profile {
  id: string;
  name: string;
  createdAt: string;
  installedModIds: string[];
  enabledModIds: string[];
}

export interface AppSettings {
  gamePath: string;
  programPath: string;
  hideNativeMods: boolean | null;
  debugConsole: boolean | null;
  customDataPath: string | null;
  language?: string | null;
}

export interface DbSnapshot {
  mods: ModInfo[];
  profiles: Profile[];
  currentProfileId: string;
  settings: AppSettings;
}
