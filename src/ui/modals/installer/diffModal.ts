import { getState } from '../../../state';
import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import { setModIgnoredKeys } from '../../../api';

export function showConfigDiffModal(diffs: any[], modId: string): void {
  const overlay = document.createElement('div');
  overlay.className = 'modal-overlay visible';
  overlay.id = 'config-diff-modal';
  overlay.style.zIndex = '4500';

  const state = getState();
  const currentMod = state.allMods.find(m => m.id === modId);
  const currentIgnoredKeys = currentMod?.ignoredKeys || [];

  let html = `
    <div class="modal" style="max-width:850px; width:100%; max-height:85vh; display:flex; flex-direction:column; background:var(--bg-secondary); border:1px solid var(--border); border-radius:8px; box-shadow:0 12px 36px rgba(0,0,0,0.5);">
      <div class="modal-header" style="padding:16px 20px; border-bottom:1px solid var(--border); display:flex; align-items:center; justify-content:space-between;">
        <h3 style="margin:0; font-size:16px; font-weight:700; color:var(--text-primary);">⚙ ${escapeHtml(t('installer.diff_modal_title') || 'Config Settings Merge Preview')}</h3>
        <button class="modal-close-btn" id="config-diff-modal-close-x" style="background:none; border:none; color:var(--text-muted); cursor:pointer; font-size:16px;">✕</button>
      </div>
      <div class="modal-body" style="flex:1; overflow-y:auto; padding:20px; display:flex; flex-direction:column; gap:16px; background:var(--bg-primary);">
  `;

  const collapseByDefault = diffs.length > 1;

  for (let i = 0; i < diffs.length; i++) {
    const diff = diffs[i];
    const isFileIgnored = currentIgnoredKeys.includes(diff.file_name);
    html += `
      <div class="config-diff-card" id="config-diff-card-${i}" style="background:var(--bg-secondary); border:1px solid ${isFileIgnored ? 'rgba(255,80,0,0.3)' : 'var(--border)'}; border-radius:6px; padding:12px; display:flex; flex-direction:column; gap:4px;">
        <div class="config-diff-file-header" data-index="${i}" style="cursor:pointer; font-weight:700; font-family:monospace; font-size:12px; color:var(--text-primary); display:flex; align-items:center; justify-content:space-between; padding:2px 0; user-select:none; word-break:break-all;">
          <div style="display:flex; align-items:center; gap:8px;">
            <span>📄 ${escapeHtml(diff.file_name)}</span>
            <span id="diff-file-badge-${i}" style="font-size:9px; padding:1px 6px; border-radius:8px; font-weight:600; text-transform:uppercase; ${isFileIgnored ? 'background:rgba(255,80,0,0.15); color:#ff5000; border:1px solid rgba(255,80,0,0.3);' : 'background:rgba(0,188,255,0.15); color:var(--accent); border:1px solid rgba(0,188,255,0.3);'}">
              ${isFileIgnored ? escapeHtml(t('installer.diff_file_ignored_badge')) : escapeHtml(t('installer.diff_merge_file'))}
            </span>
          </div>
          <div style="display:flex; align-items:center; gap:8px;">
            <button type="button" class="btn ignore-file-btn" data-index="${i}" data-file="${escapeHtml(diff.file_name)}" style="font-size:10px; padding:3px 8px; height:auto; line-height:1; margin:0; border:1px solid ${isFileIgnored ? 'rgba(255,80,0,0.5)' : 'rgba(0,188,255,0.4)'}; background:${isFileIgnored ? 'rgba(255,80,0,0.1)' : 'rgba(0,188,255,0.05)'}; color:${isFileIgnored ? '#ff5000' : 'var(--text-primary)'}; border-radius:4px; cursor:pointer;">
              ${isFileIgnored ? `<span>✕</span> ${escapeHtml(t('installer.diff_use_clean_file'))}` : `<span>✓</span> ${escapeHtml(t('installer.diff_merge_file'))}`}
            </button>
            <span class="toggle-icon" style="font-size:10px; color:var(--text-muted); padding-left:4px;">${collapseByDefault ? '▲' : '▼'}</span>
          </div>
        </div>
        <div class="config-diff-file-content" id="config-diff-file-content-${i}" style="display: ${collapseByDefault ? 'none' : 'flex'}; flex-direction:column; gap:12px; margin-top:8px; border-top:1px solid rgba(255,255,255,0.03); padding-top:8px; opacity:${isFileIgnored ? '0.4' : '1'};">
    `;

    if (diff.keys_user_changed && diff.keys_user_changed.length > 0) {
      html += `
        <div>
          <div style="color:#ff9000; font-size:11px; font-weight:700; margin-bottom:6px; display:flex; align-items:center; gap:6px;">
            <span>🟢 ${escapeHtml(t('installer.diff_user_changes_title') || 'Your changes to preserve')}</span>
            <span style="font-size:9px; padding:1px 5px; border-radius:10px; background:rgba(255,144,0,0.15); border:1px solid rgba(255,144,0,0.25);">${diff.keys_user_changed.length}</span>
          </div>
          <div style="overflow-x:auto; background:var(--bg-primary); border:1px solid var(--border); border-radius:4px; padding:4px;">
            <table style="width:100%; border-collapse:collapse; font-size:10px; text-align:left; font-family:monospace;">
              <thead>
                <tr style="border-bottom:1px solid var(--border); color:var(--text-muted);">
                  <th style="padding:6px 8px; font-weight:bold; width: 60px;">${escapeHtml(t('installer.diff_col_preserve'))}</th>
                  <th style="padding:6px 8px; font-weight:bold;">${escapeHtml(t('installer.diff_col_setting'))}</th>
                  <th style="padding:6px 8px; font-weight:bold; width:150px; text-align:right;">${escapeHtml(t('installer.diff_col_your_value'))}</th>
                  <th style="padding:6px 8px; font-weight:bold; width:150px; text-align:right;">${escapeHtml(t('installer.diff_col_default_value'))}</th>
                </tr>
              </thead>
              <tbody>
                ${diff.keys_user_changed.map((c: any) => {
                  const isPreserved = !currentIgnoredKeys.includes(c.key);
                  return `
                    <tr style="border-bottom:1px solid rgba(255,255,255,0.02);">
                      <td style="padding:6px 8px; text-align:center;">
                        <input type="checkbox" class="preserve-key-switch" data-key="${escapeHtml(c.key)}" ${isPreserved ? 'checked' : ''} style="cursor:pointer;" />
                      </td>
                      <td style="padding:6px 8px; color:var(--text-primary); word-break:break-all;" title="${escapeHtml(c.key)}">${escapeHtml(c.key)}</td>
                      <td style="padding:6px 8px; color:#ff9000; font-weight:bold; text-align:right; word-break:break-all;">${escapeHtml(c.old_value)}</td>
                      <td style="padding:6px 8px; opacity:0.6; text-decoration:line-through; text-align:right; word-break:break-all;">${escapeHtml(c.new_value)}</td>
                    </tr>
                  `;
                }).join('')}
              </tbody>
            </table>
          </div>
        </div>
      `;
    }

    if (diff.keys_added_by_author && diff.keys_added_by_author.length > 0) {
      html += `
        <div>
          <div style="color:#00bcff; font-size:11px; font-weight:700; margin-bottom:6px; display:flex; align-items:center; gap:6px;">
            <span>🔵 ${escapeHtml(t('installer.diff_new_settings_title'))}</span>
            <span style="font-size:9px; padding:1px 5px; border-radius:10px; background:rgba(0,188,255,0.15); border:1px solid rgba(0,188,255,0.25);">${diff.keys_added_by_author.length}</span>
          </div>
          <div style="display:flex; flex-wrap:wrap; gap:6px; background:var(--bg-primary); border:1px solid var(--border); border-radius:4px; padding:10px;">
            ${diff.keys_added_by_author.map((k: string) => `
              <span style="font-family:monospace; font-size:9px; padding:2px 6px; background:rgba(0,188,255,0.08); border:1px solid rgba(0,188,255,0.15); border-radius:4px; color:#00bcff; word-break:break-all;" title="${escapeHtml(k)}">${escapeHtml(k)}</span>
            `).join('')}
          </div>
        </div>
      `;
    }

    if (diff.keys_removed_by_author && diff.keys_removed_by_author.length > 0) {
      html += `
        <div>
          <div style="color:#ff5000; font-size:11px; font-weight:700; margin-bottom:6px; display:flex; align-items:center; gap:6px;">
            <span>🔴 ${escapeHtml(t('installer.diff_author_removed_title'))}</span>
            <span style="font-size:9px; padding:1px 5px; border-radius:10px; background:rgba(255,80,0,0.15); border:1px solid rgba(255,80,0,0.25);">${diff.keys_removed_by_author.length}</span>
          </div>
          <div style="display:flex; flex-wrap:wrap; gap:6px; background:var(--bg-primary); border:1px solid var(--border); border-radius:4px; padding:10px;">
            ${diff.keys_removed_by_author.map((k: string) => `
              <span style="font-family:monospace; font-size:9px; padding:2px 6px; background:rgba(255,80,0,0.08); border:1px solid rgba(255,80,0,0.15); border-radius:4px; color:#ff5000; word-break:break-all;" title="${escapeHtml(k)}">${escapeHtml(k)}</span>
            `).join('')}
          </div>
        </div>
      `;
    }

    html += `
        </div>
      </div>
    `;
  }

  html += `
      </div>
      <div class="modal-footer" style="padding:14px 20px; border-top:1px solid var(--border); display:flex; justify-content:flex-end; background:var(--bg-secondary); border-bottom-left-radius:8px; border-bottom-right-radius:8px;">
        <button id="config-diff-modal-close-btn" class="btn btn-secondary">${escapeHtml(t('common.close'))}</button>
      </div>
    </div>
  `;

  overlay.innerHTML = html;
  document.body.appendChild(overlay);

  let localIgnoredKeys = [...currentIgnoredKeys];
  overlay.querySelectorAll('.preserve-key-switch').forEach(checkbox => {
    checkbox.addEventListener('change', (e) => {
      const target = e.target as HTMLInputElement;
      const key = target.dataset.key!;
      if (target.checked) {
        localIgnoredKeys = localIgnoredKeys.filter(k => k !== key);
      } else {
        if (!localIgnoredKeys.includes(key)) {
          localIgnoredKeys.push(key);
        }
      }

      setModIgnoredKeys(modId, localIgnoredKeys).then(() => {
        const modInState = state.allMods.find(m => m.id === modId);
        if (modInState) {
          modInState.ignoredKeys = localIgnoredKeys;
        }
      }).catch(err => {
        console.error("Failed to update ignored keys:", err);
      });
    });
  });

  overlay.querySelectorAll('.ignore-file-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const fileName = (btn as HTMLElement).dataset.file!;
      const idx = (btn as HTMLElement).dataset.index!;
      const isCurrentlyIgnored = localIgnoredKeys.includes(fileName);

      if (isCurrentlyIgnored) {
        localIgnoredKeys = localIgnoredKeys.filter(k => k !== fileName);
      } else {
        localIgnoredKeys.push(fileName);
      }

      const isNowIgnored = !isCurrentlyIgnored;
      const card = overlay.querySelector(`#config-diff-card-${idx}`) as HTMLElement;
      const badge = overlay.querySelector(`#diff-file-badge-${idx}`) as HTMLElement;
      const content = overlay.querySelector(`#config-diff-file-content-${idx}`) as HTMLElement;

      if (card) {
        card.style.borderColor = isNowIgnored ? 'rgba(255,80,0,0.3)' : 'var(--border)';
      }
      if (badge) {
        badge.style.background = isNowIgnored ? 'rgba(255,80,0,0.15)' : 'rgba(0,188,255,0.15)';
        badge.style.color = isNowIgnored ? '#ff5000' : 'var(--accent)';
        badge.style.border = isNowIgnored ? '1px solid rgba(255,80,0,0.3)' : '1px solid rgba(0,188,255,0.3)';
        badge.textContent = isNowIgnored ? t('installer.diff_file_ignored_badge') : t('installer.diff_merge_file');
      }
      if (content) {
        content.style.opacity = isNowIgnored ? '0.4' : '1';
      }

      const targetBtn = btn as HTMLElement;
      targetBtn.style.borderColor = isNowIgnored ? 'rgba(255,80,0,0.5)' : 'rgba(0,188,255,0.4)';
      targetBtn.style.background = isNowIgnored ? 'rgba(255,80,0,0.1)' : 'rgba(0,188,255,0.05)';
      targetBtn.style.color = isNowIgnored ? '#ff5000' : 'var(--text-primary)';
      targetBtn.innerHTML = isNowIgnored
        ? `<span>✕</span> ${escapeHtml(t('installer.diff_use_clean_file'))}`
        : `<span>✓</span> ${escapeHtml(t('installer.diff_merge_file'))}`;

      setModIgnoredKeys(modId, localIgnoredKeys).then(() => {
        const modInState = state.allMods.find(m => m.id === modId);
        if (modInState) {
          modInState.ignoredKeys = localIgnoredKeys;
        }
      }).catch(err => {
        console.error("Failed to update ignored keys:", err);
      });
    });
  });

  overlay.querySelectorAll('.config-diff-file-header').forEach(header => {
    header.addEventListener('click', () => {
      const idx = (header as HTMLElement).dataset.index;
      const content = overlay.querySelector(`#config-diff-file-content-${idx}`) as HTMLElement;
      const icon = header.querySelector('.toggle-icon') as HTMLElement;
      if (content && icon) {
        if (content.style.display === 'none') {
          content.style.display = 'flex';
          icon.textContent = '▼';
        } else {
          content.style.display = 'none';
          icon.textContent = '▲';
        }
      }
    });
  });

  const close = () => {
    overlay.classList.remove('visible');
    setTimeout(() => overlay.remove(), 200);
  };

  overlay.querySelector('#config-diff-modal-close-x')!.addEventListener('click', close);
  overlay.querySelector('#config-diff-modal-close-btn')!.addEventListener('click', close);
}

