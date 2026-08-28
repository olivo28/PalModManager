import { setModVersion, checkGitHubVersion, setGithubVersion, ignoreModVersion } from '../../api';
import { getState, updateState } from '../../state';
import { loadMods, renderModsView } from '../modsView';
import { showToast } from '../toast';
import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';
import type { ModInfo } from '../../types';

let detailTabsSetup = false;
export function setupDetailTabs(): void {
  if (detailTabsSetup) return;
  detailTabsSetup = true;
  document.querySelectorAll('.detail-tab').forEach((tab) => {
    tab.addEventListener('click', () => {
      const tabName = (tab as HTMLElement).dataset.tab!;
      document.querySelectorAll('.detail-tab').forEach((t) => t.classList.remove('active'));
      tab.classList.add('active');
      document.getElementById('detail-info-tab')!.style.display = tabName === 'info' ? '' : 'none';
      document.getElementById('detail-tech-tab')!.style.display = tabName === 'tech' ? '' : 'none';
    });
  });
}

export function renderVersion(mod: ModInfo): void {
  const el = document.getElementById('detail-version')!;
  const state = getState();
  const updateVer = state.availableUpdates?.get(mod.id);

  let updateBadge = '';
  if (updateVer) {
    updateBadge = `
      <span class="mod-card-update-badge" title="${escapeHtml(t('card.badge_update_available', { version: updateVer }))}" style="margin-left: 6px; vertical-align: middle;">&#9650; ${escapeHtml(t('context.update_mod'))} (v${escapeHtml(updateVer)})</span>
      <button class="btn-tiny ignore-update-btn" data-latest="${escapeHtml(updateVer)}" style="margin-left: 6px; vertical-align: middle; background: rgba(255,255,255,0.05); color: var(--text-muted); border: 1px solid var(--border);">${escapeHtml(t('common.cancel'))}</button>
    `;
  }

  const ignoredLabel = mod.ignoredVersion
    ? `<span style="font-size: 10px; color: var(--text-muted); margin-left: 6px; vertical-align: middle;">(${escapeHtml(t('context.ignore_update', { version: mod.ignoredVersion }))}) <button class="btn-tiny unignore-update-btn" style="margin-left: 4px; vertical-align: middle; background: transparent; border: none; color: var(--accent); cursor: pointer; text-decoration: underline; padding: 0;">${escapeHtml(t('common.retry'))}</button></span>`
    : '';

  el.innerHTML = `<span class="version-value">v${escapeHtml(mod.version)}</span> ${updateBadge} ${ignoredLabel} <button class="btn-tiny version-edit-btn" style="margin-left: 6px;">${escapeHtml(t('common.edit'))}</button>`;

  const editBtn = el.querySelector('.version-edit-btn') as HTMLButtonElement;
  const valSpan = el.querySelector('.version-value') as HTMLSpanElement;

  // Event listeners for ignore/unignore
  const ignoreBtn = el.querySelector('.ignore-update-btn') as HTMLButtonElement | null;
  if (ignoreBtn) {
    ignoreBtn.addEventListener('click', async () => {
      const latest = ignoreBtn.dataset.latest || '';
      try {
        await ignoreModVersion(mod.id, latest);
        showToast(t('toasts.settings_saved'), 'success');
        await loadMods();
        const { openDetailPanel } = await import('./panel');
        openDetailPanel(mod.id);
      } catch (e) {
        showToast(t('toasts.export_failed', { error: String(e) }), 'error');
      }
    });
  }

  const unignoreBtn = el.querySelector('.unignore-update-btn') as HTMLButtonElement | null;
  if (unignoreBtn) {
    unignoreBtn.addEventListener('click', async () => {
      try {
        await ignoreModVersion(mod.id, null);
        showToast(t('toasts.settings_saved'), 'success');
        await loadMods();
        const { openDetailPanel } = await import('./panel');
        openDetailPanel(mod.id);
      } catch (e) {
        showToast(t('toasts.export_failed', { error: String(e) }), 'error');
      }
    });
  }

  editBtn.addEventListener('click', () => {
    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'version-input';
    input.value = mod.version;
    input.maxLength = 50;
    valSpan.textContent = '';
    valSpan.appendChild(input);
    input.focus();
    input.select();
    editBtn.style.display = 'none';
    const done = async (save: boolean) => {
      if (save) {
        const newVer = input.value.trim();
        if (newVer && newVer !== mod.version) {
          try {
            const updated = await setModVersion(mod.id, newVer);
            const state = getState();
            const idx = state.allMods.findIndex(m => m.id === mod.id);
            if (idx >= 0) {
              const newMods = [...state.allMods];
              newMods[idx] = updated;
              updateState({ allMods: newMods });
            }
            renderVersion(updated);
            renderModsView();
            showToast(t('toasts.mod_updated', { name: mod.name }), 'success');
          } catch (e) {
            showToast(t('toasts.export_failed', { error: String(e) }), 'error');
            renderVersion(mod);
          }
        } else {
          renderVersion(mod);
        }
      } else {
        renderVersion(mod);
      }
    };
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') { input.blur(); done(true); }
      if (e.key === 'Escape') { input.blur(); done(false); }
    });
    input.addEventListener('blur', () => done(true));
  });
}

