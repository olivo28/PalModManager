import {
  analyzeZip,
  checkModExistsCommand,
  updateModCommand,
  installMod,
  checkDependencies,
  installUe4ss,
  installPalschema,
  buildInstallManifest,
  installModWithManifest
} from '../../../api';
import { getState, updateState } from '../../../state';
import { showToast } from '../../toast';
import { showConfirm } from '../../confirm';
import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import type { BatchItem } from './types';
import {
  _pendingUpdateModId,
  _pendingBatchPaths,
  _batchItems,
  _isProcessingInstall,
  setPendingBatchPaths,
  setPendingUpdateModId,
  setBatchItems,
  setLastInstallSuccess,
  setIsProcessingInstall
} from './state';
import {
  showInstallModal,
  closeInstallModal,
  setModalStatus,
  getCleanNameFromFilename
} from './helpers';
import { showFileTreeModal } from './fileTree';
import { renderInstallPreview } from './single';
import { installerDom, discoveryDom } from '../../../framework';

export async function renderBatchInstallPreview(paths: string[]): Promise<void> {
  setPendingBatchPaths(paths);
  updateState({ currentAnalysis: null });
  setPendingUpdateModId(null);
  setBatchItems([]);

  const content = installerDom.el('modal-content');
  const confirmBtn = installerDom.el('modal-confirm') as HTMLButtonElement;
  const statusEl = installerDom.el('modal-status');

  confirmBtn.disabled = true;
  confirmBtn.textContent = t('installer.btn_install');
  statusEl.textContent = '';

  content.innerHTML = `
    <div style="display:flex;flex-direction:column;align-items:center;padding:24px;color:var(--text-secondary)">
      <div style="font-size:14px;font-weight:600;margin-bottom:8px">Analyzing ${paths.length} archives...</div>
      <div style="font-size:11px;color:var(--text-muted)">Scanning contents, checking versions and detecting types</div>
    </div>
  `;
  showInstallModal();

  const results: BatchItem[] = [];
  for (const path of paths) {
    const filename = path.split(/[/\\]/).pop() || '';
    try {
      const analysis = await analyzeZip(path);
      let existingModId: string | null = null;
      let existingModInfo: any = null;
      try {
        const checkResult = await checkModExistsCommand(path);
        if (checkResult.exists && checkResult.modInfo) {
          existingModId = checkResult.modInfo.id;
          existingModInfo = checkResult.modInfo;
        }
      } catch { }

      let nameVal = getCleanNameFromFilename(filename);
      if (!analysis.nexusInfo && analysis.modinfo?.name) {
        nameVal = analysis.modinfo.name;
      }

      const isLogicModsDefault = analysis.detectedType === 'logicmods' || (analysis.files && analysis.files.some((f: string) => f.toLowerCase().includes('logicmods')));
      const manifest = await buildInstallManifest(
        path,
        getState().currentSettings?.gamePath || '',
        isLogicModsDefault ? 'logicmods' : '~mods',
        nameVal
      );

      results.push({
        path,
        filename,
        name: nameVal,
        type: manifest.modType,
        existingModId,
        existingModInfo,
        existingVersion: existingModInfo?.version,
        nexusModId: analysis.nexusModId,
        version: analysis.detectedVersion || analysis.nexusInfo?.version || '1.0',
        hasPak: manifest.hasPak,
        isLogicModsDefault,
      });
    } catch (e) {
      results.push({
        path,
        filename,
        name: filename,
        type: 'unknown',
        existingModId: null,
        error: String(e),
      });
    }
  }

  setBatchItems(results);

  const rows = results.map((item, idx) => {
    let stateBadge = `<span style="background:var(--accent-dim);color:var(--accent);border:1px solid var(--accent);font-size:8px;padding:1px 4px;font-weight:700;border-radius:2px;">NEW</span>`;
    if (item.existingModId) {
      if (item.existingVersion && item.version && item.existingVersion.trim().toLowerCase() === item.version.trim().toLowerCase()) {
        stateBadge = `<span style="background:rgba(255,255,255,0.06);color:var(--text-muted);border:1px solid var(--border);font-size:8px;padding:1px 4px;font-weight:700;border-radius:2px;">INSTALLED</span>`;
      } else {
        stateBadge = `<span style="background:var(--success-dim);color:var(--success);border:1px solid var(--success);font-size:8px;padding:1px 4px;font-weight:700;border-radius:2px;">UPDATE</span>`;
      }
    }

    const idText = item.nexusModId ? `#${item.nexusModId}` : '<span style="color:var(--text-muted)">—</span>';
    const verText = item.version ? `v${item.version}` : '<span style="color:var(--text-muted)">—</span>';

    const isPakOrLogicOrHybrid = item.type === 'pak' || item.type === 'logicmods' || (item.type === 'hybrid' && item.hasPak);
    let pakDestSelectHtml = `<span style="color:var(--text-muted);font-size:10px;">—</span>`;
    if (isPakOrLogicOrHybrid) {
      pakDestSelectHtml = `
        <select id="batch-pak-dest-${idx}" style="padding:2px 4px;background:var(--bg-primary);color:var(--text-primary);border:1px solid var(--border);font-size:10px;width:100%;">
          <option value="~mods" ${!item.isLogicModsDefault ? 'selected' : ''}>~mods</option>
          <option value="logicmods" ${item.isLogicModsDefault ? 'selected' : ''}>logicmods</option>
        </select>
      `;
    }

    return `
      <tr style="border-bottom:1px solid var(--border-light)">
        <td style="padding:6px 4px;width:28px;"><input type="checkbox" id="batch-install-${idx}" checked style="cursor:pointer;" /></td>
        <td style="padding:6px;font-size:10px;width:180px;max-width:180px;color:var(--text-secondary);">
          <div style="display:flex;align-items:center;justify-content:space-between;gap:8px;overflow:hidden;">
            <span style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex-grow:1;" title="${escapeHtml(item.filename)}">${escapeHtml(item.filename)}</span>
        <button id="batch-view-files-${idx}" style="padding:2px 6px;background:var(--bg-secondary);color:var(--accent);border:1px solid var(--border);border-radius:4px;font-size:9px;cursor:pointer;white-space:nowrap;font-weight:600;" onmouseover="this.style.background='rgba(255,255,255,0.05)'" onmouseout="this.style.background='var(--bg-secondary)'">${escapeHtml(t('installer.btn_show_files'))}</button>
          </div>
        </td>
        <td style="padding:6px;font-size:11px;width:70px;white-space:nowrap;color:var(--text-muted);font-weight:600;">${idText}</td>
        <td style="padding:6px;font-size:11px;width:60px;white-space:nowrap;color:var(--text-primary);font-weight:600;">${verText}</td>
        <td style="padding:6px;"><input type="text" id="batch-name-${idx}" value="${escapeHtml(item.name)}" style="width:100%;padding:2px 4px;background:var(--bg-primary);color:var(--text-primary);border:1px solid var(--border);font-size:11px;" /></td>
        <td style="padding:6px;width:90px;">
          <select id="batch-type-${idx}" style="padding:2px 4px;background:var(--bg-primary);color:var(--text-primary);border:1px solid var(--border);font-size:10px;width:100%;">
            <option value="ue4ss" ${item.type === 'ue4ss' ? 'selected' : ''}>UE4SS</option>
            <option value="palschema" ${item.type === 'palschema' ? 'selected' : ''}>PalSchema</option>
            <option value="pak" ${item.type === 'pak' || item.type === 'logicmods' ? 'selected' : ''}>Pak</option>
            <option value="hybrid" ${item.type === 'hybrid' ? 'selected' : ''}>${escapeHtml(t('card.type_hybrid'))}</option>
            <option value="altermatic" ${item.type === 'altermatic' ? 'selected' : ''}>Altermatic</option>
          </select>
        </td>
        <td id="batch-pak-dest-container-${idx}" style="padding:6px;width:95px;">${pakDestSelectHtml}</td>
        <td style="padding:6px;width:50px;text-align:right;">${stateBadge}</td>
      </tr>
    `;
  }).join('');

  content.innerHTML = `
    <div id="batch-table-wrapper" style="overflow-y:auto;border:1px solid var(--border);background:var(--bg-secondary);border-radius:4px;margin-bottom:8px;">
      <table style="width:100%;border-collapse:collapse;text-align:left;">
        <thead style="position:sticky;top:0;z-index:2;">
          <tr style="background:var(--bg-tertiary);border-bottom:1px solid var(--border);font-size:10px;font-weight:700;color:var(--text-muted);text-transform:uppercase;">
            <th style="padding:6px;width:28px;">${escapeHtml(t('installer.batch_col_install'))}</th>
            <th style="padding:6px;width:180px;">${escapeHtml(t('installer.batch_col_archive'))}</th>
            <th style="padding:6px;width:70px;">${escapeHtml(t('installer.batch_col_nexus_id'))}</th>
            <th style="padding:6px;width:60px;">${escapeHtml(t('installer.batch_col_version'))}</th>
            <th style="padding:6px;">${escapeHtml(t('installer.batch_col_target_folder'))}</th>
            <th style="padding:6px;width:90px;">${escapeHtml(t('installer.batch_col_type'))}</th>
            <th style="padding:6px;width:95px;">${escapeHtml(t('installer.batch_col_pak_target'))}</th>
            <th style="padding:6px;width:50px;text-align:right;padding-right:12px;">${escapeHtml(t('installer.batch_col_status'))}</th>
          </tr>
        </thead>
        <tbody>
          ${rows}
        </tbody>
      </table>
    </div>
  `;

  for (let i = 0; i < results.length; i++) {
    const item = results[i];
    const typeSelect = document.getElementById(`batch-type-${i}`) as HTMLSelectElement | null;
    if (typeSelect) {
      typeSelect.addEventListener('change', () => {
        const val = typeSelect.value;
        const destContainer = document.getElementById(`batch-pak-dest-container-${i}`);
        if (destContainer) {
          if (val === 'pak' || val === 'logicmods' || (val === 'hybrid' && item.hasPak)) {
            destContainer.innerHTML = `
              <select id="batch-pak-dest-${i}" style="padding:2px 4px;background:var(--bg-primary);color:var(--text-primary);border:1px solid var(--border);font-size:10px;width:100%;">
                <option value="~mods" ${val === 'pak' || val === 'hybrid' ? 'selected' : ''}>~mods</option>
                <option value="logicmods" ${val === 'logicmods' ? 'selected' : ''}>logicmods</option>
              </select>
            `;
          } else {
            destContainer.innerHTML = `<span style="color:var(--text-muted);font-size:10px;">—</span>`;
          }
        }
      });
    }

    const viewBtn = document.getElementById(`batch-view-files-${i}`) as HTMLButtonElement | null;
    if (viewBtn) {
      viewBtn.addEventListener('click', async (e) => {
        e.preventDefault();
        viewBtn.textContent = t('installer.btn_loading_files');
        viewBtn.disabled = true;
        try {
          const customName = (document.getElementById(`batch-name-${i}`) as HTMLInputElement)?.value || item.name;
          const pakDest = (document.getElementById(`batch-pak-dest-${i}`) as HTMLSelectElement)?.value || '~mods';
          const manifest = await buildInstallManifest(
            item.path,
            getState().currentSettings?.gamePath || '',
            pakDest,
            customName
          );
          showFileTreeModal(manifest.routes, customName, item.path);
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        } finally {
          viewBtn.textContent = t('installer.btn_show_files');
          viewBtn.disabled = false;
        }
      });
    }
  }

  const modalEl = document.querySelector('#install-modal .modal') as HTMLElement | null;
  if (modalEl) {
    modalEl.style.width = '900px';
  }
  const wrapper = installerDom.elMaybe('batch-table-wrapper');
  if (wrapper) {
    wrapper.style.maxHeight = 'calc(80vh - 150px)';
  }

  confirmBtn.disabled = false;
}

