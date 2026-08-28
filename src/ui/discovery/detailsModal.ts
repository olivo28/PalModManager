import {
  getDiscoveryModDetails,
  installDiscoveryFile,
  endorseNexusMod,
  abstainNexusMod,
  trackNexusMod,
  untrackNexusMod,
  openUrl,
  type DiscoveryModItem,
  type DiscoveryModDetails,
  type DiscoveryFileItem,
} from '../../api';
import { enqueueDiscoveryDownload } from '../../features/nxm_queue';
import { getState } from '../../state';
import { showToast } from '../toast';
import { t } from '../../utils/i18n';
import { escapeHtml } from '../../utils/helpers';
import { descriptionToHtml } from '../../utils/bbcode';
import logoUrl from '../../assets/logo.png';
import { discState, isUserPremium } from './state';
import { handleDiscoveryImageError } from './helpers';
import { openLightbox } from './lightbox';
import { discoveryDom } from '../../framework';

export async function openModDetails(
  modId: number,
  autoInstallFirstPrimary: boolean = false,
  previewData?: Partial<DiscoveryModItem>
): Promise<void> {
  const modal = discoveryDom.elMaybe('discovery-mod-modal');
  if (!modal) return;

  modal.classList.add('visible');
  modal.classList.add('active');
  modal.style.display = 'flex';

  resetModalUI(previewData);

  try {
    const details = await getDiscoveryModDetails(modId);
    discState.currentModalMod = details;
    populateModalData(details);

    if (autoInstallFirstPrimary && details.files.length > 0) {
      if (isUserPremium()) {
        // Switch to files tab
        const filesTabBtn = discoveryDom.query('.discovery-modal-tab[data-tab="files"]');
        filesTabBtn?.click();

        const primary = details.files.find((f) => f.isPrimary || f.categoryName === 'MAIN') || details.files[0];
        if (primary) {
          await executeInstallFile(details, primary);
        }
      } else {
        openUrl(`https://www.nexusmods.com/palworld/mods/${details.modId}?tab=files`);
      }
    }
  } catch (err: any) {
    console.error('[Discovery] Failed to load mod details:', err);
    showToast(t('discovery.details_load_failed', { error: String(err) }), 'error');
  }
}

export function resetModalUI(previewData?: Partial<DiscoveryModItem>): void {
  const title = discoveryDom.elMaybe('discovery-modal-title');
  const author = discoveryDom.elMaybe('discovery-modal-author');
  const cat = discoveryDom.elMaybe('discovery-modal-category');
  const ver = discoveryDom.elMaybe('discovery-modal-version');
  const installedBadge = discoveryDom.elMaybe('discovery-modal-installed-badge');
  const updated = discoveryDom.elMaybe('discovery-modal-updated');
  const endorsements = discoveryDom.elMaybe('discovery-modal-endorsements');
  const downloads = discoveryDom.elMaybe('discovery-modal-downloads');
  const desc = discoveryDom.elMaybe('discovery-modal-description');
  const filesList = discoveryDom.elMaybe('discovery-files-list');
  const gallery = discoveryDom.elMaybe('discovery-media-gallery');
  const img = discoveryDom.elMaybe('discovery-modal-img');

  if (installedBadge) {
    installedBadge.style.display = 'none';
    installedBadge.textContent = '';
  }

  if (previewData) {
    if (title) title.textContent = previewData.name || 'Loading...';
    if (author) author.textContent = previewData.author || '--';
    if (cat) cat.textContent = previewData.categoryName || 'Mod';
    if (ver) ver.textContent = previewData.version ? `v${previewData.version}` : '';
    if (updated) updated.textContent = previewData.updatedAt ? new Date(previewData.updatedAt).toLocaleDateString() : '';
    if (endorsements) endorsements.textContent = (previewData.endorsements || 0).toLocaleString();
    if (downloads) downloads.textContent = (previewData.downloads || 0).toLocaleString();
    if (img) {
      if (previewData.pictureUrl && previewData.pictureUrl.trim() !== '') {
        img.src = previewData.pictureUrl;
        img.onerror = () => { handleDiscoveryImageError(img, previewData.pictureUrl!); };
      } else {
        img.src = logoUrl;
      }
    }
  } else {
    if (title) title.textContent = 'Loading Mod Details...';
    if (author) author.textContent = '--';
    if (cat) cat.textContent = '--';
    if (ver) ver.textContent = '--';
    if (updated) updated.textContent = '';
    if (endorsements) endorsements.textContent = '0';
    if (downloads) downloads.textContent = '0';
  }

  if (desc) desc.innerHTML = '<div class="loading-spinner"></div>';
  if (filesList) filesList.innerHTML = '<div class="loading-spinner"></div>';
  if (gallery) gallery.innerHTML = '<div class="loading-spinner"></div>';

  // Switch to Description tab by default
  discoveryDom.queryAll('.discovery-modal-tab').forEach((b) => b.classList.remove('active'));
  discoveryDom.query('.discovery-modal-tab[data-tab="desc"]')?.classList.add('active');

  discoveryDom.queryAll('.discovery-tab-pane').forEach((p) => {
    (p as HTMLElement).style.display = 'none';
    p.classList.remove('active');
  });
  const descPane = discoveryDom.elMaybe('discovery-tab-desc');
  if (descPane) {
    descPane.style.display = 'block';
    descPane.classList.add('active');
  }
}