export function renderGithubSection(mod: ModInfo): void {
  const container = document.getElementById('detail-github') as HTMLElement | null;
  if (!container) return;

  if (!mod.githubRepo) {
    container.innerHTML = `
      <div class="detail-row"><span class="detail-label">${escapeHtml(t('detail.github_label'))}</span>
        <span><input type="text" class="github-repo-input" placeholder="owner/repo..." /><button class="btn-tiny github-add-btn" style="margin-left:4px">${escapeHtml(t('common.save'))}</button></span>
      </div>`;
    container.style.display = 'block';
    const input = container.querySelector('.github-repo-input') as HTMLInputElement;
    const addBtn = container.querySelector('.github-add-btn') as HTMLButtonElement;
    const doAdd = async () => {
      const repo = input.value.trim();
      if (!repo) return;
      addBtn.disabled = true;
      try {
        const latest = await checkGitHubVersion(repo);
        const updated = await setGithubVersion(mod.id, repo, latest);
        const state = getState();
        const idx = state.allMods.findIndex(m => m.id === mod.id);
        if (idx >= 0) {
          const newMods = [...state.allMods];
          newMods[idx] = updated;
          updateState({ allMods: newMods });
        }
        renderGithubSection(updated);
        renderModsView();
        showToast(t('toasts.settings_saved'), 'success');
      } catch (e) {
        showToast(t('toasts.export_failed', { error: String(e) }), 'error');
        renderGithubSection(mod);
      }
    };
    addBtn.addEventListener('click', doAdd);
    input.addEventListener('keydown', (e) => { if (e.key === 'Enter') doAdd(); });
    return;
  }

  const repoLink = `https://github.com/${mod.githubRepo}`;
  const versionDisplay = mod.githubVersion
    ? `<span class="github-version-value">${escapeHtml(mod.githubVersion)}</span>`
    : `<span class="github-version-value" style="color:var(--text-muted)">${escapeHtml(t('common.unknown'))}</span>`;
  const cachedInfo = mod.githubCachedAt
    ? `<div class="detail-row"><span class="detail-label">${escapeHtml(t('detail.last_updated_label'))}</span> ${new Date(mod.githubCachedAt).toLocaleDateString()}</div>`
    : '';

  container.innerHTML = `
    <div class="detail-row"><span class="detail-label">${escapeHtml(t('detail.github_label'))}</span> <a class="nexus-link" href="${repoLink}" target="_blank">${escapeHtml(mod.githubRepo)}</a></div>
    <div class="detail-row"><span class="detail-label">${escapeHtml(t('common.version'))}:</span> ${versionDisplay} <button class="btn-tiny github-refresh-btn">${escapeHtml(t('common.refresh'))}</button></div>
    ${cachedInfo}
  `;

  const refreshBtn = container.querySelector('.github-refresh-btn') as HTMLButtonElement;
  refreshBtn.addEventListener('click', async () => {
    refreshBtn.disabled = true;
    refreshBtn.textContent = '...';
    try {
      const latest = await checkGitHubVersion(mod.githubRepo!);
      const updated = await setGithubVersion(mod.id, mod.githubRepo!, latest);
      const state = getState();
      const idx = state.allMods.findIndex(m => m.id === mod.id);
      if (idx >= 0) {
        const newMods = [...state.allMods];
        newMods[idx] = updated;
        updateState({ allMods: newMods });
      }
      renderGithubSection(updated);
      renderModsView();
      showToast(t('toasts.settings_saved'), 'success');
    } catch (e) {
      showToast(t('toasts.export_failed', { error: String(e) }), 'error');
      renderGithubSection(mod);
    }
  });
}
