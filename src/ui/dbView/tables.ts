import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';
import type { ModInfo, AppSettings } from './types';

export function formatSettingValue(key: string, val: unknown): string {
  if (val === null || val === undefined) return 'null';
  if (typeof val === 'boolean' || typeof val === 'number') return String(val);
  if (typeof val === 'string') {
    const lower = key.toLowerCase();
    if (lower.includes('token') || lower.includes('secret') || lower === 'apikey') {
      return '•••••••••••••••• (Encrypted)';
    }
    return val;
  }
  if (Array.isArray(val)) {
    if (val.length === 0) return '[]';
    return `[ ${val.length} item${val.length === 1 ? '' : 's'} ]`;
  }
  if (typeof val === 'object') {
    const acc = val as any;
    if (acc.name || acc.username) {
      return `{ user: "${acc.name || acc.username}", id: ${acc.userId || acc.user_id || '?'}, ... }`;
    }
    const keys = Object.keys(val as object);
    if (keys.length === 0) return '{}';
    return `{ ${keys.length} field${keys.length === 1 ? '' : 's'} }`;
  }
  return String(val);
}

export function maskSensitiveData(data: unknown): unknown {
  if (!data || typeof data !== 'object') return data;
  const clone = JSON.parse(JSON.stringify(data));

  function recurse(o: any) {
    if (!o || typeof o !== 'object') return;
    for (const k of Object.keys(o)) {
      const lower = k.toLowerCase();
      if (typeof o[k] === 'string' && (lower.includes('token') || lower.includes('secret') || lower === 'apikey')) {
        const val = o[k];
        if (val.length > 10) {
          o[k] = `[ENCRYPTED: ${val.slice(0, 4)}••••••••${val.slice(-4)}]`;
        } else {
          o[k] = '••••••••••••••••';
        }
      } else if (typeof o[k] === 'object') {
        recurse(o[k]);
      }
    }
  }

  recurse(clone);
  return clone;
}

export function renderModsTable(mods: ModInfo[]): string {
  if (!mods.length) return `<div class="db-empty">${escapeHtml(t('db.empty_mods'))}</div>`;

  const typeColor: Record<string, string> = {
    ue4ss: 'var(--type-ue4ss)',
    palschema: 'var(--type-palschema)',
    pak: 'var(--type-pak)',
    logicmods: 'var(--type-logicmods)',
    hybrid: 'var(--type-hybrid)',
  };

  const rows = mods.map(m => {
    const typeLabel = m.type.toLowerCase() === 'hybrid' ? t('card.type_hybrid') : m.type.toUpperCase();
    return `
    <tr class="db-row" data-id="${escapeHtml(m.id)}" title="${escapeHtml(m.id)}">
      <td class="db-cell db-cell-name">${escapeHtml(m.name)}</td>
      <td class="db-cell"><span class="db-type-badge" style="color:${typeColor[m.type] ?? 'var(--text-muted)'}">${escapeHtml(typeLabel)}</span></td>
      <td class="db-cell"><span class="db-status-dot ${m.enabled ? 'on' : 'off'}"></span></td>
      <td class="db-cell db-cell-mono">${escapeHtml(m.version)}</td>
      <td class="db-cell db-cell-date">${escapeHtml(m.installDate?.split('T')[0] ?? '')}</td>
    </tr>
  `;
  }).join('');

  return `
    <table class="db-grid-table">
      <thead>
        <tr>
          <th>${escapeHtml(t('card.table_col_name'))}</th>
          <th>${escapeHtml(t('card.table_col_type'))}</th>
          <th>${escapeHtml(t('common.on'))}</th>
          <th>${escapeHtml(t('card.table_col_version'))}</th>
          <th>${escapeHtml(t('detail.installed_label'))}</th>
        </tr>
      </thead>
      <tbody>${rows}</tbody>
    </table>
  `;
}

export function renderProfilesTable(profiles: any[], currentId: string): string {
  if (!profiles.length) return `<div class="db-empty">${escapeHtml(t('db.empty_profiles'))}</div>`;

  const rows = profiles.map(p => {
    const installed = p.installed_mod_ids ?? p.installedModIds ?? [];
    const enabled = p.enabled_mod_ids ?? p.enabledModIds ?? [];
    const created = p.created_at ?? p.createdAt ?? '';

    return `
    <tr class="db-row ${p.id === currentId ? 'db-row-active' : ''}" data-id="${escapeHtml(p.id)}" title="${escapeHtml(p.id)}">
      <td class="db-cell db-cell-name">
        ${escapeHtml(p.name)}
        ${p.id === currentId ? `<span class="db-active-badge">${escapeHtml(t('profiles.active_badge'))}</span>` : ''}
      </td>
      <td class="db-cell db-cell-mono">${installed.length} ${escapeHtml(t('detail.installed_label')).toLowerCase()}</td>
      <td class="db-cell db-cell-mono">${enabled.length} ${escapeHtml(t('common.enabled')).toLowerCase()}</td>
      <td class="db-cell db-cell-date">${escapeHtml(created.split('T')[0] ?? '')}</td>
    </tr>
  `;
  }).join('');

  return `
    <table class="db-grid-table">
      <thead>
        <tr>
          <th>${escapeHtml(t('db.col_profile_name'))}</th>
          <th>${escapeHtml(t('detail.installed_label'))}</th>
          <th>${escapeHtml(t('common.enabled'))}</th>
          <th>${escapeHtml(t('db.col_created'))}</th>
        </tr>
      </thead>
      <tbody>${rows}</tbody>
    </table>
  `;
}

export function renderSettingsTable(settings: AppSettings): string {
  const rows = Object.entries(settings).map(([key, val]) => `
    <tr class="db-row db-row-settings" title="${escapeHtml(t('db.click_to_edit'))}">
      <td class="db-cell db-cell-key">${escapeHtml(key)}</td>
      <td class="db-cell db-cell-mono db-cell-val">${escapeHtml(formatSettingValue(key, val))}</td>
    </tr>
  `).join('');

  return `
    <div style="padding: 8px 12px; font-size: 11px; color: var(--text-muted);">${escapeHtml(t('db.settings_hint'))}</div>
    <table class="db-grid-table">
      <thead><tr><th>${escapeHtml(t('db.col_key'))}</th><th>${escapeHtml(t('db.col_val'))}</th></tr></thead>
      <tbody>${rows}</tbody>
    </table>
  `;
}
