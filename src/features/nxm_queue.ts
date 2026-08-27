import { listen } from '@tauri-apps/api/event';
import {
  parseNxmLink,
  getNxmModMetadata,
  downloadNxmFile,
  type NxmDownloadProgressEvent,
  type NxmModMetadata,
} from '../api';
import { showToast } from '../ui/toast';
import { escapeHtml } from '../utils/helpers';
import { t } from '../utils/i18n';
import { openInstallModalForZip, setInstallModalCallback } from '../ui/modals/installer';

export interface NxmQueueItem {
  id: string;
  nxmUrl: string;
  gameDomain: string;
  modId: number;
  fileId: number;
  modName: string;
  modAuthor?: string;
  modPictureUrl?: string;
  modVersion?: string;
  status: 'queued' | 'downloading' | 'awaiting_install' | 'installing' | 'done' | 'cancelled' | 'error';
  progress: number;
  downloadedBytes: number;
  totalBytes?: number;
  errorMessage?: string;
  tempZipPath?: string;
  addedAt: number;
}

let queue: NxmQueueItem[] = [];
let isQueueProcessing = false;
let isInstallingActive = false;
let isInitialized = false;

export async function initNxmQueue(): Promise<void> {
  if (isInitialized) return;
  isInitialized = true;

  // 1. Listen for progress events from Rust backend
  try {
    await listen<NxmDownloadProgressEvent>('nxm-download-progress', (event) => {
      const { downloadId, bytesDownloaded, totalBytes, percentage } = event.payload;
      const item = queue.find((q) => q.id === downloadId);
      if (item && (item.status === 'downloading' || item.status === 'queued')) {
        item.status = 'downloading';
        item.progress = Math.min(100, Math.max(0, Math.round(percentage)));
        item.downloadedBytes = bytesDownloaded;
        if (totalBytes) item.totalBytes = totalBytes;
        renderQueueUI();
      }
    });
  } catch (err) {
    console.warn('[NxmQueue] Failed to register progress listener:', err);
  }

  // 2. Setup Tray Toggle Controls
  const toggleBtn = document.getElementById('nxm-tray-toggle-collapse');
  const closeBtn = document.getElementById('nxm-tray-close');
  const header = document.getElementById('nxm-tray-header-toggle');
  const tray = document.getElementById('nxm-download-tray');

  if (toggleBtn && tray) {
    toggleBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      const isMin = tray.classList.toggle('minimized');
      toggleBtn.textContent = isMin ? '▼' : '▲';
    });
  }

  if (header && tray) {
    header.addEventListener('click', () => {
      const isMin = tray.classList.toggle('minimized');
      if (toggleBtn) toggleBtn.textContent = isMin ? '▼' : '▲';
    });
  }

  if (closeBtn && tray) {
    closeBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      tray.classList.add('hidden');
    });
  }
}

export async function enqueueNxmDownload(nxmUrl: string): Promise<void> {
  await initNxmQueue();

  try {
    const parsed = await parseNxmLink(nxmUrl);
    const downloadId = 'nxm_' + Date.now() + '_' + Math.random().toString(36).substring(2, 7);

    const newItem: NxmQueueItem = {
      id: downloadId,
      nxmUrl,
      gameDomain: parsed.gameDomain,
      modId: parsed.modId,
      fileId: parsed.fileId,
      modName: `Mod #${parsed.modId}`,
      status: 'queued',
      progress: 0,
      downloadedBytes: 0,
      addedAt: Date.now(),
    };

    queue.push(newItem);
    showTray();
    renderQueueUI();
    showToast(t('nexus_profile.nxm_downloading'), 'info');

    // Fetch rich metadata asynchronously
    getNxmModMetadata(parsed.gameDomain, parsed.modId).then((meta: NxmModMetadata) => {
      if (meta && meta.name) {
        newItem.modName = meta.name;
        newItem.modAuthor = meta.author || undefined;
        newItem.modPictureUrl = meta.pictureUrl || undefined;
        if (meta.version) newItem.modVersion = meta.version;
        renderQueueUI();
      }
    }).catch(() => {});

    // Start background download pipeline
    processDownloads();
  } catch (err: any) {
    console.error('[NxmQueue] Failed to parse NXM URL:', err);
    showToast(t('nexus_profile.nxm_download_failed', { error: String(err) }), 'error');
  }
}

export async function enqueueDiscoveryDownload(
  modId: number,
  fileId: number,
  modName: string,
  downloadUrl: string,
  modAuthor?: string,
  modPictureUrl?: string,
  modVersion?: string
): Promise<void> {
  const downloadId = 'disc_' + Date.now() + '_' + Math.random().toString(36).substring(2, 7);
  const newItem: NxmQueueItem = {
    id: downloadId,
    nxmUrl: downloadUrl,
    gameDomain: 'palworld',
    modId,
    fileId,
    modName: modName || `Mod #${modId}`,
    modAuthor,
    modPictureUrl,
    modVersion,
    status: 'queued',
    progress: 0,
    downloadedBytes: 0,
    addedAt: Date.now(),
  };

  queue.push(newItem);
  showTray();
  renderQueueUI();
  showToast(t('nexus_profile.nxm_downloading'), 'info');
  processDownloads();
}

