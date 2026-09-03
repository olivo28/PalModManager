import {
  buildInstallManifest,
  fetchNexusInfoAsync,
  previewConfigDiff,
  setModIgnoredKeys,
  openUrl
} from '../../../api';
import type { ZipAnalysis, InstallManifest } from '../../../api';
import { getState, updateState } from '../../../state';
import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import { getCleanNameFromFilename } from './helpers';
import { showFileTreeModal } from './fileTree';
import { setPendingUpdateModId } from './state';
import { installerDom } from '../../../framework';

export async function renderInstallPreview(analysis: ZipAnalysis, existingMod: { id: string; name: string, version: string } | null = null): Promise<void> {
  updateState({ currentAnalysis: analysis });
  const content = installerDom.el('modal-content');
  const confirmBtn = installerDom.el('modal-confirm');
  const statusEl = installerDom.el('modal-status');

  const retryBtn = installerDom.elMaybe('modal-install-deps-retry');
  if (retryBtn) {
    retryBtn.style.display = 'none';
  }

  statusEl.textContent = 'Preparing install...';
  confirmBtn.disabled = true;

  // Inherit existing mod metadata if available and missing on analysis
  if (existingMod && !analysis.nexusInfo) {
    const installed = getState().allMods.find(m => m.id === existingMod.id || m.name === existingMod.name);
    if (installed && (installed.nexusPictureUrl || installed.nexusAuthor || installed.nexusModId)) {
      if (!analysis.nexusModId && installed.nexusModId) {
        analysis.nexusModId = installed.nexusModId;
      }
      analysis.nexusInfo = {
        name: installed.name,
        author: installed.nexusAuthor || '',
        summary: installed.nexusSummary || '',
        pictureUrl: installed.nexusPictureUrl || '',
        version: installed.version || '',
        downloads: installed.nexusDownloads || 0,
        endorsements: installed.nexusEndorsements || 0,
      };
    }
  }

  // Background fetch of Nexus metadata for rich single-mod preview card
  if (analysis.nexusModId && !analysis.nexusInfo) {
    fetchNexusInfoAsync(analysis.nexusModId).then(info => {
      if (getState().currentAnalysis === analysis && info) {
        analysis.nexusInfo = info;
        renderInstallPreview(analysis, existingMod);
      }
    }).catch(() => { });
  }

  let cleanName = (analysis as any).preferredName || '';
  if (!cleanName && analysis.nexusInfo?.name) {
    cleanName = analysis.nexusInfo.name;
  }
  if (!cleanName && analysis.modinfo?.name) {
    cleanName = analysis.modinfo.name;
  }
  if (!cleanName) {
    const filename = analysis.zipPath.split(/[/\\]/).pop() || '';
    if (!filename.toLowerCase().startsWith('nexus_')) {
      cleanName = getCleanNameFromFilename(filename);
    }
  }

  let manifest: InstallManifest;
  try {
    manifest = await buildInstallManifest(
      analysis.zipPath,
      getState().currentSettings?.gamePath || '',
      analysis.detectedType === 'logicmods' ? 'logicmods' : '~mods',
      cleanName || null
    );
  } catch (err) {
    content.innerHTML = `<div style="padding:20px;color:#ff4a4a;font-weight:bold;">Error analyzing manifest: ${escapeHtml(String(err))}</div>`;
    return;
  }

  if (!cleanName || cleanName.toLowerCase().startsWith('nexus_') || /^[0-9a-fA-F-]{8,}$/.test(cleanName)) {
    if (manifest.folderName && manifest.folderName !== 'unknown' && !manifest.folderName.toLowerCase().startsWith('nexus_') && !/^[0-9a-fA-F-]{8,}$/.test(manifest.folderName)) {
      cleanName = manifest.folderName;
    } else if (analysis.nexusInfo?.name) {
      cleanName = analysis.nexusInfo.name;
    } else {
      const rawStem = analysis.zipPath.split(/[/\\]/).pop() || '';
      const candidate = getCleanNameFromFilename(rawStem);
      if (candidate && !/^[0-9a-fA-F-]{8,}$/.test(candidate) && !candidate.toLowerCase().startsWith('nexus_')) {
        cleanName = candidate;
      } else if (manifest.folderName && !/^[0-9a-fA-F-]{8,}$/.test(manifest.folderName)) {
        cleanName = manifest.folderName;
      }
    }
  }

  confirmBtn.disabled = false;
  statusEl.textContent = '';

  // Update banner if existing mod found
  let updateHtml = '';
  if (existingMod) {
    setPendingUpdateModId(existingMod.id);
    const existingVerStr = existingMod.version && existingMod.version !== 'unknown' ? `v${existingMod.version}` : '';
    const normPath = analysis.zipPath.replace(/\\/g, '/').toLowerCase();
    const isFromLibrary = normPath.includes('/pmm_library/') || normPath.includes('/library/') || normPath.includes('mods-library');
    const isFromNexusDownload = normPath.includes('nexus_') || normPath.includes('temp') || normPath.includes('palmodmanager_');

    let sourceBadge = '';
    if (isFromLibrary) {
      sourceBadge = `<span class="update-source-badge local" style="background:rgba(46, 204, 113, 0.15);color:#2ecc71;border:1px solid rgba(46, 204, 113, 0.35);font-size:9px;font-weight:700;padding:2px 6px;border-radius:4px;display:inline-flex;align-items:center;gap:4px;text-transform:uppercase;letter-spacing:0.4px;">📦 ${escapeHtml(t('installer.source_local_library'))}</span>`;
    } else if (isFromNexusDownload || analysis.nexusModId) {
      sourceBadge = `<span class="update-source-badge remote" style="background:rgba(0, 188, 255, 0.15);color:#00bcff;border:1px solid rgba(0, 188, 255, 0.35);font-size:9px;font-weight:700;padding:2px 6px;border-radius:4px;display:inline-flex;align-items:center;gap:4px;text-transform:uppercase;letter-spacing:0.4px;">⚡ ${escapeHtml(t('installer.source_nexus_download'))}</span>`;
    } else {
      sourceBadge = `<span class="update-source-badge custom" style="background:rgba(255, 255, 255, 0.08);color:var(--text-muted);border:1px solid var(--border);font-size:9px;font-weight:700;padding:2px 6px;border-radius:4px;display:inline-flex;align-items:center;gap:4px;text-transform:uppercase;letter-spacing:0.4px;">📁 ${escapeHtml(t('installer.source_custom_file'))}</span>`;
    }

    updateHtml = `
      <div class="update-banner" id="update-banner" style="margin-bottom:12px;padding:8px 12px;background:rgba(0,188,255,0.08);border:1px solid rgba(0,188,255,0.25);border-radius:6px;display:flex;align-items:center;justify-content:space-between;gap:10px;">
        <div style="display:flex;flex-direction:column;gap:4px;flex:1;overflow:hidden;">
          <span class="update-banner-text" style="font-size:11px;font-weight:600;color:var(--text-primary);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;" title="${escapeHtml(t('installer.already_exists', { name: existingMod.name, version: existingVerStr }))}">${escapeHtml(t('installer.already_exists', { name: existingMod.name, version: existingVerStr }))}</span>
          <div>${sourceBadge}</div>
        </div>
        <div style="display:flex;gap:4px;background:var(--bg-primary);padding:2px;border-radius:5px;border:1px solid var(--border);flex-shrink:0;">
          <button class="update-mode-btn" id="update-mode-btn" type="button" style="padding:4px 8px;background:#00bcff;color:#fff;border:none;border-radius:3px;font-size:11px;font-weight:600;cursor:pointer;transition:all 0.15s ease;">${escapeHtml(t('installer.mode_update'))}</button>
          <button class="update-mode-btn" id="install-new-mode-btn" type="button" style="padding:4px 8px;background:transparent;color:var(--text-secondary);border:none;border-radius:3px;font-size:11px;font-weight:600;cursor:pointer;transition:all 0.15s ease;">${escapeHtml(t('installer.mode_new'))}</button>
        </div>
      </div>
    `;
    confirmBtn.textContent = t('installer.btn_update');
  } else {
    setPendingUpdateModId(null);
    confirmBtn.textContent = t('installer.btn_install');
  }

  const picUrl = analysis.nexusInfo?.pictureUrl || (analysis.nexusInfo as any)?.picture_url || '';
  let versionVal = analysis.modinfo?.version || '';
  if (!versionVal && analysis.detectedVersion && !/^[0-9a-fA-F-]{6,}$/.test(analysis.detectedVersion.trim()) && analysis.detectedVersion.trim() !== '1.0.0' && analysis.detectedVersion.trim() !== 'unknown') {
    versionVal = analysis.detectedVersion;
  }
  if (!versionVal && analysis.nexusInfo?.version && analysis.nexusInfo.version !== '1.0.0' && analysis.nexusInfo.version !== 'unknown') {
    versionVal = analysis.nexusInfo.version;
  }
  if (!versionVal && analysis.detectedVersion && !/^[0-9a-fA-F-]{6,}$/.test(analysis.detectedVersion.trim())) {
    versionVal = analysis.detectedVersion;
  }
  if (!versionVal) {
    versionVal = '1.0.0';
  }

  const displayType = manifest.modType === 'hybrid' ? `Hybrid (${[manifest.hasUe4ss ? 'UE4SS' : '', manifest.hasPalschema ? 'PalSchema' : '', manifest.hasPak ? 'Pak' : ''].filter(Boolean).join(' + ')})` : manifest.modType.toUpperCase();
  const isLogicModsDefault = manifest.modType === 'logicmods' || manifest.routes.some((r: any) => r.routeType === 'logicmods');

  let pakDestHtml = `
    <div class="pak-dest-section" id="single-pak-dest-section" style="display: ${manifest.hasPak ? 'block' : 'none'}; margin-top:8px;">
      <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;display:block;margin-bottom:6px;">${escapeHtml(t('installer.pak_dest_title'))}</label>
      <div class="pak-dest-options" style="display:flex;gap:12px;">
        <label class="pak-dest-option" style="display:flex;align-items:center;gap:6px;font-size:12px;cursor:pointer;">
          <input type="radio" name="pak-dest" value="~mods" ${isLogicModsDefault ? '' : 'checked'} />
          <span>${escapeHtml(t('installer.pak_dest_res'))}</span>
        </label>
        <label class="pak-dest-option" style="display:flex;align-items:center;gap:6px;font-size:12px;cursor:pointer;">
          <input type="radio" name="pak-dest" value="logicmods" ${isLogicModsDefault ? 'checked' : ''} />
          <span>${escapeHtml(t('installer.pak_dest_logic'))}</span>
        </label>
      </div>
    </div>
  `;

  const deps = getState().dependencies;
  const allMods = getState().allMods;
  const isAltermatic = manifest.modType === 'altermatic' || (analysis as any).hasAltermatic || (analysis as any).detectedType === 'altermatic';

  const isSelfAltermatic = analysis.nexusModId === 1626
    || cleanName.toLowerCase().includes('altermatic')
    || manifest.folderName.toLowerCase().includes('altermatic')
    || analysis.zipPath.toLowerCase().includes('altermatic');

  const isSelfUniPalUI = analysis.nexusModId === 1894
    || cleanName.toLowerCase().includes('unipalui')
    || manifest.folderName.toLowerCase().includes('unipalui')
    || analysis.zipPath.toLowerCase().includes('unipalui');

  const isAltermaticPresent = Boolean(
    deps?.altermatic_installed ||
    allMods.some(m => m.enabled && (m.nexusModId === 1626 || m.name.toLowerCase().includes('altermatic') || m.id.toLowerCase().includes('altermatic')))
  );

  const isUniPalUIPresent = Boolean(
    deps?.unipalui_installed ||
    allMods.some(m => m.enabled && (m.nexusModId === 1894 || m.name.toLowerCase().includes('unipalui') || m.id.toLowerCase().includes('unipalui')))
  );

  const missingAltermatic = isAltermatic && !isSelfAltermatic && !isAltermaticPresent;
  const missingUniPalUI = isAltermatic && !isSelfAltermatic && !isSelfUniPalUI && !isUniPalUIPresent;

  let altermaticAlertHtml = '';
  if (missingAltermatic) {
    altermaticAlertHtml += `
      <div style="background: rgba(255, 118, 117, 0.12); border: 1px solid rgba(255, 118, 117, 0.35); border-radius: 6px; padding: 7px 10px; display: flex; align-items: center; justify-content: space-between; gap: 10px; margin-top: 2px;">
        <div style="display: flex; align-items: center; gap: 8px;">
          <span style="font-size: 14px;">⚠️</span>
          <div style="display: flex; flex-direction: column; text-align: left;">
            <span style="font-size: 11px; font-weight: bold; color: var(--type-altermatic);">${escapeHtml(t('installer.altermatic_missing_title'))}</span>
            <span style="font-size: 9.5px; color: var(--text-secondary);">${escapeHtml(t('installer.altermatic_missing_desc'))}</span>
          </div>
        </div>
        <button id="open-altermatic-nexus-btn" type="button" class="btn btn-secondary" style="font-size: 10px; padding: 3px 8px; border-color: var(--type-altermatic); color: var(--type-altermatic); white-space: nowrap; height: auto; margin: 0;">${escapeHtml(t('installer.btn_get_altermatic'))}</button>
      </div>
    `;
  }
  if (missingUniPalUI) {
    altermaticAlertHtml += `
      <div style="background: rgba(0, 188, 255, 0.08); border: 1px solid rgba(0, 188, 255, 0.25); border-radius: 6px; padding: 7px 10px; display: flex; align-items: center; justify-content: space-between; gap: 10px; margin-top: 2px;">
        <div style="display: flex; align-items: center; gap: 8px;">
          <span style="font-size: 14px;">ℹ️</span>
          <div style="display: flex; flex-direction: column; text-align: left;">
            <span style="font-size: 11px; font-weight: bold; color: var(--type-ue4ss);">${escapeHtml(t('installer.unipalui_recommended_title'))}</span>
            <span style="font-size: 9.5px; color: var(--text-secondary);">${escapeHtml(t('installer.unipalui_recommended_desc'))}</span>
          </div>
        </div>
        <button id="open-unipalui-nexus-btn" type="button" class="btn btn-secondary" style="font-size: 10px; padding: 3px 8px; border-color: var(--type-ue4ss); color: var(--type-ue4ss); white-space: nowrap; height: auto; margin: 0;">${escapeHtml(t('installer.btn_get_unipalui'))}</button>
      </div>
    `;
  }

  content.innerHTML = `
    <div style="display:flex;gap:18px;align-items:flex-start;padding:2px 0;">
       <!-- Left Column: Card Preview (Nexus Info or Local Modinfo) -->
       ${analysis.nexusInfo ? `
       <div style="width:230px;min-width:230px;max-width:230px;flex-shrink:0;background:var(--bg-secondary);border:1px solid var(--border);border-radius:8px;overflow:hidden;display:flex;flex-direction:column;box-shadow:0 4px 15px rgba(0,0,0,0.35);">
          <div style="position:relative;width:100%;height:120px;overflow:hidden;background:#000;">
             ${picUrl ? `<img src="${escapeHtml(picUrl)}" data-original-src="${escapeHtml(picUrl)}" style="width:100%;height:100%;object-fit:cover;opacity:0.85;" alt="" onerror="window.handleUniversalImageFallback ? window.handleUniversalImageFallback(this) : (this.onerror=null, this.style.display='none', this.nextElementSibling && (this.nextElementSibling.style.display='flex'));" /><div style="display:none;align-items:center;justify-content:center;height:100%;color:var(--text-muted);font-weight:bold;font-size:32px;">N</div>` : `<div style="display:flex;align-items:center;justify-content:center;height:100%;color:var(--text-muted);font-weight:bold;font-size:32px;">N</div>`}
             <div style="position:absolute;bottom:6px;right:6px;background:rgba(0,0,0,0.75);padding:2px 7px;border-radius:10px;font-size:9px;color:#00ffcc;font-weight:700;letter-spacing:0.5px;">
                ${analysis.nexusInfo.downloads.toLocaleString()} DLs
             </div>
          </div>
          <div style="padding:10px;display:flex;flex-direction:column;gap:5px;">
             <div style="font-size:12.5px;font-weight:700;color:var(--text-primary);line-height:1.3;word-break:break-word;">${escapeHtml(analysis.nexusInfo.name)}</div>
             <div style="font-size:9.5px;color:var(--text-muted)">${escapeHtml(t('installer.by_author', { author: analysis.nexusInfo.author }))}</div>
             <div style="font-size:10.5px;color:var(--text-secondary);line-height:1.4;margin-top:2px;display:-webkit-box;-webkit-line-clamp:3;-webkit-box-orient:vertical;overflow:hidden;">${escapeHtml(analysis.nexusInfo.summary)}</div>
          </div>
       </div>
       ` : (analysis.modinfo ? `
       <div style="width:230px;min-width:230px;max-width:230px;flex-shrink:0;background:var(--bg-secondary);border:1px solid var(--border);border-radius:8px;overflow:hidden;display:flex;flex-direction:column;box-shadow:0 4px 15px rgba(0,0,0,0.35);">
          <div style="position:relative;width:100%;height:120px;overflow:hidden;background:var(--bg-primary);display:flex;align-items:center;justify-content:center;border-bottom:1px solid var(--border);">
             <div style="font-size:38px;color:var(--accent);">🛠</div>
             <div style="position:absolute;bottom:6px;right:6px;background:rgba(0,0,0,0.75);padding:2px 7px;border-radius:10px;font-size:9px;color:var(--accent);font-weight:700;letter-spacing:0.5px;text-transform:uppercase;">
                ${escapeHtml(t('installer.local_package'))}
             </div>
          </div>
          <div style="padding:10px;display:flex;flex-direction:column;gap:5px;">
             <div style="font-size:12.5px;font-weight:700;color:var(--text-primary);line-height:1.3;word-break:break-word;">${escapeHtml(analysis.modinfo.name || cleanName)}</div>
             <div style="font-size:9.5px;color:var(--text-muted)">${escapeHtml(t('installer.by_author', { author: analysis.modinfo.author || t('common.unknown') }))}</div>
             <div style="font-size:10.5px;color:var(--text-secondary);line-height:1.4;margin-top:2px;display:-webkit-box;-webkit-line-clamp:3;-webkit-box-orient:vertical;overflow:hidden;">${escapeHtml(analysis.modinfo.description || t('installer.no_description'))}</div>
          </div>
       </div>
       ` : '')}

        <!-- Right Column: Settings Form -->
        <div style="flex:1;min-width:0;display:flex;flex-direction:column;gap:10px;">
           ${updateHtml}
           ${altermaticAlertHtml}

           <div id="config-diff-container" style="display: none; border: 1px solid rgba(0, 188, 255, 0.25); background: rgba(0, 40, 60, 0.15); border-radius: 6px; padding: 6px 10px; margin-top: -2px; margin-bottom: 2px; align-items: center; justify-content: space-between; gap: 12px;">
              <div style="display: flex; align-items: center; gap: 8px;">
                <span style="font-size: 14px;">⚙</span>
                <div style="display: flex; flex-direction: column; text-align: left;">
                   <span style="font-size: 11px; font-weight: bold; color: var(--text-primary);">${escapeHtml(t('installer.config_merge_title'))}</span>
                   <span id="config-diff-summary-text" style="font-size: 9px; color: var(--text-muted);">${escapeHtml(t('installer.config_merge_desc'))}</span>
                </div>
              </div>
              <button id="view-config-diff-btn" class="btn btn-secondary" style="font-size: 10px; padding: 4px 8px; height: auto; line-height: 1; margin: 0;">${escapeHtml(t('installer.btn_show_details'))}</button>
           </div>
           
           <div style="display:flex;gap:12px;">
             <div style="flex:1;display:flex;flex-direction:column;gap:4px;">
                <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">${escapeHtml(t('installer.lbl_mod_name'))}</label>
                <input type="text" id="mod-name-input" value="${escapeHtml(existingMod ? existingMod.name : cleanName)}" style="width:100%;padding:7px 10px;background:var(--bg-secondary);color:var(--text-primary);border:1px solid var(--border);border-radius:4px;font-size:12px;font-weight:600;" />
             </div>
             <div style="flex:1;display:flex;flex-direction:column;gap:4px;">
                <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">${escapeHtml(t('installer.lbl_folder_name'))}</label>
                <input type="text" id="mod-folder-name-input" value="${escapeHtml(manifest.folderName)}" disabled style="width:100%;padding:7px 10px;background:var(--bg-primary);color:var(--text-muted);border:1px solid var(--border);border-radius:4px;font-size:12px;font-weight:600;cursor:not-allowed;" />
             </div>
           </div>

           <div style="display:flex;gap:12px;">
              <div style="flex:1;display:flex;flex-direction:column;gap:6px;">
                 <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">${escapeHtml(t('installer.lbl_detected_type'))}</label>
                 <input type="text" value="${escapeHtml(displayType)}" disabled style="width:100%;padding:8px 12px;background:var(--bg-primary);color:var(--text-muted);border:1px solid var(--border);border-radius:4px;font-size:12px;font-weight:600;cursor:not-allowed;" />
              </div>
              <div style="width:120px;display:flex;flex-direction:column;gap:6px;">
                 <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">${escapeHtml(t('installer.lbl_version'))}</label>
                 <input type="text" id="mod-version-input" value="${escapeHtml(versionVal)}" style="width:100%;padding:8px 12px;background:var(--bg-secondary);color:var(--text-primary);border:1px solid var(--border);border-radius:4px;font-size:12px;text-align:center;" />
              </div>
           </div>

           <div style="display:flex;flex-direction:column;gap:6px;">
              <div style="display:flex;justify-content:space-between;align-items:center;">
                 <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">${escapeHtml(t('installer.lbl_files_to_install'))}</label>
                 <button id="view-all-files-btn" class="btn btn-secondary" style="font-size:10px;padding:2px 6px;height:auto;line-height:1;margin:0;">${escapeHtml(t('installer.btn_show_full_list'))}</button>
              </div>
              <div class="manifest-files-list" style="max-height:85px;overflow-y:auto;background:var(--bg-primary);border:1px solid var(--border);border-radius:4px;padding:6px;font-family:monospace;font-size:10px;display:flex;flex-direction:column;gap:4px;">
                ${manifest.routes.map((r: any) => `
                  <div style="display:flex;justify-content:space-between;align-items:center;padding:2px 4px;border-radius:2px;background:rgba(255,255,255,0.02);">
                    <span style="color:var(--text-primary);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:180px;" title="${escapeHtml(r.zipPath)}">${escapeHtml(r.zipPath)}</span>
                    <span style="font-size:8px;padding:1px 3px;border-radius:3px;background:var(--bg-secondary);color:var(--accent);border:1px solid var(--border);text-transform:uppercase;">${r.routeType}</span>
                  </div>
                `).join('')}
              </div>
           </div>

           ${pakDestHtml}
        </div>
    </div>
  `;

  // Wire up Altermatic & UniPalUI buttons
  document.getElementById('open-altermatic-nexus-btn')?.addEventListener('click', (e) => {
    e.preventDefault();
    openUrl('https://www.nexusmods.com/palworld/mods/1626');
  });
  document.getElementById('open-unipalui-nexus-btn')?.addEventListener('click', (e) => {
    e.preventDefault();
    openUrl('https://www.nexusmods.com/palworld/mods/1894');
  });

  // Wire up Show Full List button
  const viewAllBtn = installerDom.elMaybe('view-all-files-btn');
  if (viewAllBtn) {
    viewAllBtn.addEventListener('click', (e) => {
      e.preventDefault();
      showFileTreeModal(manifest.routes, cleanName, analysis.zipPath);
    });
  }

  const pakDestRadios = document.querySelectorAll('input[name="pak-dest"]');
  pakDestRadios.forEach(radio => {
    radio.addEventListener('change', async (e) => {
      const selectedDest = (e.target as HTMLInputElement).value;
      try {
        const newManifest = await buildInstallManifest(
          analysis.zipPath,
          getState().currentSettings?.gamePath || '',
          selectedDest,
          cleanName
        );
        const filesListContainer = document.querySelector('.manifest-files-list');
        if (filesListContainer) {
          filesListContainer.innerHTML = newManifest.routes.map((r: any) => `
            <div style="display:flex;justify-content:space-between;align-items:center;padding:2px 4px;border-radius:2px;background:rgba(255,255,255,0.02);">
              <span style="color:var(--text-primary);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:180px;" title="${escapeHtml(r.zipPath)}">${escapeHtml(r.zipPath)}</span>
              <span style="font-size:8px;padding:1px 3px;border-radius:3px;background:var(--bg-secondary);color:var(--accent);border:1px solid var(--border);text-transform:uppercase;">${r.routeType}</span>
            </div>
          `).join('');
        }
        manifest = newManifest;
      } catch (err) {
        console.error("Failed to rebuild manifest on pak dest change:", err);
      }
    });
  });

  // Wire up update/install-new mode toggle buttons
  const updateModeBtn = installerDom.elMaybe('update-mode-btn');
  const installNewModeBtn = installerDom.elMaybe('install-new-mode-btn');
  const folderInput = installerDom.elMaybe('mod-folder-name-input');

  function setInstallMode(isUpdate: boolean) {
    if (!existingMod) return;
    if (isUpdate) {
      setPendingUpdateModId(existingMod.id);
      confirmBtn.textContent = t('installer.btn_update');
      if (updateModeBtn) {
        updateModeBtn.style.background = '#00bcff';
        updateModeBtn.style.color = '#fff';
      }
      if (installNewModeBtn) {
        installNewModeBtn.style.background = 'transparent';
        installNewModeBtn.style.color = 'var(--text-secondary)';
      }
      if (folderInput) {
        folderInput.value = manifest.folderName;
        folderInput.disabled = true;
        folderInput.style.cursor = 'not-allowed';
      }
    } else {
      setPendingUpdateModId(null);
      confirmBtn.textContent = t('installer.btn_install');
      if (updateModeBtn) {
        updateModeBtn.style.background = 'transparent';
        updateModeBtn.style.color = 'var(--text-secondary)';
      }
      if (installNewModeBtn) {
        installNewModeBtn.style.background = '#00bcff';
        installNewModeBtn.style.color = '#fff';
      }
      if (folderInput) {
        folderInput.disabled = false;
        folderInput.style.cursor = 'text';
        if (folderInput.value === manifest.folderName) {
          folderInput.value = `${manifest.folderName}_New`;
        }
      }
    }
  }

  if (updateModeBtn) {
    updateModeBtn.addEventListener('click', (e) => {
      e.preventDefault();
      setInstallMode(true);
    });
  }
  if (installNewModeBtn) {
    installNewModeBtn.addEventListener('click', (e) => {
      e.preventDefault();
      setInstallMode(false);
    });
  }

  if (existingMod) {
    previewConfigDiff(analysis.zipPath, existingMod.id).then(diffs => {
      const diffContainer = installerDom.elMaybe('config-diff-container');
      const summaryText = installerDom.elMaybe('config-diff-summary-text');
      const viewBtn = installerDom.elMaybe('view-config-diff-btn');
      if (diffContainer && diffs && diffs.length > 0) {
        diffContainer.style.display = 'flex';
        if (summaryText) {
          const filesCount = diffs.length;
          summaryText.textContent = `Differences detected in ${filesCount} config file${filesCount === 1 ? '' : 's'}.`;
        }
        if (viewBtn) {
          viewBtn.onclick = (e) => {
            e.preventDefault();
            showConfigDiffModal(diffs, existingMod.id);
          };
        }
      }
    }).catch(err => {
      console.error("Failed to preview config diffs:", err);
    });
  }

  confirmBtn.disabled = false;
}

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
        <h3 style="margin:0; font-size:16px; font-weight:700; color:var(--text-primary);">⚙ Config Settings Merge Preview</h3>
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
            <span>🟢 Your changes to preserve</span>
            <span style="font-size:9px; padding:1px 5px; border-radius:10px; background:rgba(255,144,0,0.15); border:1px solid rgba(255,144,0,0.25);">${diff.keys_user_changed.length}</span>
          </div>
          <div style="overflow-x:auto; background:var(--bg-primary); border:1px solid var(--border); border-radius:4px; padding:4px;">
            <table style="width:100%; border-collapse:collapse; font-size:10px; text-align:left; font-family:monospace;">
              <thead>
                <tr style="border-bottom:1px solid var(--border); color:var(--text-muted);">
                  <th style="padding:6px 8px; font-weight:bold; width: 60px;">Preserve</th>
                  <th style="padding:6px 8px; font-weight:bold;">Setting / Key</th>
                  <th style="padding:6px 8px; font-weight:bold; width:150px; text-align:right;">Your Value</th>
                  <th style="padding:6px 8px; font-weight:bold; width:150px; text-align:right;">Default Value</th>
                </tr>
              </thead>
              <tbody>
                ${diff.keys_user_changed.map((c: any) => {
        const isPreserved = !currentIgnoredKeys.includes(c.key);
        return `
                    <tr style="border-bottom:1px solid rgba(255,255,255,0.02); hover:background:rgba(255,255,255,0.01);">
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
            <span>🔵 New settings added by author</span>
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
            <span>🔴 Settings removed by author</span>
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

      setModIgnoredKeys(modId, localIgnoredKeys).then(updatedMod => {
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
