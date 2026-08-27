export function escapeHtml(str: string): string {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

export function getTypeLabel(type: string): string {
  switch (type) {
    case 'ue4ss': return 'LUA';
    case 'palschema': return 'PS';
    case 'pak': return 'PAK';
    case 'logicmods': return 'LM';
    case 'hybrid': return 'HY';
    default: return '??';
  }
}

export function formatBytes(bytes: number, decimals = 1): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}