function showTray(): void {
  const tray = document.getElementById('nxm-download-tray');
  if (tray) {
    tray.classList.remove('hidden');
    tray.classList.remove('minimized');
    const toggleBtn = document.getElementById('nxm-tray-toggle-collapse');
    if (toggleBtn) toggleBtn.textContent = '▲';
  }
}

async function processDownloads(): Promise<void> {
  const pending = queue.find((q) => q.status === 'queued');
  if (!pending) {
    checkNextInstall();
    return;
  }

  pending.status = 'downloading';
  renderQueueUI();

  try {
    const zipPath = await downloadNxmFile(pending.nxmUrl, pending.id);
    pending.tempZipPath = zipPath;
    pending.status = 'awaiting_install';
    pending.progress = 100;
    renderQueueUI();
  } catch (err: any) {
    console.error(`[NxmQueue] Download error for ${pending.modName}:`, err);
    pending.status = 'error';
    pending.errorMessage = String(err);
    renderQueueUI();
    showToast(t('nexus_profile.nxm_download_failed', { error: String(err) }), 'error');
  }

  // Continue downloading remaining items
  processDownloads();
}

export async function checkNextInstall(): Promise<void> {
  const installModal = document.getElementById('install-modal');
  const isModalOpen = installModal && installModal.classList.contains('visible');
  if (!isModalOpen) {
    isInstallingActive = false;
    queue.forEach((q) => {
      if (q.status === 'installing') {
        q.status = 'awaiting_install';
      }
    });
  }

  if (isInstallingActive) return;

  const nextToInstall = queue.find((q) => q.status === 'awaiting_install');
  if (!nextToInstall || !nextToInstall.tempZipPath) {
    renderQueueUI();
    return;
  }

  isInstallingActive = true;
  nextToInstall.status = 'installing';
  renderQueueUI();

  // If mod metadata is still the fallback placeholder 'Mod #1234', fetch real metadata before opening modal
  if (nextToInstall.modName.startsWith('Mod #') && nextToInstall.modId) {
    try {
      const meta = await getNxmModMetadata(nextToInstall.gameDomain || 'palworld', nextToInstall.modId);
      if (meta && meta.name) {
        nextToInstall.modName = meta.name;
        if (meta.author) nextToInstall.modAuthor = meta.author;
        if (meta.pictureUrl) nextToInstall.modPictureUrl = meta.pictureUrl;
        if (meta.version) nextToInstall.modVersion = meta.version;
        renderQueueUI();
      }
    } catch (e) {
      console.warn('[NxmQueue] Failed to resolve metadata before install:', e);
    }
  }

  setInstallModalCallback(async (success: boolean) => {
    isInstallingActive = false;
    nextToInstall.status = success ? 'done' : 'cancelled';
    renderQueueUI();

    if (success) {
      showToast(t('nexus_profile.nxm_download_success'), 'success');
      const { loadMods } = await import('../ui/mods/loader');
      await loadMods();
    }

    // Clean up temporary zip file
    if (nextToInstall.tempZipPath) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        // Let temporary files naturally clean or invoke remove
      } catch {}
    }

    // Check if there are more waiting installs in queue
    setTimeout(() => {
      checkNextInstall();
    }, 400);
  });

  openInstallModalForZip(
    nextToInstall.tempZipPath,
    nextToInstall.modName,
    nextToInstall.modId,
    nextToInstall.modVersion
  ).catch((err) => {
    console.error('[NxmQueue] Failed to open installer modal:', err);
    isInstallingActive = false;
    nextToInstall.status = 'error';
    nextToInstall.errorMessage = String(err);
    renderQueueUI();
    checkNextInstall();
  });
}

export function cancelQueueItem(id: string): void {
  const item = queue.find((q) => q.id === id);
  if (item) {
    item.status = 'cancelled';
    isInstallingActive = false;
    renderQueueUI();
    checkNextInstall();
  }
}

export function removeQueueItem(id: string): void {
  queue = queue.filter((q) => q.id !== id);
  renderQueueUI();
  if (queue.length === 0) {
    const tray = document.getElementById('nxm-download-tray');
    if (tray) tray.classList.add('hidden');
  }
}

const DEFAULT_NXM_THUMB = 'data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIzNiIgaGVpZ2h0PSIzNiIgdmlld0JveD0iMCAwIDI0IDI0IiBmaWxsPSIjNzA3ODg4Ij48cGF0aCBkPSJNMTkgOWgtNFYzSDl2Nkg1bDcgNyA3LTd6TTUgMTh2MmgxNHYtMkg1eiIvPjwvc3ZnPg==';