export async function handleInstallConfirm(): Promise<void> {
  if (_isProcessingInstall) return;
  setIsProcessingInstall(true);

  const confirmBtn = installerDom.el('modal-confirm');
  const cancelBtn = installerDom.el('modal-cancel');
  const statusEl = installerDom.el('modal-status');
  const contentEl = installerDom.el('modal-content');

  confirmBtn.disabled = true;
  cancelBtn.disabled = true;

  if (_pendingBatchPaths.length > 0) {
    const itemsToInstall: Array<{
      path: string;
      filename: string;
      customName: string;
      customType: string;
      pakDestination: string | null;
      existingModId: string | null;
    }> = [];

    for (let i = 0; i < _batchItems.length; i++) {
      const item = _batchItems[i];
      const installCheckbox = document.getElementById(`batch-install-${i}`) as HTMLInputElement | null;

      if (installCheckbox && installCheckbox.checked && !item.error) {
        const nameInput = document.getElementById(`batch-name-${i}`) as HTMLInputElement | null;
        const typeSelect = document.getElementById(`batch-type-${i}`) as HTMLSelectElement | null;
        const pakDestSelect = document.getElementById(`batch-pak-dest-${i}`) as HTMLSelectElement | null;

        const inputName = nameInput && nameInput.value.trim() ? nameInput.value.trim() : item.name;
        const isUpdate = !!item.existingModId;

        itemsToInstall.push({
          path: item.path,
          filename: item.filename,
          customName: inputName,
          customType: typeSelect ? typeSelect.value : item.type,
          pakDestination: pakDestSelect ? pakDestSelect.value : null,
          existingModId: isUpdate ? item.existingModId : null,
        });
      }
    }

    let installed = 0;
    let updated = 0;
    let failed = 0;

    const resultsHtml: string[] = [];
    contentEl.innerHTML = `
      <div class="install-console-header" style="display:flex;align-items:center;background:#181818;padding:6px 12px;border-top-left-radius:6px;border-top-right-radius:6px;border-bottom:1px solid #282828;">
        <span style="font-size:10px;font-family:monospace;color:#888;font-weight:600;">install_log.sh</span>
        <div style="flex:1"></div>
        <div style="display:flex;gap:5px;">
          <span style="width:8px;height:8px;border-radius:50%;background:#ff5f56;display:inline-block;"></span>
          <span style="width:8px;height:8px;border-radius:50%;background:#ffbd2e;display:inline-block;"></span>
          <span style="width:8px;height:8px;border-radius:50%;background:#27c93f;display:inline-block;"></span>
        </div>
      </div>
      <div class="batch-results-list" style="display:flex;flex-direction:column;gap:6px;max-height:280px;min-height:220px;overflow-y:auto;background:#0d0d0d;padding:14px;font-family:monospace;font-size:11px;line-height:1.5;border-bottom-left-radius:6px;border-bottom-right-radius:6px;box-shadow:inset 0 0 10px rgba(0,0,0,0.8);color:#d0d0d0;border:1px solid #282828;border-top:none;"></div>
    `;
    const resultsList = contentEl.querySelector('.batch-results-list')!;

    for (let i = 0; i < itemsToInstall.length; i++) {
      const item = itemsToInstall[i];
      statusEl.textContent = `Processing ${item.filename} (${i + 1}/${itemsToInstall.length})...`;

      resultsHtml.push(`<div class="batch-result-item" style="color:#e0af68;font-style:italic;">&gt; Extracting and copying files for ${escapeHtml(item.customName)}...</div>`);
      resultsList.innerHTML = resultsHtml.join('');
      resultsList.scrollTop = resultsList.scrollHeight;

      try {
        if (item.existingModId) {
          await updateModCommand(item.path, item.existingModId);
          updated++;
          resultsHtml.pop();
          resultsHtml.push(`<div class="batch-result-item success" style="color:#00bcff;font-weight:bold;"><span style="color:#777;">[UP]</span> Updated successfully: ${escapeHtml(item.customName)} (${escapeHtml(item.customType)})</div>`);
        } else {
          await installMod(item.path, item.customType, item.pakDestination, item.customName);
          installed++;
          resultsHtml.pop();
          resultsHtml.push(`<div class="batch-result-item success" style="color:#4af626;font-weight:bold;"><span style="color:#777;">[OK]</span> Installed successfully: ${escapeHtml(item.customName)} (${escapeHtml(item.customType)})</div>`);
        }
      } catch (e) {
        failed++;
        resultsHtml.pop();
        resultsHtml.push(`<div class="batch-result-item error" style="color:#ff4a4a;font-weight:bold;"><span style="color:#777;">[ERR]</span> Failed: ${escapeHtml(item.filename)} - ${escapeHtml(String(e))}</div>`);
      }

      resultsList.innerHTML = resultsHtml.join('');
      resultsList.scrollTop = resultsList.scrollHeight;
    }

    statusEl.textContent = t('installer.status_batch_complete', { installed, updated, failed });
    cancelBtn.disabled = false;
    cancelBtn.textContent = t('common.close');
    confirmBtn.style.display = 'none';

    const { loadMods, loadLibrary } = await import('../../modsView');
    await loadMods();
    await loadLibrary();
    return;
  }

  const state = getState();
  if (!state.currentAnalysis) {
    setIsProcessingInstall(false);
    return;
  }

  const typeSelect = installerDom.elMaybe('mod-type-select');
  const customType = typeSelect ? typeSelect.value : state.currentAnalysis.detectedType;

  const nameInput = installerDom.elMaybe('mod-name-input');
  const customName = nameInput && nameInput.value.trim() ? nameInput.value.trim() : null;

  let pakDestination: string | null = null;
  if (customType === 'pak' || customType === 'logicmods' || customType === 'hybrid') {
    const checked = document.querySelector('input[name="pak-dest"]:checked') as HTMLInputElement;
    pakDestination = checked ? checked.value : (customType === 'logicmods' ? 'logicmods' : '~mods');
  }

  confirmBtn.textContent = _pendingUpdateModId ? 'Updating...' : 'Installing...';
  statusEl.textContent = _pendingUpdateModId ? 'Updating mod...' : 'Extracting and installing mod...';

  contentEl.innerHTML = `
    <div class="install-console-header" style="display:flex;align-items:center;background:#181818;padding:6px 12px;border-top-left-radius:6px;border-top-right-radius:6px;border-bottom:1px solid #282828;">
      <span style="font-size:10px;font-family:monospace;color:#888;font-weight:600;">install_log.sh</span>
      <div style="flex:1"></div>
      <div style="display:flex;gap:5px;">
        <span style="width:8px;height:8px;border-radius:50%;background:#ff5f56;display:inline-block;"></span>
        <span style="width:8px;height:8px;border-radius:50%;background:#ffbd2e;display:inline-block;"></span>
        <span style="width:8px;height:8px;border-radius:50%;background:#27c93f;display:inline-block;"></span>
      </div>
    </div>
    <div class="batch-results-list" style="display:flex;flex-direction:column;gap:6px;max-height:280px;min-height:220px;overflow-y:auto;background:#0d0d0d;padding:14px;font-family:monospace;font-size:11px;line-height:1.5;border-bottom-left-radius:6px;border-bottom-right-radius:6px;box-shadow:inset 0 0 10px rgba(0,0,0,0.8);color:#d0d0d0;border:1px solid #282828;border-top:none;"></div>
  `;
  confirmBtn.textContent = _pendingUpdateModId ? t('installer.status_updating_btn') : t('installer.status_installing_btn');
  statusEl.textContent = _pendingUpdateModId ? t('installer.status_updating') : t('installer.status_installing');
  confirmBtn.disabled = true;
  cancelBtn.disabled = true;

  const logs: string[] = [];
  const resultsList = contentEl.querySelector('.batch-results-list') as HTMLElement;
  if (resultsList) {
    resultsList.innerHTML = logs.join('');
  }

  const depStatus = await checkDependencies();
  const ue4ssRequired = ['ue4ss', 'palschema', 'hybrid'].includes(customType);
  const palschemaRequired = (customType === 'palschema') || (customType === 'hybrid' && (state.currentAnalysis.hasPalSchemaJson || (state.currentAnalysis.files || []).some((f: string) => f.toLowerCase().includes('palschema'))));

  const missingUe4ss = ue4ssRequired && !depStatus.ue4ss_installed;
  const missingPalSchema = palschemaRequired && !depStatus.palschema_installed;

  if (missingUe4ss || missingPalSchema) {
    const missingNames: string[] = [];
    if (missingUe4ss) missingNames.push('UE4SS');
    if (missingPalSchema) missingNames.push('PalSchema');

    logs.push(`<div style="color:#ff9d00;font-weight:bold;">[WARN] Missing required dependencies: ${missingNames.join(', ')}</div>`);
    logs.push(`<div style="color:#888;">&gt; Please click "Install Deps & Retry" to install them automatically.</div>`);
    resultsList.innerHTML = logs.join('');
    resultsList.scrollTop = resultsList.scrollHeight;
    statusEl.textContent = t('installer.status_missing_deps');

    confirmBtn.disabled = false;
    cancelBtn.disabled = false;
    confirmBtn.textContent = _pendingUpdateModId ? t('installer.btn_update') : t('installer.btn_install');

    // Allow the user to retry — unlock the processing guard so the Install
    // button is responsive again if they dismiss the deps dialog.
    setIsProcessingInstall(false);

    const retryBtn = installerDom.elMaybe('modal-install-deps-retry');
    if (retryBtn) {
      retryBtn.style.display = '';
      retryBtn.onclick = async () => {
        retryBtn.disabled = true;
        try {
          if (missingUe4ss) {
            logs.push(`<div style="color:#e0af68;">&gt; Downloading and installing UE4SS dependency...</div>`);
            resultsList.innerHTML = logs.join('');
            resultsList.scrollTop = resultsList.scrollHeight;
            statusEl.textContent = t('installer.status_downloading_ue4ss');
            await installUe4ss();
            logs.push(`<div style="color:#4af626;font-weight:bold;">[OK] UE4SS installed successfully!</div>`);
            resultsList.innerHTML = logs.join('');
            resultsList.scrollTop = resultsList.scrollHeight;
          }
          if (missingPalSchema) {
            logs.push(`<div style="color:#e0af68;">&gt; Downloading and installing PalSchema dependency...</div>`);
            resultsList.innerHTML = logs.join('');
            resultsList.scrollTop = resultsList.scrollHeight;
            statusEl.textContent = t('installer.status_downloading_palschema');
            await installPalschema();
            logs.push(`<div style="color:#4af626;font-weight:bold;">[OK] PalSchema installed successfully!</div>`);
            resultsList.innerHTML = logs.join('');
            resultsList.scrollTop = resultsList.scrollHeight;
          }

          const { loadDependencies } = await import('../../modsView');
          await loadDependencies();

          logs.push(`<div style="color:#888;">&gt; Dependencies installed. Starting mod installation...</div>`);
          resultsList.innerHTML = logs.join('');
          resultsList.scrollTop = resultsList.scrollHeight;

          retryBtn.style.display = 'none';

          setTimeout(() => {
            executeModInstallation(logs, resultsList, statusEl, confirmBtn, cancelBtn, customType, customName, state, pakDestination);
          }, 1000);
        } catch (err) {
          logs.push(`<div style="color:#ff4a4a;font-weight:bold;">[ERR] ${escapeHtml(t('installer.status_deps_failed'))}: ${escapeHtml(String(err))}</div>`);
          resultsList.innerHTML = logs.join('');
          resultsList.scrollTop = resultsList.scrollHeight;
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
          statusEl.textContent = t('installer.status_deps_failed');
        } finally {
          retryBtn.disabled = false;
        }
      };

      showConfirm(t('installer.confirm_install_missing_deps', { deps: missingNames.join(' & ') }))
        .then(confirmed => {
          if (confirmed) {
            retryBtn.click();
          }
        });
    }
    return;
  }

  await executeModInstallation(logs, resultsList, statusEl, confirmBtn, cancelBtn, customType, customName, state, pakDestination);
}

