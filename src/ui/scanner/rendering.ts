export const customStyles = `
  <style>
    .scanner-sub-tabs {
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 20px;
      padding: 3px;
      display: inline-flex;
      gap: 2px;
    }
    .scanner-sub-tab {
      border: none;
      background: transparent;
      color: var(--text-muted);
      font-size: 11px;
      font-weight: 600;
      padding: 6px 14px;
      border-radius: 17px;
      cursor: pointer;
      transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
      outline: none;
    }
    .scanner-sub-tab.active {
      background: var(--accent);
      color: #000 !important;
      font-weight: 700;
      box-shadow: 0 2px 8px rgba(0, 188, 255, 0.3);
    }
    .scanner-sub-tab:hover:not(.active) {
      color: var(--text-primary);
      background: rgba(255, 255, 255, 0.05);
    }

    .premium-stat-card {
      background: linear-gradient(135deg, rgba(255, 255, 255, 0.03) 0%, rgba(255, 255, 255, 0.01) 100%);
      border: 1px solid rgba(255, 255, 255, 0.06);
      border-radius: 12px;
      padding: 14px 20px;
      min-width: 130px;
      transition: all 0.3s ease;
      position: relative;
      overflow: hidden;
    }
    .premium-stat-card::before {
      content: '';
      position: absolute;
      top: 0; left: 0; right: 0; height: 2px;
      background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.1), transparent);
    }
    .premium-stat-card:hover {
      transform: translateY(-2px);
      border-color: rgba(255, 255, 255, 0.12);
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
    }
    .premium-stat-value {
      font-size: 26px;
      font-weight: 800;
      font-family: var(--font-mono, monospace);
      color: var(--text-primary);
      margin-top: 4px;
    }
    .premium-stat-value.danger {
      color: #ff5f56;
      text-shadow: 0 0 10px rgba(255, 95, 86, 0.2);
    }
    .premium-stat-value.success {
      color: #4af626;
      text-shadow: 0 0 10px rgba(74, 246, 38, 0.2);
    }

    .kbd-chip {
      display: inline-block;
      background: linear-gradient(180deg, #373a40 0%, #212327 100%);
      border: 1px solid #4f525c;
      border-bottom: 3px solid #151619;
      border-radius: 6px;
      padding: 4px 10px;
      font-family: var(--font-mono, monospace);
      font-size: 11px;
      font-weight: 700;
      color: #e2e8f0;
      box-shadow: 0 2px 4px rgba(0, 0, 0, 0.4);
      text-shadow: 0 1px 0 #000;
      transition: all 0.1s ease;
      letter-spacing: 0.5px;
      margin: 2px;
    }
    .kbd-chip-modifier {
      border-color: #00bcff;
      color: #00bcff;
    }

    .premium-table {
      width: 100%;
      border-collapse: collapse;
      text-align: left;
    }
    .premium-table th {
      background: rgba(0, 0, 0, 0.25);
      border-bottom: 1.5px solid rgba(255, 255, 255, 0.08);
      font-size: 10px;
      font-weight: 700;
      color: var(--text-muted);
      letter-spacing: 0.8px;
      text-transform: uppercase;
      padding: 12px 16px;
    }
    .premium-table tr {
      border-bottom: 1px solid rgba(255, 255, 255, 0.04);
      transition: all 0.25s ease;
    }
    .premium-table tbody tr:hover {
      background: rgba(255, 255, 255, 0.015) !important;
    }
    .premium-table td {
      padding: 14px 16px;
      vertical-align: middle;
    }

    .search-wrapper {
      position: relative;
      display: flex;
      align-items: center;
      width: 100%;
      max-width: 320px;
    }
    .search-icon {
      position: absolute;
      left: 12px;
      color: var(--text-muted);
      font-size: 13px;
      pointer-events: none;
    }
    .premium-search-input {
      width: 100%;
      padding: 8px 12px 8px 34px;
      font-size: 12px;
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid rgba(255, 255, 255, 0.08);
      color: var(--text-primary);
      border-radius: 20px;
      outline: none;
      transition: all 0.3s ease;
    }
    .premium-search-input:focus {
      background: rgba(255, 255, 255, 0.05);
      border-color: var(--accent);
      box-shadow: 0 0 10px rgba(0, 188, 255, 0.15);
    }
  </style>
`;

export function formatKeyboardBadge(keysStr: string): string {
  if (!keysStr) return '';
  const tokens = keysStr.split(',').map(t => t.trim()).filter(Boolean);

  return tokens.map(token => {
    let clean = token;
    let isModifier = false;

    if (token.startsWith('{') && token.endsWith('}')) {
      clean = token.substring(1, token.length - 1).trim();
      isModifier = true;
    }

    if (clean.startsWith('ModifierKey.')) {
      clean = clean.substring('ModifierKey.'.length);
      isModifier = true;
    } else if (clean.startsWith('Key.')) {
      clean = clean.substring('Key.'.length);
    }

    const modifierClass = isModifier ? 'kbd-chip-modifier' : '';
    return `<span class="kbd-chip ${modifierClass}">${escapeHtml(clean)}</span>`;
  }).join('');
}

export function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}
export { escapeHtml as escape };
