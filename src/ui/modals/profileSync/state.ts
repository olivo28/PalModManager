import type { MissingModInfo } from '../../../api/types';

export interface ProfileSyncSession {
  manifestPath: string;
  profileName: string;
  missingMods: MissingModInfo[];
  totalMods: number;
  initialMissingCount: number;
}

let activeSession: ProfileSyncSession | null = null;

export function getActiveSyncSession(): ProfileSyncSession | null {
  return activeSession;
}

export function setActiveSyncSession(session: ProfileSyncSession | null): void {
  activeSession = session;
}

export function removeMissingModFromSession(modId: string): boolean {
  if (!activeSession) return false;
  const initialLen = activeSession.missingMods.length;
  activeSession.missingMods = activeSession.missingMods.filter(m => m.id !== modId);
  return activeSession.missingMods.length < initialLen;
}