async function executeModInstallation(
  logs: string[],
  resultsList: HTMLElement,
  statusEl: HTMLElement,
  confirmBtn: HTMLButtonElement,
  cancelBtn: HTMLButtonElement,
  customType: string,
  customName: string | null,
  state: any,
  pakDestination: string | null
) {
  logs.push(`<div style="color:#e0af68;">&gt; Extracting ZIP contents to temporary directory...</div>`);
  resultsList.innerHTML = logs.join('');

  try {
    logs.push(`<div style="color:#e0af68;">&gt; Copying files to destination folder...</div>`);
    resultsList.innerHTML = logs.join('');
    resultsList.scrollTop = resultsList.scrollHeight;

    const manifest = await buildInstallManifest(
      state.currentAnalysis.zipPath,
      state.currentSettings?.gamePath || '',
      pakDestination,
      customName
    );

    if (customName) {
      manifest.displayName = customName;
    }
    const versionInput = installerDom.elMaybe('mod-version-input');
    const inputVer = versionInput?.value.trim();
    if (inputVer && !/^[0-9a-fA-F-]{6,}$/.test(inputVer)) {
      manifest.version = inputVer;
    } else if (state.currentAnalysis.modinfo?.version) {
      manifest.version = state.currentAnalysis.modinfo.version;
    } else if (state.currentAnalysis.detectedVersion && !/^[0-9a-fA-F-]{6,}$/.test(state.currentAnalysis.detectedVersion)) {
      manifest.version = state.currentAnalysis.detectedVersion;
    } else if (state.currentAnalysis.nexusInfo?.version) {
      manifest.version = state.currentAnalysis.nexusInfo.version;
    } else {
      manifest.version = '1.0.0';
    }

    if (state.currentAnalysis.nexusModId) {
      manifest.nexusModId = state.currentAnalysis.nexusModId;
    }

    const appState = getState();
    const existingCompanionMod = state.currentAnalysis.nexusModId
      ? appState.allMods.find(m => m.nexusModId === state.currentAnalysis.nexusModId && m.id !== _pendingUpdateModId)
      : null;

    let installedMod: any = null;
    if (_pendingUpdateModId) {
      installedMod = await updateModCommand(state.currentAnalysis.zipPath, _pendingUpdateModId);
    } else {
      installedMod = await installModWithManifest(manifest, state.currentAnalysis.zipPath);
    }

    if (existingCompanionMod) {
      logs.push(`<div style="color:#00bcff;font-weight:bold;">${escapeHtml(t('installer.log_merged_hybrid', { name: existingCompanionMod.name }))}</div>`);
      showToast(t('toasts.merge_hybrid_success', { name: existingCompanionMod.name }), 'success');
    }

    const restoreCheckbox = document.getElementById('restore-archived-config-checkbox') as HTMLInputElement | null;
    if (restoreCheckbox && restoreCheckbox.checked && restoreCheckbox.dataset.archiveId && installedMod) {
      try {
        const { applyArchivedConfig } = await import('../../../api');
        const targetId = installedMod.id || manifest.displayName || customName || '';
        const restored = await applyArchivedConfig(targetId, restoreCheckbox.dataset.archiveId);
        if (restored) {
          logs.push(`<div style="color:#2ecc71;font-weight:bold;">[OK] Restored previously archived configuration settings!</div>`);
          resultsList.innerHTML = logs.join('');
        }
      } catch (err) {
        logs.push(`<div style="color:#ffaa00;">[WARN] Could not restore archived config: ${escapeHtml(String(err))}</div>`);
        resultsList.innerHTML = logs.join('');
      }
    }

    setLastInstallSuccess(true);

    logs.push(`<div style="color:#4af626;font-weight:bold;">[OK] Mod installed successfully!</div>`);

    resultsList.innerHTML = logs.join('');
    resultsList.scrollTop = resultsList.scrollHeight;

    statusEl.textContent = _pendingUpdateModId ? 'Updated successfully!' : 'Installed successfully!';
    setTimeout(async () => {
      closeInstallModal();
      const { loadMods, loadLibrary, loadProfiles, loadDependencies } = await import('../../modsView');
      await Promise.all([loadMods(), loadLibrary(), loadProfiles(), loadDependencies(true)]);
    }, 1500);
  } catch (e) {
    setIsProcessingInstall(false);
    setLastInstallSuccess(false);
    logs.push(`<div style="color:#ff4a4a;font-weight:bold;">[ERR] Installation failed: ${escapeHtml(String(e))}</div>`);
    resultsList.innerHTML = logs.join('');
    resultsList.scrollTop = resultsList.scrollHeight;
    statusEl.textContent = 'Installation failed';
    confirmBtn.disabled = false;
    cancelBtn.disabled = false;
    confirmBtn.textContent = _pendingUpdateModId ? t('installer.btn_update') : t('installer.btn_install');
  }
}