export function showArchivedConfigDiffModal(
  diffs: any[],
  initialIgnoredFiles: string[],
  initialIgnoredKeys: string[],
  onUpdate: (ignoredFiles: string[], ignoredKeys: string[]) => void
): void {
  const overlay = document.createElement('div');
  overlay.className = 'modal-overlay visible';
  overlay.id = 'archived-config-diff-modal';
  overlay.style.zIndex = '4500';

  let localIgnoredFiles = [...initialIgnoredFiles];
  let localIgnoredKeys = [...initialIgnoredKeys];

  let html = `
    <div class="modal" style="max-width:850px; width:100%; max-height:85vh; display:flex; flex-direction:column; background:var(--bg-secondary); border:1px solid var(--border); border-radius:8px; box-shadow:0 12px 36px rgba(0,0,0,0.5);">
      <div class="modal-header" style="padding:16px 20px; border-bottom:1px solid var(--border); display:flex; align-items:center; justify-content:space-between;">
        <div style="display:flex; align-items:center; gap:8px;">
          <span style="font-size:18px;">💾</span>
          <h3 style="margin:0; font-size:16px; font-weight:700; color:var(--text-primary);">${escapeHtml(t('installer.archived_diff_modal_title') || 'Archived Configs Merge Preview')}</h3>
        </div>
        <button class="modal-close-btn" id="archived-diff-modal-close-x" style="background:none; border:none; color:var(--text-muted); cursor:pointer; font-size:16px;">✕</button>
      </div>
      <div class="modal-body" style="flex:1; overflow-y:auto; padding:20px; display:flex; flex-direction:column; gap:16px; background:var(--bg-primary);">
  `;

  const collapseByDefault = diffs.length > 1;

  for (let i = 0; i < diffs.length; i++) {
    const diff = diffs[i];
    const isFileIgnored = localIgnoredFiles.includes(diff.file_name);
    html += `
      <div class="config-diff-card" id="archived-diff-card-${i}" style="background:var(--bg-secondary); border:1px solid ${isFileIgnored ? 'rgba(255,80,0,0.3)' : 'rgba(46,204,113,0.3)'}; border-radius:6px; padding:12px; display:flex; flex-direction:column; gap:4px;">
        <div class="config-diff-file-header" data-index="${i}" style="cursor:pointer; font-weight:700; font-family:monospace; font-size:12px; color:var(--text-primary); display:flex; align-items:center; justify-content:space-between; padding:2px 0; user-select:none; word-break:break-all;">
          <div style="display:flex; align-items:center; gap:8px;">
            <span>📄 ${escapeHtml(diff.file_name)}</span>
            <span id="archived-diff-file-badge-${i}" style="font-size:9px; padding:1px 6px; border-radius:8px; font-weight:600; text-transform:uppercase; ${isFileIgnored ? 'background:rgba(255,80,0,0.15); color:#ff5000; border:1px solid rgba(255,80,0,0.3);' : 'background:rgba(46,204,113,0.15); color:#2ecc71; border:1px solid rgba(46,204,113,0.3);'}">
              ${isFileIgnored ? escapeHtml(t('installer.archived_file_skipped_badge')) : escapeHtml(t('installer.archived_file_restore_badge'))}
            </span>
          </div>
          <div style="display:flex; align-items:center; gap:8px;">
            <button type="button" class="btn ignore-archived-file-btn" data-index="${i}" data-file="${escapeHtml(diff.file_name)}" style="font-size:10px; padding:3px 8px; height:auto; line-height:1; margin:0; border:1px solid ${isFileIgnored ? 'rgba(255,80,0,0.5)' : 'rgba(46,204,113,0.4)'}; background:${isFileIgnored ? 'rgba(255,80,0,0.1)' : 'rgba(46,204,113,0.08)'}; color:${isFileIgnored ? '#ff5000' : '#2ecc71'}; border-radius:4px; cursor:pointer;">
              ${isFileIgnored ? `<span>✕</span> ${escapeHtml(t('installer.archived_file_skip'))}` : `<span>✓</span> ${escapeHtml(t('installer.archived_file_restore'))}`}
            </button>
            <span class="toggle-icon" style="font-size:10px; color:var(--text-muted); padding-left:4px;">${collapseByDefault ? '▲' : '▼'}</span>
          </div>
        </div>
        <div class="config-diff-file-content" id="archived-diff-file-content-${i}" style="display: ${collapseByDefault ? 'none' : 'flex'}; flex-direction:column; gap:12px; margin-top:8px; border-top:1px solid rgba(255,255,255,0.03); padding-top:8px; opacity:${isFileIgnored ? '0.4' : '1'};">
    `;

    if (diff.keys_user_changed && diff.keys_user_changed.length > 0) {
      html += `
        <div>
          <div style="color:#2ecc71; font-size:11px; font-weight:700; margin-bottom:6px; display:flex; align-items:center; gap:6px;">
            <span>🟢 ${escapeHtml(t('installer.archived_custom_keys_title'))}</span>
            <span style="font-size:9px; padding:1px 5px; border-radius:10px; background:rgba(46,204,113,0.15); border:1px solid rgba(46,204,113,0.25);">${diff.keys_user_changed.length}</span>
          </div>
          <div style="overflow-x:auto; background:var(--bg-primary); border:1px solid var(--border); border-radius:4px; padding:4px;">
            <table style="width:100%; border-collapse:collapse; font-size:10px; text-align:left; font-family:monospace;">
              <thead>
                <tr style="border-bottom:1px solid var(--border); color:var(--text-muted);">
                  <th style="padding:6px 8px; font-weight:bold; width: 60px;">${escapeHtml(t('installer.diff_col_restore'))}</th>
                  <th style="padding:6px 8px; font-weight:bold;">${escapeHtml(t('installer.diff_col_setting'))}</th>
                  <th style="padding:6px 8px; font-weight:bold; width:150px; text-align:right;">${escapeHtml(t('installer.diff_col_archived_value'))}</th>
                  <th style="padding:6px 8px; font-weight:bold; width:150px; text-align:right;">${escapeHtml(t('installer.diff_col_default_mod_value'))}</th>
                </tr>
              </thead>
              <tbody>
                ${diff.keys_user_changed.map((c: any) => {
                  const isPreserved = !localIgnoredKeys.includes(c.key);
                  return `
                    <tr style="border-bottom:1px solid rgba(255,255,255,0.02);">
                      <td style="padding:6px 8px; text-align:center;">
                        <input type="checkbox" class="preserve-archived-key-switch" data-key="${escapeHtml(c.key)}" ${isPreserved ? 'checked' : ''} style="cursor:pointer; accent-color:#2ecc71;" />
                      </td>
                      <td style="padding:6px 8px; color:var(--text-primary); word-break:break-all;" title="${escapeHtml(c.key)}">${escapeHtml(c.key)}</td>
                      <td style="padding:6px 8px; color:#2ecc71; font-weight:bold; text-align:right; word-break:break-all;">${escapeHtml(c.old_value)}</td>
                      <td style="padding:6px 8px; opacity:0.6; text-decoration:line-through; text-align:right; word-break:break-all;">${escapeHtml(c.new_value)}</td>
                    </tr>
                  `;
                }).join('')}
              </tbody>
            </table>
          </div>
        </div>
      `;
    } else {
      html += `
        <div style="font-size:11px; color:var(--text-muted); font-style:italic;">
          ${escapeHtml(t('installer.archived_file_matches_or_custom'))}
        </div>
      `;
    }

    if (diff.keys_added_by_author && diff.keys_added_by_author.length > 0) {
      html += `
        <div>
          <div style="color:#00bcff; font-size:11px; font-weight:700; margin-bottom:6px; display:flex; align-items:center; gap:6px;">
            <span>🔵 ${escapeHtml(t('installer.archived_new_keys_title'))}</span>
            <span style="font-size:9px; padding:1px 5px; border-radius:10px; background:rgba(0,188,255,0.15); border:1px solid rgba(0,188,255,0.25);">${diff.keys_added_by_author.length}</span>
          </div>
          <div style="display:flex; flex-wrap:wrap; gap:6px; background:var(--bg-primary); border:1px solid var(--border); border-radius:4px; padding:10px;">
            ${diff.keys_added_by_author.map((k: string) => `
              <span style="font-family:monospace; font-size:9px; padding:2px 6px; background:rgba(0,188,255,0.08); border:1px solid rgba(0,188,255,0.15); border-radius:4px; color:#00bcff; word-break:break-all;" title="${escapeHtml(k)}">${escapeHtml(k)}</span>
            `).join('')}
          </div>
        </div>
      `;
    }

    if (diff.keys_removed_by_author && diff.keys_removed_by_author.length > 0) {
      html += `
        <div>
          <div style="color:#ff5000; font-size:11px; font-weight:700; margin-bottom:6px; display:flex; align-items:center; gap:6px;">
            <span>🔴 ${escapeHtml(t('installer.diff_author_removed_title'))}</span>
            <span style="font-size:9px; padding:1px 5px; border-radius:10px; background:rgba(255,80,0,0.15); border:1px solid rgba(255,80,0,0.25);">${diff.keys_removed_by_author.length}</span>
          </div>
          <div style="display:flex; flex-wrap:wrap; gap:6px; background:var(--bg-primary); border:1px solid var(--border); border-radius:4px; padding:10px;">
            ${diff.keys_removed_by_author.map((k: string) => `
              <span style="font-family:monospace; font-size:9px; padding:2px 6px; background:rgba(255,80,0,0.08); border:1px solid rgba(255,80,0,0.15); border-radius:4px; color:#ff5000; word-break:break-all;" title="${escapeHtml(k)}">${escapeHtml(k)}</span>
            `).join('')}
          </div>
        </div>
      `;
    }

    html += `
        </div>
      </div>
    `;
  }

  html += `
      </div>
      <div class="modal-footer" style="padding:14px 20px; border-top:1px solid var(--border); display:flex; justify-content:flex-end; background:var(--bg-secondary); border-bottom-left-radius:8px; border-bottom-right-radius:8px;">
        <button id="archived-diff-modal-close-btn" class="btn btn-primary" style="background:#2ecc71; border:none; color:#fff;">${escapeHtml(t('common.done'))}</button>
      </div>
    </div>
  `;

  overlay.innerHTML = html;
  document.body.appendChild(overlay);

  overlay.querySelectorAll('.preserve-archived-key-switch').forEach(checkbox => {
    checkbox.addEventListener('change', (e) => {
      const target = e.target as HTMLInputElement;
      const key = target.dataset.key!;
      if (target.checked) {
        localIgnoredKeys = localIgnoredKeys.filter(k => k !== key);
      } else {
        if (!localIgnoredKeys.includes(key)) {
          localIgnoredKeys.push(key);
        }
      }
      onUpdate(localIgnoredFiles, localIgnoredKeys);
    });
  });

  overlay.querySelectorAll('.ignore-archived-file-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const fileName = (btn as HTMLElement).dataset.file!;
      const idx = (btn as HTMLElement).dataset.index!;
      const isCurrentlyIgnored = localIgnoredFiles.includes(fileName);

      if (isCurrentlyIgnored) {
        localIgnoredFiles = localIgnoredFiles.filter(f => f !== fileName);
      } else {
        localIgnoredFiles.push(fileName);
      }

      const isNowIgnored = !isCurrentlyIgnored;
      const card = overlay.querySelector(`#archived-diff-card-${idx}`) as HTMLElement;
      const badge = overlay.querySelector(`#archived-diff-file-badge-${idx}`) as HTMLElement;
      const content = overlay.querySelector(`#archived-diff-file-content-${idx}`) as HTMLElement;

      if (card) {
        card.style.borderColor = isNowIgnored ? 'rgba(255,80,0,0.3)' : 'rgba(46,204,113,0.3)';
      }
      if (badge) {
        badge.style.background = isNowIgnored ? 'rgba(255,80,0,0.15)' : 'rgba(46,204,113,0.15)';
        badge.style.color = isNowIgnored ? '#ff5000' : '#2ecc71';
        badge.style.border = isNowIgnored ? '1px solid rgba(255,80,0,0.3)' : '1px solid rgba(46,204,113,0.3)';
        badge.textContent = isNowIgnored ? t('installer.archived_file_skipped_badge') : t('installer.archived_file_restore_badge');
      }
      if (content) {
        content.style.opacity = isNowIgnored ? '0.4' : '1';
      }

      const targetBtn = btn as HTMLElement;
      targetBtn.style.borderColor = isNowIgnored ? 'rgba(255,80,0,0.5)' : 'rgba(46,204,113,0.4)';
      targetBtn.style.background = isNowIgnored ? 'rgba(255,80,0,0.1)' : 'rgba(46,204,113,0.08)';
      targetBtn.style.color = isNowIgnored ? '#ff5000' : '#2ecc71';
      targetBtn.innerHTML = isNowIgnored
        ? `<span>✕</span> ${escapeHtml(t('installer.archived_file_skip'))}`
        : `<span>✓</span> ${escapeHtml(t('installer.archived_file_restore'))}`;

      onUpdate(localIgnoredFiles, localIgnoredKeys);
    });
  });

  overlay.querySelectorAll('.config-diff-file-header').forEach(header => {
    header.addEventListener('click', () => {
      const idx = (header as HTMLElement).dataset.index;
      const content = overlay.querySelector(`#archived-diff-file-content-${idx}`) as HTMLElement;
      const icon = header.querySelector('.toggle-icon') as HTMLElement;
      if (content && icon) {
        if (content.style.display === 'none') {
          content.style.display = 'flex';
          icon.textContent = '▼';
        } else {
          content.style.display = 'none';
          icon.textContent = '▲';
        }
      }
    });
  });

  const close = () => {
    overlay.classList.remove('visible');
    setTimeout(() => overlay.remove(), 200);
  };

  overlay.querySelector('#archived-diff-modal-close-x')!.addEventListener('click', close);
  overlay.querySelector('#archived-diff-modal-close-btn')!.addEventListener('click', close);
}
