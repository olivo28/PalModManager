export const DEFAULT_AVATAR = 'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="%23da8e35"><path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/></svg>';

export let isListening = false;
export function setIsListening(val: boolean) { isListening = val; }

export function isNexusModEndorsed(nexusModId: number | string | undefined | null, endorsementsCache?: Array<{ modId: number | string; status?: string }>): boolean {
  if (!nexusModId || !endorsementsCache || !Array.isArray(endorsementsCache)) return false;
  const targetId = Number(nexusModId);
  if (!targetId) return false;
  return endorsementsCache.some(e => {
    if (Number(e.modId) !== targetId) return false;
    const status = (e.status || '').toLowerCase().trim();
    return !status || status === 'endorsed';
  });
}

export function isNexusModTracked(nexusModId: number | string | undefined | null, trackedCache?: Array<{ modId: number | string }>): boolean {
  if (!nexusModId || !trackedCache || !Array.isArray(trackedCache)) return false;
  const targetId = Number(nexusModId);
  if (!targetId) return false;
  return trackedCache.some(t => Number(t.modId) === targetId);
}