export function populateModalData(details: DiscoveryModDetails): void {
  const title = discoveryDom.elMaybe('discovery-modal-title');
  const author = discoveryDom.elMaybe('discovery-modal-author');
  const cat = discoveryDom.elMaybe('discovery-modal-category');
  const ver = discoveryDom.elMaybe('discovery-modal-version');
  const installedBadge = discoveryDom.elMaybe('discovery-modal-installed-badge');
  const updated = discoveryDom.elMaybe('discovery-modal-updated');
  const endorsements = discoveryDom.elMaybe('discovery-modal-endorsements');
  const downloads = discoveryDom.elMaybe('discovery-modal-downloads');
  const desc = discoveryDom.elMaybe('discovery-modal-description');
  const filesList = discoveryDom.elMaybe('discovery-files-list');
  const gallery = discoveryDom.elMaybe('discovery-media-gallery');
  const img = discoveryDom.elMaybe('discovery-modal-img');

  if (title) title.textContent = details.name;
  if (author) author.textContent = details.author;
  if (cat) cat.textContent = details.categoryName || 'Mod';
  if (ver) ver.textContent = details.version ? `v${details.version}` : '';

  // Check if mod is currently installed in PMM
  const allMods = getState().allMods || [];
  const normDetailsName = details.name.toLowerCase().replace(/[^a-z0-9]/g, '');
  const installedMod = allMods.find((m) => {
    if (m.nexusModId && m.nexusModId === details.modId) return true;
    const normModName = m.name.toLowerCase().replace(/[^a-z0-9]/g, '');
    return normModName !== '' && (normModName === normDetailsName || normModName.includes(normDetailsName) || normDetailsName.includes(normModName));
  });

  if (installedBadge) {
    if (installedMod) {
      const instVer = installedMod.version && installedMod.version !== 'unknown' ? `v${installedMod.version}` : '';
      installedBadge.style.display = 'inline-flex';
      installedBadge.textContent = instVer ? `✓ ${t('discovery.status_installed')}: ${instVer}` : `✓ ${t('discovery.status_installed')}`;
      installedBadge.title = t('discovery.status_installed_title', { name: installedMod.name, version: instVer || 'unknown' });
    } else {
      installedBadge.style.display = 'none';
      installedBadge.textContent = '';
    }
  }
  if (updated) {
    const d = details.updatedAt || details.createdAt;
    updated.textContent = d ? new Date(d).toLocaleDateString() : '';
  }
  if (endorsements) endorsements.textContent = details.endorsements.toLocaleString();
  if (downloads) downloads.textContent = details.downloads.toLocaleString();

  if (img) {
    if (details.pictureUrl && details.pictureUrl.trim() !== '') {
      img.src = details.pictureUrl;
      img.onerror = () => { handleDiscoveryImageError(img, details.pictureUrl!); };
    } else {
      img.src = logoUrl;
    }
  }

  // 1. Description with iterative BBCode parser
  if (desc) {
    const rawText = details.description || details.summary || 'No description provided.';
    desc.innerHTML = descriptionToHtml(rawText);

    // Attach click-to-zoom on embedded description images
    desc.querySelectorAll('img').forEach((descImg) => {
      descImg.classList.add('cursor-zoom');
      descImg.setAttribute('data-original-src', descImg.src);
      descImg.onerror = () => {
        if ((window as any).handleUniversalImageFallback) {
          (window as any).handleUniversalImageFallback(descImg);
        }
      };
      descImg.addEventListener('click', () => {
        if (descImg.src) openLightbox(descImg.src);
      });
    });
  }

  // Update files instruction text for premium vs free
  const filesInstruction = discoveryDom.elMaybe('discovery-files-instruction-text');
  if (filesInstruction) {
    if (isUserPremium()) {
      filesInstruction.textContent = t('discovery.files_instruction');
      filesInstruction.style.color = 'var(--text-muted)';
    } else {
      filesInstruction.textContent = `ℹ️ ${t('discovery.premium_required_info')}`;
      filesInstruction.style.color = '#da8e35';
    }
  }

  // 2. Files List (Categorized, sorted newest to oldest, with archived toggle and scan badges)
  const filesContainer = discoveryDom.elMaybe('discovery-files-list');
  if (filesContainer) {
    renderFilesList(details, filesContainer);
  }

  // 3. Changelogs Tab
  const changelogsContainer = discoveryDom.elMaybe('discovery-modal-changelogs');
  if (changelogsContainer) {
    renderChangelogsList(details, changelogsContainer);
  }

  // 4. Media gallery (screenshots)
  const galleryContainer = discoveryDom.elMaybe('discovery-media-gallery');
  if (galleryContainer) {
    const allImages = details.images && details.images.length > 0
      ? details.images
      : details.pictureUrl ? [details.pictureUrl] : [];

    if (allImages.length === 0) {
      galleryContainer.innerHTML = `<p style="color: var(--text-muted); font-size: 12px;">${t('discovery.no_media')}</p>`;
    } else {
      galleryContainer.innerHTML = allImages.map((src) => `
        <div class="discovery-media-item" data-src="${escapeHtml(src)}">
          <img src="${escapeHtml(src)}" data-original-src="${escapeHtml(src)}" alt="Screenshot" class="discovery-media-img" loading="lazy" onerror="window.handleUniversalImageFallback ? window.handleUniversalImageFallback(this) : (this.parentElement.style.display='none')" />
        </div>
      `).join('');

      galleryContainer.querySelectorAll('.discovery-media-item').forEach((item) => {
        item.addEventListener('click', () => {
          const src = (item as HTMLElement).dataset.src;
          if (src) openLightbox(src);
        });
      });
    }
  }

  // Setup social action buttons
  setupModalActions(details);
}

