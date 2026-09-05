export function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(0) + ' KB';
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

export function parseModFilename(filename: string): { name: string; version: string | null; nexusId: number | null } {
  const stem = filename.replace(/\.(zip|rar)$/i, '');
  const parts = stem.split(/[ _()]/).filter(s => s);

  const idIdx = parts.findIndex(p => {
    const num = parseInt(p, 10);
    return !isNaN(num) && num >= 100 && num <= 99999 && num !== 2026 && num !== 2025 && num !== 2024;
  });

  if (idIdx >= 0) {
    const name = parts.slice(0, idIdx).join(' ');
    let version: string | null = null;
    const nexusId = parseInt(parts[idIdx], 10);
    if (idIdx + 1 < parts.length) {
      const next = parts[idIdx + 1];
      if (/^[v\d]/.test(next) && !next.includes('-')) {
        version = next;
      }
    }
    return { name: name || stem, version, nexusId: isNaN(nexusId) ? null : nexusId };
  }
  return { name: stem, version: null, nexusId: null };
}

export function compareVersions(a: string, b: string): number {
  const parseParts = (v: string) => v.replace(/^[^\d]*/, '').split(/[\.-]/).map(n => parseInt(n, 10) || 0);
  const partsA = parseParts(a);
  const partsB = parseParts(b);
  for (let i = 0; i < Math.max(partsA.length, partsB.length); i++) {
    const numA = partsA[i] || 0;
    const numB = partsB[i] || 0;
    if (numA !== numB) return numA - numB;
  }
  return a.localeCompare(b);
}