export function renderQueueUI(): void {
  const body = document.getElementById('nxm-download-body');
  const countBadge = document.getElementById('nxm-tray-badge-count');
  if (!body) return;

  const activeCount = queue.filter((q) => ['queued', 'downloading', 'awaiting_install', 'installing'].includes(q.status)).length;
  if (countBadge) {
    countBadge.textContent = String(activeCount);
    countBadge.style.display = activeCount > 0 ? 'inline-block' : 'none';
  }

  if (queue.length === 0) {
    body.innerHTML = `
      <div style="text-align:center; padding: 18px 8px; color: var(--text-muted); font-size: 11px;">
        ${t('nxm_queue.no_items')}
      </div>
    `;
    return;
  }

  body.innerHTML = queue.map((item) => {
    const isProgressActive = item.status === 'downloading';
    let statusClass = 'nxm-status-queued';
    let statusText = t('nxm_queue.status_pending');

    if (item.status === 'downloading') {
      statusClass = 'nxm-status-downloading';
      statusText = `${t('nxm_queue.status_downloading')} ${item.progress}%`;
    } else if (item.status === 'awaiting_install') {
      statusClass = 'nxm-status-ready';
      statusText = t('nxm_queue.status_awaiting_install');
    } else if (item.status === 'installing') {
      statusClass = 'nxm-status-installing';
      statusText = t('nxm_queue.status_installing');
    } else if (item.status === 'done') {
      statusClass = 'nxm-status-done';
      statusText = t('nxm_queue.status_done');
    } else if (item.status === 'cancelled') {
      statusClass = 'nxm-status-queued';
      statusText = t('nxm_queue.status_cancelled');
    } else if (item.status === 'error') {
      statusClass = 'nxm-status-error';
      statusText = t('nxm_queue.status_error');
    }

    const thumbSrc = item.modPictureUrl || DEFAULT_NXM_THUMB;

    let actionBtnHtml = '';
    if (item.status === 'awaiting_install' || item.status === 'installing') {
      actionBtnHtml = `
        <button type="button" class="btn btn-primary btn-sm btn-queue-install" data-id="${item.id}" style="padding: 2px 8px !important; font-size: 10px !important;">
          ${t('nxm_queue.btn_install_now')}
        </button>
        <button type="button" class="nxm-tray-btn btn-queue-cancel" data-id="${item.id}" title="Cancel">✕</button>
      `;
    } else if (item.status === 'done' || item.status === 'cancelled' || item.status === 'error') {
      actionBtnHtml = `
        <button type="button" class="nxm-tray-btn btn-queue-remove" data-id="${item.id}" title="Remove">✕</button>
      `;
    } else if (item.status === 'queued') {
      actionBtnHtml = `
        <button type="button" class="nxm-tray-btn btn-queue-cancel" data-id="${item.id}" title="Cancel">✕</button>
      `;
    }

    return `
      <div class="nxm-queue-item ${item.status === 'downloading' || item.status === 'installing' ? 'active' : ''}" id="queue-item-${item.id}">
        <div class="nxm-queue-item-main">
          <img src="${escapeHtml(thumbSrc)}" alt="" class="nxm-queue-thumb" />
          <div class="nxm-queue-info">
            <div class="nxm-queue-name" title="${escapeHtml(item.modName)}">${escapeHtml(item.modName)}</div>
            <div class="nxm-queue-sub">
              <span class="nxm-queue-status-tag ${statusClass}">${statusText}</span>
              ${item.modAuthor ? `<span>by ${escapeHtml(item.modAuthor)}</span>` : ''}
            </div>
          </div>
          <div class="nxm-queue-actions">
            ${actionBtnHtml}
          </div>
        </div>
        ${isProgressActive ? `
          <div class="nxm-queue-progress-bar">
            <div class="nxm-queue-progress-fill" style="width: ${item.progress}%;"></div>
          </div>
        ` : ''}
      </div>
    `;
  }).join('');

  // Setup fallback for broken images
  body.querySelectorAll<HTMLImageElement>('.nxm-queue-thumb').forEach((img) => {
    img.onerror = () => {
      img.src = DEFAULT_NXM_THUMB;
    };
  });


  // Bind item button events
  body.querySelectorAll<HTMLButtonElement>('.btn-queue-install').forEach((btn) => {
    btn.onclick = () => {
      checkNextInstall();
    };
  });

  body.querySelectorAll<HTMLButtonElement>('.btn-queue-cancel').forEach((btn) => {
    btn.onclick = () => {
      const id = btn.getAttribute('data-id');
      if (id) cancelQueueItem(id);
    };
  });

  body.querySelectorAll<HTMLButtonElement>('.btn-queue-remove').forEach((btn) => {
    btn.onclick = () => {
      const id = btn.getAttribute('data-id');
      if (id) removeQueueItem(id);
    };
  });
}
