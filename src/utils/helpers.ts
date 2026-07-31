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
    default: return '??';
  }
}
