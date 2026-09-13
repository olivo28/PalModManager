import { compareVersions } from '../../mods/library/helpers';

/**
 * Normalizes a version string by trimming whitespace and removing leading 'v' or 'V'.
 */
export function normalizeVersionToken(v?: string | null): string {
  if (!v) return '';
  const trimmed = v.trim();
  return trimmed.replace(/^[vV]/, '').trim();
}

/**
 * Checks if a version string is valid (not empty or generic placeholder).
 */
export function isValidVersionCandidate(v?: string | null): boolean {
  if (!v) return false;
  const t = v.trim().toLowerCase();
  return t !== '' && t !== 'unknown' && t !== 'null' && t !== 'none';
}

/**
 * Resolves the version of a FOMOD mod using majority consensus ("more =") among the three primary sources:
 * 1. xmlVer: from fomod/info.xml
 * 2. zipVer: from the .zip filename
 * 3. apiVer: from Nexus API / cached metadata
 *
 * If two or more sources agree, that agreed version is selected.
 * If all sources disagree or only one exists, the newest/highest valid version is chosen.
 */
export function resolveFomodVersionConsensus(
  xmlVer?: string | null,
  zipVer?: string | null,
  apiVer?: string | null,
  fallback: string = '1.0.0'
): string {
  const candidates: string[] = [];

  if (isValidVersionCandidate(xmlVer)) candidates.push(xmlVer!.trim());
  if (isValidVersionCandidate(zipVer)) candidates.push(zipVer!.trim());
  if (isValidVersionCandidate(apiVer)) candidates.push(apiVer!.trim());

  if (candidates.length === 0) return fallback;
  if (candidates.length === 1) return candidates[0];

  const normXml = isValidVersionCandidate(xmlVer) ? normalizeVersionToken(xmlVer) : null;
  const normZip = isValidVersionCandidate(zipVer) ? normalizeVersionToken(zipVer) : null;
  const normApi = isValidVersionCandidate(apiVer) ? normalizeVersionToken(apiVer) : null;

  // 1. Majority consensus checks
  // If XML == API -> XML/API wins
  if (normXml && normApi && normXml === normApi) {
    return xmlVer!.trim();
  }

  // If XML == ZIP -> XML/ZIP wins
  if (normXml && normZip && normXml === normZip) {
    return xmlVer!.trim();
  }

  // If ZIP == API -> ZIP/API wins
  if (normZip && normApi && normZip === normApi) {
    return zipVer!.trim();
  }

  // 2. Disagreement / No majority -> Pick newest/highest valid candidate
  let best = candidates[0];
  for (let i = 1; i < candidates.length; i++) {
    if (compareVersions(candidates[i], best) > 0) {
      best = candidates[i];
    }
  }

  return best;
}