export async function openInstallModalForZip(
  zipPath: string,
  preferredName?: string,
  preferredNexusId?: number,
  preferredVersion?: string
): Promise<void> {
  setLastInstallSuccess(false);
  // Always reset the processing guard so a new install never silently no-ops.
  setIsProcessingInstall(false);

  // Dismiss Discovery modal cleanly if currently open
  const discModal = discoveryDom.elMaybe('discovery-mod-modal');
  if (discModal && discModal.classList.contains('visible')) {
    const { closeDiscoveryModal } = await import('../../discoveryView');
    closeDiscoveryModal();
  }

  showInstallModal();
  setModalStatus(t('installer.status_analyzing'));

  try {
    const analysis = await analyzeZip(zipPath);
    if (preferredNexusId) {
      analysis.nexusModId = preferredNexusId;
    }
    if (preferredName) {
      (analysis as any).preferredName = preferredName;
    }
    if (preferredVersion) {
      analysis.detectedVersion = preferredVersion;
    }

    let existingMod: { id: string; name: string, version: string } | null = null;
    try {
      const checkResult = await checkModExistsCommand(zipPath);
      if (checkResult.exists && checkResult.modInfo) {
        existingMod = { id: checkResult.modInfo.id, name: checkResult.modInfo.name, version: checkResult.modInfo.version };
      }
    } catch { }

    renderInstallPreview(analysis, existingMod);
  } catch (err: any) {
    console.error('[Installer] analyzeZip error:', err);
    closeInstallModal();
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
    throw err;
  }
}

export async function handleInstall(): Promise<void> {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      multiple: true,
      filters: [{ name: 'Mod Archives', extensions: ['zip', 'rar', '7z'] }],
      title: t('installer.dialog_select_archive_title'),
    });

    if (!selected) return;

    const paths = Array.isArray(selected) ? selected : [selected];

    if (paths.length === 1) {
      await openInstallModalForZip(paths[0]);
    } else {
      renderBatchInstallPreview(paths);
    }
  } catch (e) {
    console.error('Error analyzing:', e);
    closeInstallModal();
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  }
}
