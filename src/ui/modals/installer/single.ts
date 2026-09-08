import { convertFileSrc } from '@tauri-apps/api/core';
import {
  buildInstallManifest,
  fetchNexusInfoAsync,
  previewConfigDiff,
  previewArchivedConfigDiff,
  openUrl
} from '../../../api';
import type { ZipAnalysis, InstallManifest } from '../../../api';
import { getState, updateState } from '../../../state';
import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import { getCleanNameFromFilename } from './helpers';
import { showFileTreeModal } from './fileTree';
import { setPendingUpdateModId } from './state';
import { showConfigDiffModal, showArchivedConfigDiffModal } from './diffModal';
import { installerDom } from '../../../framework';

let _archivedIgnoredFiles: string[] = [];
let _archivedIgnoredKeys: string[] = [];

export function getArchivedIgnoredSettings(): { files: string[], keys: string[] } {
  return { files: _archivedIgnoredFiles, keys: _archivedIgnoredKeys };
}

export async function renderInstallPreview(analysis: ZipAnalysis, existingMod: { id: string; name: string, version: string } | null = null): Promise<void> {
  _archivedIgnoredFiles = [];
  _archivedIgnoredKeys = [];
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

  let archivedConfigHtml = '';
  let currentArchivedInfo: any = null;
  if (!existingMod) {
    try {
      const { checkArchivedConfig } = await import('../../../api');
      const archivedInfo = await checkArchivedConfig(analysis.nexusModId || null, cleanName);
      if (archivedInfo && archivedInfo.files.length > 0) {
        currentArchivedInfo = archivedInfo;
        archivedConfigHtml = `
          <div class="archived-config-banner" style="margin-bottom:6px;padding:8px 12px;background:rgba(46,204,113,0.08);border:1px solid rgba(46,204,113,0.3);border-radius:6px;display:flex;align-items:center;justify-content:space-between;gap:10px;">
            <div style="display:flex;align-items:center;gap:8px;flex:1;min-width:0;">
              <span style="font-size:16px;">💾</span>
              <div style="display:flex;flex-direction:column;min-width:0;">
                <span style="font-size:11px;font-weight:600;color:var(--text-primary);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">${escapeHtml(t('installer.archived_config_detected', { count: archivedInfo.files.length }))}</span>
                <span style="font-size:10px;color:var(--text-muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;" title="${escapeHtml(archivedInfo.files.join(', '))}">${escapeHtml(archivedInfo.files.join(', '))}</span>
              </div>
            </div>
            <div style="display:flex;align-items:center;gap:8px;flex-shrink:0;">
              <button id="review-archived-config-btn" type="button" class="btn btn-secondary" style="font-size: 10px; padding: 3px 8px; border-color: rgba(46, 204, 113, 0.5); color: #2ecc71; white-space: nowrap; height: auto; margin: 0;">⚙ ${escapeHtml(t('installer.btn_review_archived_config') || 'Review')}</button>
              <label style="display:flex;align-items:center;gap:6px;font-size:11px;font-weight:600;color:#2ecc71;cursor:pointer;flex-shrink:0;">
                <input type="checkbox" id="restore-archived-config-checkbox" data-archive-id="${escapeHtml(archivedInfo.archiveId)}" checked style="accent-color:#2ecc71;cursor:pointer;" />
                <span>${escapeHtml(t('installer.restore_archived_config'))}</span>
              </label>
            </div>
          </div>
        `;
      }
    } catch { }
  }

  const rawPic = analysis.nexusInfo?.pictureUrl || (analysis.nexusInfo as any)?.picture_url || '';
  const picUrl = (rawPic && !rawPic.startsWith('http://') && !rawPic.startsWith('https://') && !rawPic.startsWith('asset://'))
    ? convertFileSrc(rawPic)
    : rawPic;
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
             ${picUrl ? `<img src="${escapeHtml(picUrl)}" data-original-src="${escapeHtml(picUrl)}" style="width:100%;height:100%;object-fit:cover;opacity:0.85;" alt="" onerror="window.handleUniversalImageFallback ? window.handleUniversalImageFallback(this) : (this.onerror=null, this.style.display='none', this.nextElementSibling && (this.nextElementSibling.style.display='flex'));" /><div style="display:none;align-items:center;justify-content:center;height:100%;color:var(--text-muted);font-weight:bold;font-size:32px;">${analysis.nexusInfo.isWorkshop ? 'W' : 'N'}</div>` : `<div style="display:flex;align-items:center;justify-content:center;height:100%;color:var(--text-muted);font-weight:bold;font-size:32px;">${analysis.nexusInfo.isWorkshop ? 'W' : 'N'}</div>`}
             <div style="position:absolute;bottom:6px;right:6px;background:rgba(0,0,0,0.75);padding:2px 7px;border-radius:10px;font-size:9px;color:${analysis.nexusInfo.isWorkshop ? '#ff9d00' : '#00ffcc'};font-weight:700;letter-spacing:0.5px;text-transform:uppercase;">
                ${analysis.nexusInfo.isWorkshop ? `WORKSHOP${analysis.nexusInfo.modId ? ` (ID: ${analysis.nexusInfo.modId})` : ''}` : `${analysis.nexusInfo.downloads.toLocaleString()} DLs`}
             </div>
          </div>
          <div style="padding:10px;display:flex;flex-direction:column;gap:5px;">
             <div style="font-size:12.5px;font-weight:700;color:var(--text-primary);line-height:1.3;word-break:break-word;">${escapeHtml(analysis.nexusInfo.name)}</div>
             <div style="font-size:9.5px;color:var(--text-muted)">${escapeHtml(t('installer.by_author', { author: analysis.nexusInfo.author || t('common.unknown') }))}</div>
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
           ${archivedConfigHtml}
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
                    <span style="color:var(--text-primary);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex:1;min-width:0;margin-right:8px;" title="${escapeHtml(r.zipPath)}">${escapeHtml(r.zipPath)}</span>
                    <span style="font-size:8px;padding:1px 3px;border-radius:3px;background:var(--bg-secondary);color:var(--accent);border:1px solid var(--border);text-transform:uppercase;flex-shrink:0;">${r.routeType}</span>
                  </div>
                `).join('')}
              </div>
           </div>

           ${pakDestHtml}
        </div>
    </div>
  `;

  // Wire up Altermatic & UniPalUI buttons
  installerDom.elMaybe('open-altermatic-nexus-btn')?.addEventListener('click', (e) => {
    e.preventDefault();
    openUrl('https://www.nexusmods.com/palworld/mods/1626');
  });
  installerDom.elMaybe('open-unipalui-nexus-btn')?.addEventListener('click', (e) => {
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

  const reviewArchivedBtn = installerDom.elMaybe('review-archived-config-btn');
  if (reviewArchivedBtn && currentArchivedInfo) {
    reviewArchivedBtn.addEventListener('click', async (e) => {
      e.preventDefault();
      try {
        const diffs = await previewArchivedConfigDiff(analysis.zipPath, currentArchivedInfo.archiveId);
        showArchivedConfigDiffModal(
          diffs,
          _archivedIgnoredFiles,
          _archivedIgnoredKeys,
          (newIgnoredFiles, newIgnoredKeys) => {
            _archivedIgnoredFiles = newIgnoredFiles;
            _archivedIgnoredKeys = newIgnoredKeys;
          }
        );
      } catch (err) {
        console.error("Failed to preview archived config diff:", err);
      }
    });
  }

  confirmBtn.disabled = false;
}