export function renderChangelogsList(details: DiscoveryModDetails, container: HTMLElement): void {
  // Collect all changelogs from files
  const versionMap = new Map<string, string[]>();

  for (const f of details.files) {
    if (f.changelogEntries && f.changelogEntries.length > 0) {
      const v = f.version || details.version || 'Latest';
      const existing = versionMap.get(v) || [];
      for (const entry of f.changelogEntries) {
        if (!existing.includes(entry)) {
          existing.push(entry);
        }
      }
      versionMap.set(v, existing);
    }
  }

  if (versionMap.size === 0) {
    container.innerHTML = `
      <div class="discovery-changelogs-empty">
        <p>${t('discovery.no_changelogs')}</p>
        <button id="discovery-view-nexus-changelog-btn" class="btn-discovery-action" style="margin-top: 10px; display: inline-flex;">
          <span>🔗</span> ${t('discovery.view_changelog_nexus')}
        </button>
      </div>
    `;
    const btn = container.querySelector('#discovery-view-nexus-changelog-btn');
    if (btn) {
      btn.addEventListener('click', () => {
        openUrl(`https://www.nexusmods.com/palworld/mods/${details.modId}?tab=logs`);
      });
    }
    return;
  }

  const sortedVersions = Array.from(versionMap.entries()).sort((a, b) => b[0].localeCompare(a[0], undefined, { numeric: true }));

  container.innerHTML = sortedVersions.map(([ver, items]) => `
    <div class="discovery-changelog-card">
      <div class="discovery-changelog-header">
        <span class="discovery-changelog-version">v${escapeHtml(ver)}</span>
      </div>
      <ul class="discovery-changelog-items">
        ${items.map((item) => `<li>${escapeHtml(item)}</li>`).join('')}
      </ul>
    </div>
  `).join('');
}

export function renderFilesList(details: DiscoveryModDetails, container: HTMLElement): void {
  if (!details.files || details.files.length === 0) {
    container.innerHTML = `<p style="color: var(--text-muted); font-size: 12px;">${t('discovery.no_files')}</p>`;
    return;
  }

  const isPremium = isUserPremium();

  // Helper comparator: Sort files by timestamp descending or fileId descending (newest to oldest)
  const sortFilesDesc = (a: DiscoveryFileItem, b: DiscoveryFileItem): number => {
    if (a.uploadedTimestamp && b.uploadedTimestamp) {
      return b.uploadedTimestamp - a.uploadedTimestamp;
    }
    return b.fileId - a.fileId;
  };

  // Group files
  const mainFiles: DiscoveryFileItem[] = [];
  const updateFiles: DiscoveryFileItem[] = [];
  const optionalFiles: DiscoveryFileItem[] = [];
  const archivedFiles: DiscoveryFileItem[] = [];

  for (const f of details.files) {
    const cat = (f.categoryName || '').toUpperCase();
    if (cat.includes('OLD') || cat.includes('ARCHIVE') || f.categoryId === 4) {
      archivedFiles.push(f);
    } else if (cat.includes('UPDATE') || f.categoryId === 2) {
      updateFiles.push(f);
    } else if (cat.includes('OPTIONAL') || f.categoryId === 3) {
      optionalFiles.push(f);
    } else {
      mainFiles.push(f);
    }
  }

  mainFiles.sort(sortFilesDesc);
  updateFiles.sort(sortFilesDesc);
  optionalFiles.sort(sortFilesDesc);
  archivedFiles.sort(sortFilesDesc);

  const getScanBadge = (status?: string | null) => {
    const st = (status || 'VERIFIED').toUpperCase();
    if (st === 'VERIFIED' || st === 'SAFE' || st === 'PASSED') {
      return `<span class="discovery-scan-badge scan-verified" title="Virus scan verified clean">🟢 ${t('discovery.scan_verified')}</span>`;
    }
    if (st.includes('MANUAL')) {
      return `<span class="discovery-scan-badge scan-manual" title="Manually verified by Nexus Mods staff">🔵 ${t('discovery.scan_manual')}</span>`;
    }
    if (st.includes('QUARANTINE') || st.includes('SUSPICIOUS') || st.includes('INFECTED')) {
      return `<span class="discovery-scan-badge scan-quarantine" title="Suspicious file under quarantine">🔴 ${t('discovery.scan_quarantine')}</span>`;
    }
    return `<span class="discovery-scan-badge scan-unverified" title="Awaiting or scanning">⚪ ${t('discovery.scan_unverified')}</span>`;
  };

  const renderFileCard = (f: DiscoveryFileItem) => {
    const catClass = (f.categoryName || 'main').toLowerCase().replace(/\s+/g, '_');
    const isPrimaryCard = f.isPrimary || catClass === 'main';
    const descHtml = f.description ? descriptionToHtml(f.description) : '';
    const hasExtraContent = !!(descHtml || (f.changelogEntries && f.changelogEntries.length > 0));

    const uniqueDls = f.uniqueDownloads !== undefined && f.uniqueDownloads !== null ? f.uniqueDownloads.toLocaleString() : '--';
    const totalDls = f.totalDownloads !== undefined && f.totalDownloads !== null ? f.totalDownloads.toLocaleString() : '--';

    const btnHtml = isPremium
      ? `<button class="discovery-file-btn-install" data-file-id="${f.fileId}" title="${t('discovery.install_file')}">⚡ ${t('discovery.install_short')}</button>`
      : `<button class="discovery-file-btn-install free-download" data-file-id="${f.fileId}" title="${t('discovery.manual_download')}">🌐 ${t('discovery.manual_download')}</button>`;

    return `
      <div class="discovery-file-card ${isPrimaryCard ? 'primary-file' : ''}" data-file-id="${f.fileId}">
        <div class="discovery-file-top">
          <div class="discovery-file-title-wrap">
            <span class="discovery-file-cat-badge ${catClass}">${escapeHtml(f.categoryName || 'FILE')}</span>
            <span class="discovery-file-name">${escapeHtml(f.name)}</span>
            ${getScanBadge(f.scanStatus)}
          </div>
          <div style="display: flex; align-items: center; gap: 8px;">
            ${btnHtml}
            ${hasExtraContent ? `
              <button type="button" class="btn-discovery-action discovery-file-toggle-btn" data-file-id="${f.fileId}" title="Toggle description & changelog" style="padding: 6px 10px;">
                🔽
              </button>
            ` : ''}
          </div>
        </div>

        <div class="discovery-file-meta">
          <span>📅 ${t('discovery.uploaded_label')}: <strong>${f.uploadedAt || '--'}</strong></span>
          <span>•</span>
          <span>💾 ${t('discovery.size_label')}: <strong>${escapeHtml(f.sizeFormatted)}</strong></span>
          <span>•</span>
          <span>🏷️ ${t('discovery.version_label')}: <strong>${escapeHtml(f.version)}</strong></span>
          ${uniqueDls !== '--' ? `<span>•</span><span>👤 ${t('discovery.unique_dls')}: <strong>${uniqueDls}</strong></span>` : ''}
          ${totalDls !== '--' ? `<span>•</span><span>📥 ${t('discovery.total_dls')}: <strong>${totalDls}</strong></span>` : ''}
        </div>

        ${hasExtraContent ? `
          <div class="discovery-file-accordion-body" id="file-accordion-body-${f.fileId}" style="display:none; margin-top: 8px; padding-top: 8px; border-top: 1px solid var(--border);">
            ${descHtml ? `<div class="discovery-file-desc">${descHtml}</div>` : ''}
            ${f.changelogEntries && f.changelogEntries.length > 0 ? `
              <div class="discovery-file-changelog-wrap" style="margin-top: 8px;">
                <strong style="font-size: 11px; color: var(--text-primary);">📜 ${t('discovery.file_changelog')}:</strong>
                <ul class="discovery-changelog-items" style="margin-top: 4px; padding-left: 16px; font-size: 11px; color: var(--text-secondary);">
                  ${f.changelogEntries.map((c) => `<li>${escapeHtml(c)}</li>`).join('')}
                </ul>
              </div>
            ` : ''}
          </div>
        ` : ''}
      </div>
    `;
  };

  let html = '';
  if (mainFiles.length > 0) {
    html += `<div class="discovery-file-section-title">⭐ ${t('discovery.main_files')} <span class="section-count">${mainFiles.length}</span></div>`;
    html += mainFiles.map(renderFileCard).join('');
  }
  if (updateFiles.length > 0) {
    html += `<div class="discovery-file-section-title">🔄 ${t('discovery.update_files')} <span class="section-count">${updateFiles.length}</span></div>`;
    html += updateFiles.map(renderFileCard).join('');
  }
  if (optionalFiles.length > 0) {
    html += `<div class="discovery-file-section-title">📦 ${t('discovery.optional_files')} <span class="section-count">${optionalFiles.length}</span></div>`;
    html += optionalFiles.map(renderFileCard).join('');
  }
  if (archivedFiles.length > 0) {
    html += `
      <button id="discovery-toggle-archived-btn" class="discovery-archived-toggle-btn">
        <span id="discovery-toggle-archived-icon">🔽</span>
        <span id="discovery-toggle-archived-text">${t('discovery.show_archived_files', { count: archivedFiles.length })}</span>
      </button>
      <div id="discovery-archived-files-container" style="display: none; flex-direction: column; gap: 12px; margin-top: 6px;">
        ${archivedFiles.map(renderFileCard).join('')}
      </div>
    `;
  }

  container.innerHTML = html;

  const toggleBtn = container.querySelector('#discovery-toggle-archived-btn') as HTMLButtonElement | null;
  const archivedContainer = container.querySelector('#discovery-archived-files-container') as HTMLElement | null;
  const toggleText = container.querySelector('#discovery-toggle-archived-text');
  const toggleIcon = container.querySelector('#discovery-toggle-archived-icon');

  if (toggleBtn && archivedContainer) {
    toggleBtn.addEventListener('click', () => {
      const isHidden = archivedContainer.style.display === 'none';
      archivedContainer.style.display = isHidden ? 'flex' : 'none';
      if (toggleText) {
        toggleText.textContent = isHidden
          ? t('discovery.hide_archived_files')
          : t('discovery.show_archived_files', { count: archivedFiles.length });
      }
      if (toggleIcon) {
        toggleIcon.textContent = isHidden ? '🔼' : '🔽';
      }
    });
  }

  // Attach accordion toggle handlers for individual files
  container.querySelectorAll('.discovery-file-toggle-btn').forEach((btn) => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const fid = (btn as HTMLElement).dataset.fileId;
      if (!fid) return;

      const body = document.getElementById(`file-accordion-body-${fid}`);
      if (body) {
        const isShown = body.style.display !== 'none';
        body.style.display = isShown ? 'none' : 'block';
        btn.textContent = isShown ? '🔽' : '🔼';
      }
    });
  });

  // Attach install / download handlers
  container.querySelectorAll('.discovery-file-btn-install').forEach((btn) => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const fid = parseInt((btn as HTMLElement).dataset.fileId || '0', 10);
      const targetFile = details.files.find((f) => f.fileId === fid);
      if (targetFile) {
        if (isUserPremium()) {
          await executeInstallFile(details, targetFile);
        } else {
          openUrl(`https://www.nexusmods.com/palworld/mods/${details.modId}?tab=files&file_id=${targetFile.fileId}`);
        }
      }
    });
  });
}

export function setupModalActions(details: DiscoveryModDetails): void {
  const endorseBtn = discoveryDom.elMaybe('discovery-modal-endorse-btn');
  const endorseText = discoveryDom.elMaybe('discovery-modal-endorse-text');
  const trackBtn = discoveryDom.elMaybe('discovery-modal-track-btn');
  const trackText = discoveryDom.elMaybe('discovery-modal-track-text');
  const communityBtn = discoveryDom.elMaybe('discovery-modal-community-btn');
  const bugsBtn = discoveryDom.elMaybe('discovery-modal-bugs-btn');
  const nexusLinkBtn = discoveryDom.elMaybe('discovery-modal-nexus-link-btn');

  let isEndorsed = details.isEndorsed || false;
  let isTracked = details.isTracked || false;

  const currentAccount = getState().currentSettings?.nexusAccount;
  const currentUsername = (currentAccount?.username || '').trim().toLowerCase();
  const authorName = (details.author || '').trim().toLowerCase();
  const isOwnMod = !!(currentUsername && authorName && currentUsername === authorName);

  if (endorseBtn) {
    if (isOwnMod) {
      endorseBtn.classList.remove('active');
      endorseBtn.classList.add('own-mod-btn');
      endorseBtn.title = t('discovery.own_mod_cannot_endorse');
      if (endorseText) endorseText.textContent = `👑 ${t('discovery.own_mod_label')}`;
      endorseBtn.onclick = () => {
        showToast(t('discovery.own_mod_cannot_endorse'), 'warning');
      };
    } else {
      endorseBtn.classList.remove('own-mod-btn');
      endorseBtn.title = t('discovery.endorse_title');
      endorseBtn.classList.toggle('active', isEndorsed);
      if (endorseText) endorseText.textContent = isEndorsed ? t('discovery.endorsed_btn') : t('discovery.endorse_btn');

      endorseBtn.onclick = async () => {
        try {
          if (!isEndorsed) {
            await endorseNexusMod(details.modId, details.version);
            isEndorsed = true;
            endorseBtn.classList.add('active');
            if (endorseText) endorseText.textContent = t('discovery.endorsed_btn');
            showToast(t('discovery.endorse_success'), 'success');
          } else {
            await abstainNexusMod(details.modId, details.version);
            isEndorsed = false;
            endorseBtn.classList.remove('active');
            if (endorseText) endorseText.textContent = t('discovery.endorse_btn');
            showToast(t('discovery.endorse_removed'), 'info');
          }
        } catch (err: any) {
          const errStr = String(err);
          if (errStr.includes('IS_OWN_MOD')) {
            showToast(t('discovery.own_mod_cannot_endorse'), 'warning');
            endorseBtn.classList.add('own-mod-btn');
            if (endorseText) endorseText.textContent = `👑 ${t('discovery.own_mod_label')}`;
          } else {
            showToast(errStr, 'error');
          }
        }
      };
    }
  }

  if (trackBtn) {
    trackBtn.classList.toggle('active', isTracked);
    if (trackText) trackText.textContent = isTracked ? t('discovery.tracked_btn') : t('discovery.track_btn');

    trackBtn.onclick = async () => {
      try {
        if (!isTracked) {
          await trackNexusMod(details.modId);
          isTracked = true;
          trackBtn.classList.add('active');
          if (trackText) trackText.textContent = t('discovery.tracked_btn');
          showToast(t('discovery.track_success'), 'success');
        } else {
          await untrackNexusMod(details.modId);
          isTracked = false;
          trackBtn.classList.remove('active');
          if (trackText) trackText.textContent = t('discovery.track_btn');
          showToast(t('discovery.track_removed'), 'info');
        }
      } catch (err: any) {
        showToast(String(err), 'error');
      }
    };
  }

  if (communityBtn) {
    communityBtn.onclick = () => {
      openUrl(`https://www.nexusmods.com/palworld/mods/${details.modId}?tab=posts`);
    };
  }

  if (bugsBtn) {
    bugsBtn.onclick = () => {
      openUrl(`https://www.nexusmods.com/palworld/mods/${details.modId}?tab=bugs`);
    };
  }

  if (nexusLinkBtn) {
    nexusLinkBtn.onclick = () => {
      openUrl(`https://www.nexusmods.com/palworld/mods/${details.modId}`);
    };
  }
}

export async function executeInstallFile(mod: DiscoveryModDetails, file: DiscoveryFileItem): Promise<void> {
  try {
    showToast(t('discovery.fetching_download_url'), 'info');
    const directUrl = await installDiscoveryFile(mod.modId, file.fileId);
    if (!directUrl) {
      showToast(t('discovery.download_url_failed'), 'error');
      return;
    }

    closeDiscoveryModal();

    await enqueueDiscoveryDownload(
      mod.modId,
      file.fileId,
      `${mod.name} - ${file.name}`,
      directUrl,
      mod.author,
      mod.pictureUrl,
      file.version
    );
  } catch (err: any) {
    console.error('[Discovery] Direct installation error:', err);
    showToast(t('discovery.install_failed', { error: String(err) }), 'error');
  }
}

export function closeDiscoveryModal(): void {
  const modal = discoveryDom.elMaybe('discovery-mod-modal');
  if (modal) {
    modal.classList.remove('visible');
    modal.classList.remove('active');
    modal.style.display = 'none';
  }
  discState.currentModalMod = null;
}
