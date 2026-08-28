import { invoke } from '@tauri-apps/api/core';
import {
  getNexusUserAuthoredMods,
  getNexusUserEndorsements,
  getNexusUserTrackedMods,
  refreshNexusAccountProfile,
  checkNexusProtocolStatus,
  type NexusAccountInfo,
  type NexusUserAuthoredMod,
  type NexusUserEndorsement,
  type NexusUserTrackedMod,
} from '../../api';
import { getState, updateState } from '../../state';
import { showToast } from '../../ui/toast';
import { t } from '../../utils/i18n';
import { DEFAULT_AVATAR } from './state';
import { formatDate } from './helpers';
import { triggerLogout } from './oauth';

export function openNexusProfileModal(): void {
  const modal = document.getElementById('nexus-profile-modal');
  if (!modal) return;

  const state = getState();
  const account = state.currentSettings?.nexusAccount;
  if (!account) return;

  const avatarEl = document.getElementById('nexus-profile-modal-avatar') as HTMLImageElement | null;
  const nameEl = document.getElementById('nexus-profile-modal-name');
  const idEl = document.getElementById('nexus-profile-modal-id');
  const badgeEl = document.getElementById('nexus-profile-modal-badge');
  const authorBadgeEl = document.getElementById('nexus-profile-modal-author-badge');
  const downloadsEl = document.getElementById('nexus-profile-modal-downloads');
  const downloadsCountEl = document.getElementById('nexus-profile-modal-downloads-count');
  const protoEl = document.getElementById('nexus-profile-modal-proto');
  const endorsementsEl = document.getElementById('nexus-profile-modal-endorsements');
  const viewsEl = document.getElementById('nexus-profile-modal-views');
  const kudosEl = document.getElementById('nexus-profile-modal-kudos');
  const lastActiveEl = document.getElementById('nexus-profile-modal-last-active');
  const joinedEl = document.getElementById('nexus-profile-modal-joined');
  const aboutEl = document.getElementById('nexus-profile-modal-about');

  const updateAuthorMetrics = (mods: NexusUserAuthoredMod[]) => {
    if (mods && mods.length > 0) {
      if (authorBadgeEl) authorBadgeEl.style.display = 'inline-flex';
      const totalDownloads = mods.reduce((sum, m) => sum + (m.downloads || 0), 0);
      if (downloadsEl && downloadsCountEl && totalDownloads > 0) {
        downloadsCountEl.textContent = totalDownloads.toLocaleString();
        downloadsEl.style.display = 'inline-flex';
      }
    }
  };

  const populateFields = (acc: NexusAccountInfo) => {
    if (avatarEl) {
      avatarEl.src = acc.avatarUrl || DEFAULT_AVATAR;
      avatarEl.onerror = () => { avatarEl.src = DEFAULT_AVATAR; };
    }
    if (nameEl) nameEl.textContent = acc.username || 'Nexus User';
    if (idEl) idEl.textContent = acc.userId ? `#${acc.userId}` : 'N/A';

    if (badgeEl) {
      const isPremium = acc.isPremium;
      const isSupporter = acc.isSupporter;
      badgeEl.className = `nexus-tier-badge ${isPremium ? 'nexus-badge-premium' : isSupporter ? 'nexus-badge-supporter' : 'nexus-badge-free'}`;
      badgeEl.textContent = isPremium ? t('settings.nexus_status_premium') : isSupporter ? t('settings.nexus_status_supporter') : t('settings.nexus_status_free');
    }

    const isAuthor = (acc.modCount !== undefined && acc.modCount !== null && acc.modCount > 0) || (acc.roles && acc.roles.some(r => r.toLowerCase().includes('author') || r.toLowerCase().includes('creator')));
    if (authorBadgeEl) {
      authorBadgeEl.style.display = isAuthor ? 'inline-flex' : 'none';
    }

    if (endorsementsEl) {
      endorsementsEl.textContent = acc.endorsementsGiven !== undefined && acc.endorsementsGiven !== null 
        ? acc.endorsementsGiven.toLocaleString() 
        : '--';
    }

    if (viewsEl) {
      viewsEl.textContent = acc.profileViews !== undefined && acc.profileViews !== null 
        ? acc.profileViews.toLocaleString() 
        : '--';
    }

    if (kudosEl) {
      kudosEl.textContent = acc.kudos !== undefined && acc.kudos !== null 
        ? acc.kudos.toLocaleString() 
        : '--';
    }

    if (lastActiveEl) {
      lastActiveEl.textContent = formatDate(acc.lastActiveDate);
    }

    if (joinedEl) {
      joinedEl.textContent = formatDate(acc.joinedDate);
    }

    if (aboutEl) {
      aboutEl.textContent = acc.aboutMe?.trim() || t('nexus_profile.no_about');
    }
  };

  populateFields(account);

  // Pre-fetch authored mods in background to populate downloads counter & author badge early
  getNexusUserAuthoredMods(false).then(myMods => {
    cachedMyMods = myMods;
    const authoredCountBadge = document.getElementById('nexus-tab-authored-count');
    if (authoredCountBadge) authoredCountBadge.textContent = String(myMods.length);
    updateAuthorMetrics(myMods);
  }).catch(() => {});

  // Tab Switching Logic
  const tabs = modal.querySelectorAll<HTMLButtonElement>('.nexus-modal-tab');
  const panels: Record<string, HTMLElement | null> = {
    overview: document.getElementById('nexus-panel-overview'),
    endorsements: document.getElementById('nexus-panel-endorsements'),
    tracked: document.getElementById('nexus-panel-tracked'),
    'my-mods': document.getElementById('nexus-panel-my-mods'),
  };

  let cachedEndorsements: NexusUserEndorsement[] | null = null;
  let cachedTracked: NexusUserTrackedMod[] | null = null;
  let cachedMyMods: NexusUserAuthoredMod[] | null = null;

  const switchTab = (targetTab: string) => {
    tabs.forEach(t => {
      const isMatch = t.getAttribute('data-tab') === targetTab;
      t.classList.toggle('active', isMatch);
      t.style.borderBottom = isMatch ? '2px solid var(--accent)' : '2px solid transparent';
      t.style.color = isMatch ? 'var(--text-primary)' : 'var(--text-muted)';
    });

    Object.entries(panels).forEach(([tabKey, panelEl]) => {
      if (panelEl) {
        panelEl.style.display = tabKey === targetTab ? 'flex' : 'none';
      }
    });

    if (targetTab === 'endorsements') loadEndorsementsTab();
    if (targetTab === 'tracked') loadTrackedTab();
    if (targetTab === 'my-mods') loadMyModsTab();
  };

  tabs.forEach(t => {
    t.onclick = () => {
      const tab = t.getAttribute('data-tab') || 'overview';
      switchTab(tab);
    };
  });

  // Endorsements Tab Renderer
  const renderEndorsementsList = (items: NexusUserEndorsement[]) => {
    const listContainer = document.getElementById('nexus-endorsements-list');
    const countBadge = document.getElementById('nexus-tab-endorsements-count');
    if (!listContainer) return;

    const palworldOnly = (document.getElementById('nexus-endorsements-palworld-only') as HTMLInputElement)?.checked ?? true;
    const searchVal = (document.getElementById('nexus-endorsements-search') as HTMLInputElement)?.value.toLowerCase().trim() || '';

    let filtered = items.filter(item => {
      if (palworldOnly && item.domainName.toLowerCase() !== 'palworld') return false;
      if (searchVal) {
        const titleMatch = (item.modTitle || '').toLowerCase().includes(searchVal);
        const idMatch = String(item.modId).includes(searchVal);
        const domainMatch = item.domainName.toLowerCase().includes(searchVal);
        return titleMatch || idMatch || domainMatch;
      }
      return true;
    });

    if (countBadge) {
      countBadge.textContent = String(items.length);
    }

    if (filtered.length === 0) {
      listContainer.innerHTML = `
        <div style="text-align: center; padding: 24px 12px; color: var(--text-muted); font-size: 11px;">
          ${t('nexus_profile.no_endorsements_found')}
        </div>
      `;
      return;
    }

    listContainer.innerHTML = filtered.map(item => {
      const domain = item.domainName.toLowerCase();
      const isPalworld = domain === 'palworld';
      const modName = item.modTitle || `Mod #${item.modId}`;
      const url = `https://www.nexusmods.com/${domain}/mods/${item.modId}`;
      const formattedDate = formatDate(item.date);

      return `
        <div style="display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 8px 12px; background: var(--bg-secondary); border-radius: 8px; border: 1px solid var(--border);">
          ${item.pictureUrl ? `<img src="${item.pictureUrl}" data-original-src="${item.pictureUrl}" alt="${modName}" style="width: 38px; height: 38px; border-radius: 6px; object-fit: cover; border: 1px solid var(--border); flex-shrink: 0;" onerror="window.handleUniversalImageFallback ? window.handleUniversalImageFallback(this) : (this.style.display='none')" />` : ''}
          <div style="display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1;">
            <div style="display: flex; align-items: center; gap: 6px;">
              <span style="font-size: 12px; font-weight: 700; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${modName}</span>
              ${isPalworld ? '<span style="font-size: 9px; font-weight: 700; background: rgba(46, 213, 115, 0.2); color: #2ed573; padding: 1px 5px; border-radius: 4px;">Palworld</span>' : `<span style="font-size: 9px; font-weight: 600; color: var(--text-muted);">${domain}</span>`}
            </div>
            ${item.summary ? `<div style="font-size: 10px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${item.summary}</div>` : ''}
            <div style="font-size: 10px; color: var(--text-muted); display: flex; gap: 8px;">
              <span>ID: #${item.modId}</span>
              ${item.version ? `<span>v${item.version}</span>` : ''}
              <span>${formattedDate}</span>
            </div>
          </div>
          <button type="button" class="btn btn-secondary btn-sm" style="padding: 3px 8px; font-size: 10px; flex-shrink: 0;" onclick="window.__openNexusModUrl('${url}')">
            🔗 Nexus
          </button>
        </div>
      `;
    }).join('');
  };

  const loadEndorsementsTab = async (force = false) => {
    const listContainer = document.getElementById('nexus-endorsements-list');
    if (!cachedEndorsements || force) {
      if (listContainer) listContainer.innerHTML = `<div style="text-align: center; padding: 20px; color: var(--text-muted); font-size: 11px;">${t('nexus_profile.loading_endorsements')}</div>`;
      try {
        cachedEndorsements = await getNexusUserEndorsements(force);
        renderEndorsementsList(cachedEndorsements);
      } catch (err: any) {
        if (listContainer) listContainer.innerHTML = `<div style="text-align: center; padding: 20px; color: var(--danger); font-size: 11px;">${String(err)}</div>`;
      }
    } else {
      renderEndorsementsList(cachedEndorsements);
    }
  };

  // Tracked Tab Renderer
  const renderTrackedList = (items: NexusUserTrackedMod[]) => {
    const listContainer = document.getElementById('nexus-tracked-list');
    const countBadge = document.getElementById('nexus-tab-tracked-count');
    if (!listContainer) return;

    const palworldOnly = (document.getElementById('nexus-tracked-palworld-only') as HTMLInputElement)?.checked ?? true;
    const searchVal = (document.getElementById('nexus-tracked-search') as HTMLInputElement)?.value.toLowerCase().trim() || '';

    let filtered = items.filter(item => {
      if (palworldOnly && item.domainName.toLowerCase() !== 'palworld') return false;
      if (searchVal) {
        const titleMatch = (item.modTitle || '').toLowerCase().includes(searchVal);
        const idMatch = String(item.modId).includes(searchVal);
        const domainMatch = item.domainName.toLowerCase().includes(searchVal);
        return titleMatch || idMatch || domainMatch;
      }
      return true;
    });

    if (countBadge) {
      countBadge.textContent = String(items.length);
    }

    if (filtered.length === 0) {
      listContainer.innerHTML = `
        <div style="text-align: center; padding: 24px 12px; color: var(--text-muted); font-size: 11px;">
          ${t('nexus_profile.no_tracked_found')}
        </div>
      `;
      return;
    }

    listContainer.innerHTML = filtered.map(item => {
      const domain = item.domainName.toLowerCase();
      const isPalworld = domain === 'palworld';
      const modName = item.modTitle || `Mod #${item.modId}`;
      const url = `https://www.nexusmods.com/${domain}/mods/${item.modId}`;

      return `
        <div style="display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 8px 12px; background: var(--bg-secondary); border-radius: 8px; border: 1px solid var(--border);">
          ${item.pictureUrl ? `<img src="${item.pictureUrl}" data-original-src="${item.pictureUrl}" alt="${modName}" style="width: 38px; height: 38px; border-radius: 6px; object-fit: cover; border: 1px solid var(--border); flex-shrink: 0;" onerror="window.handleUniversalImageFallback ? window.handleUniversalImageFallback(this) : (this.style.display='none')" />` : ''}
          <div style="display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1;">
            <div style="display: flex; align-items: center; gap: 6px;">
              <span style="font-size: 12px; font-weight: 700; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${modName}</span>
              ${isPalworld ? '<span style="font-size: 9px; font-weight: 700; background: rgba(218, 142, 53, 0.2); color: #da8e35; padding: 1px 5px; border-radius: 4px;">Palworld</span>' : `<span style="font-size: 9px; font-weight: 600; color: var(--text-muted);">${domain}</span>`}
            </div>
            ${item.summary ? `<div style="font-size: 10px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${item.summary}</div>` : ''}
            <div style="font-size: 10px; color: var(--text-muted);">
              <span>ID: #${item.modId}</span>
            </div>
          </div>
          <button type="button" class="btn btn-secondary btn-sm" style="padding: 3px 8px; font-size: 10px; flex-shrink: 0;" onclick="window.__openNexusModUrl('${url}')">
            🔗 Nexus
          </button>
        </div>
      `;
    }).join('');
  };

  const loadTrackedTab = async (force = false) => {
    const listContainer = document.getElementById('nexus-tracked-list');
    if (!cachedTracked || force) {
      if (listContainer) listContainer.innerHTML = `<div style="text-align: center; padding: 20px; color: var(--text-muted); font-size: 11px;">${t('nexus_profile.loading_tracked')}</div>`;
      try {
        cachedTracked = await getNexusUserTrackedMods(force);
        renderTrackedList(cachedTracked);
      } catch (err: any) {
        if (listContainer) listContainer.innerHTML = `<div style="text-align: center; padding: 20px; color: var(--danger); font-size: 11px;">${String(err)}</div>`;
      }
    } else {
      renderTrackedList(cachedTracked);
    }
  };

  // My Authored Mods Tab
  const renderMyModsList = (items: NexusUserAuthoredMod[]) => {
    const listContainer = document.getElementById('nexus-my-mods-list');
    const countBadge = document.getElementById('nexus-tab-authored-count');
    if (!listContainer) return;

    const palworldOnly = (document.getElementById('nexus-my-mods-palworld-only') as HTMLInputElement)?.checked ?? true;
    const searchVal = (document.getElementById('nexus-my-mods-search') as HTMLInputElement)?.value.toLowerCase().trim() || '';

    let filtered = items.filter(item => {
      const domain = (item.domainName || '').toLowerCase();
      if (palworldOnly && domain !== 'palworld') return false;
      if (searchVal) {
        const nameMatch = (item.name || '').toLowerCase().includes(searchVal);
        const descMatch = (item.summary || '').toLowerCase().includes(searchVal);
        const idMatch = String(item.modId).includes(searchVal);
        return nameMatch || descMatch || idMatch;
      }
      return true;
    });

    if (countBadge) {
      countBadge.textContent = String(items.length);
    }

    if (filtered.length === 0) {
      listContainer.innerHTML = `
        <div style="text-align: center; padding: 24px 12px; color: var(--text-muted); font-size: 11px;">
          ${t('nexus_profile.no_my_mods_found')}
        </div>
      `;
      return;
    }

    listContainer.innerHTML = filtered.map(item => {
      const domain = (item.domainName || 'palworld').toLowerCase();
      const isPalworld = domain === 'palworld';
      const url = `https://www.nexusmods.com/${domain}/mods/${item.modId}`;

      return `
        <div style="display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 8px 12px; background: var(--bg-secondary); border-radius: 8px; border: 1px solid var(--border);">
          ${item.pictureUrl ? `<img src="${item.pictureUrl}" data-original-src="${item.pictureUrl}" alt="${item.name}" style="width: 38px; height: 38px; border-radius: 6px; object-fit: cover; border: 1px solid var(--border); flex-shrink: 0;" onerror="window.handleUniversalImageFallback ? window.handleUniversalImageFallback(this) : (this.style.display='none')" />` : ''}
          <div style="display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1;">
            <div style="display: flex; align-items: center; gap: 6px;">
              <span style="font-size: 12px; font-weight: 700; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${item.name}</span>
              ${isPalworld ? '<span style="font-size: 9px; font-weight: 700; background: rgba(46, 213, 115, 0.2); color: #2ed573; padding: 1px 5px; border-radius: 4px;">Palworld</span>' : `<span style="font-size: 9px; font-weight: 600; color: var(--text-muted);">${domain}</span>`}
              ${item.version ? `<span style="font-size: 10px; color: var(--text-muted); font-family: monospace;">v${item.version}</span>` : ''}
            </div>
            ${item.summary ? `<div style="font-size: 10px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${item.summary}</div>` : ''}
            <div style="font-size: 10px; color: var(--text-secondary); display: flex; gap: 10px;">
              <span>📥 ${(item.downloads || 0).toLocaleString()}</span>
              <span>👍 ${(item.endorsements || 0).toLocaleString()}</span>
              <span>ID: #${item.modId}</span>
            </div>
          </div>
          <button type="button" class="btn btn-secondary btn-sm" style="padding: 3px 8px; font-size: 10px; flex-shrink: 0;" onclick="window.__openNexusModUrl('${url}')">
            🔗 Nexus
          </button>
        </div>
      `;
    }).join('');
  };

  const loadMyModsTab = async (force = false) => {
    const listContainer = document.getElementById('nexus-my-mods-list');
    if (!cachedMyMods || force) {
      if (listContainer) listContainer.innerHTML = `<div style="text-align: center; padding: 20px; color: var(--text-muted); font-size: 11px;">${t('nexus_profile.loading_my_mods')}</div>`;
      try {
        cachedMyMods = await getNexusUserAuthoredMods(force);
        updateAuthorMetrics(cachedMyMods);
        renderMyModsList(cachedMyMods);
      } catch (err: any) {
        if (listContainer) listContainer.innerHTML = `<div style="text-align: center; padding: 20px; color: var(--danger); font-size: 11px;">${String(err)}</div>`;
      }
    } else {
      updateAuthorMetrics(cachedMyMods);
      renderMyModsList(cachedMyMods);
    }
  };

  // Search & filter event listeners
  document.getElementById('nexus-endorsements-search')?.addEventListener('input', () => {
    if (cachedEndorsements) renderEndorsementsList(cachedEndorsements);
  });
  document.getElementById('nexus-endorsements-palworld-only')?.addEventListener('change', () => {
    if (cachedEndorsements) renderEndorsementsList(cachedEndorsements);
  });
  document.getElementById('nexus-tracked-search')?.addEventListener('input', () => {
    if (cachedTracked) renderTrackedList(cachedTracked);
  });
  document.getElementById('nexus-tracked-palworld-only')?.addEventListener('change', () => {
    if (cachedTracked) renderTrackedList(cachedTracked);
  });
  document.getElementById('nexus-my-mods-search')?.addEventListener('input', () => {
    if (cachedMyMods) renderMyModsList(cachedMyMods);
  });
  document.getElementById('nexus-my-mods-palworld-only')?.addEventListener('change', () => {
    if (cachedMyMods) renderMyModsList(cachedMyMods);
  });

  // Global helper for opening Nexus mod links from HTML onclick
  (window as any).__openNexusModUrl = (url: string) => {
    invoke('open_url', { url }).catch(() => {
      window.open(url, '_blank');
    });
  };

  // Auto-refresh profile if avatar or rich stats are missing
  if (!account.avatarUrl || account.endorsementsGiven === undefined || account.endorsementsGiven === null) {
    refreshNexusAccountProfile().then(updated => {
      if (updated) {
        const curSettings = getState().currentSettings;
        if (curSettings) {
          updateState({
            currentSettings: {
              ...curSettings,
              nexusAccount: updated,
            },
          });
        }
        populateFields(updated);
        import('./ui').then(({ renderSidebarNexusWidget, renderNexusAccountUI }) => {
          renderSidebarNexusWidget();
          renderNexusAccountUI();
        });
      }
    }).catch(err => {
      console.warn('Auto-refresh profile error:', err);
    });
  }

  // Instantly preload cached counts for tabs (Endorsements, Tracked, My Mods)
  const preloadCountsAndCaches = async () => {
    try {
      const [endorsements, tracked, authored] = await Promise.all([
        getNexusUserEndorsements(false).catch(() => []),
        getNexusUserTrackedMods(false).catch(() => []),
        getNexusUserAuthoredMods(false).catch(() => [])
      ]);
      cachedEndorsements = endorsements;
      cachedTracked = tracked;
      cachedMyMods = authored;

      const endBadge = document.getElementById('nexus-tab-endorsements-count');
      const trackBadge = document.getElementById('nexus-tab-tracked-count');
      const myModsBadge = document.getElementById('nexus-tab-authored-count') || document.getElementById('nexus-tab-my-mods-count');
      if (endBadge) endBadge.textContent = String(endorsements.length);
      if (trackBadge) trackBadge.textContent = String(tracked.length);
      if (myModsBadge) myModsBadge.textContent = String(authored.length);
    } catch (e) {
      console.warn('Preload profile counts failed:', e);
    }
  };
  preloadCountsAndCaches();

  checkNexusProtocolStatus().then(details => {
    if (protoEl) {
      if (details.palmodmanager.status === 'Registered') {
        protoEl.style.color = 'var(--success)';
        protoEl.textContent = t('settings.nexus_proto_registered');
      } else if (details.palmodmanager.status === 'OutdatedPath') {
        protoEl.style.color = 'var(--warning)';
        protoEl.textContent = t('settings.nexus_proto_outdated');
      } else {
        protoEl.style.color = 'var(--text-muted)';
        protoEl.textContent = t('settings.nexus_proto_not_registered');
      }
    }
  }).catch(() => {});

  const refreshBtn = document.getElementById('nexus-profile-modal-refresh');
  if (refreshBtn) {
    refreshBtn.onclick = async () => {
      refreshBtn.setAttribute('disabled', 'true');
      showToast(t('nexus_profile.refreshing'), 'info');
      try {
        cachedEndorsements = null;
        cachedTracked = null;
        cachedMyMods = null;
        const updated = await refreshNexusAccountProfile();
        if (updated) {
          const curSettings = getState().currentSettings;
          if (curSettings) {
            updateState({
              currentSettings: {
                ...curSettings,
                nexusAccount: updated,
              },
            });
          }
          populateFields(updated);
          const { renderSidebarNexusWidget, renderNexusAccountUI } = await import('./ui');
          renderSidebarNexusWidget();
          renderNexusAccountUI();
          showToast(t('nexus_profile.refreshed_success'), 'success');
          // Reload active tab with force = true
          const activeTab = modal.querySelector('.nexus-modal-tab.active')?.getAttribute('data-tab') || 'overview';
          if (activeTab === 'endorsements') loadEndorsementsTab(true);
          else if (activeTab === 'tracked') loadTrackedTab(true);
          else if (activeTab === 'my-mods') loadMyModsTab(true);
          else switchTab('overview');
        }
      } catch (err: any) {
        showToast(String(err), 'error');
      } finally {
        refreshBtn.removeAttribute('disabled');
      }
    };
  }

  const openWebProfile = () => {
    const currentAcc = getState().currentSettings?.nexusAccount;
    const profileUrl = currentAcc?.userId 
      ? `https://www.nexusmods.com/users/${currentAcc.userId}`
      : currentAcc?.username 
        ? `https://next.nexusmods.com/profile/${encodeURIComponent(currentAcc.username)}`
        : 'https://next.nexusmods.com';
    invoke('open_url', { url: profileUrl }).catch(() => {
      window.open(profileUrl, '_blank');
    });
  };

  const webBtn = document.getElementById('nexus-profile-modal-open-web');
  if (webBtn) {
    webBtn.onclick = openWebProfile;
  }

  const modalBanner = document.getElementById('nexus-profile-modal-banner');
  if (modalBanner) {
    modalBanner.onclick = openWebProfile;
  }

  const disconnectBtn = document.getElementById('nexus-profile-modal-disconnect');
  if (disconnectBtn) {
    disconnectBtn.onclick = async () => {
      modal.classList.remove('visible');
      await triggerLogout();
    };
  }

  const closeX = document.getElementById('nexus-profile-modal-close-x');
  const closeBtn = document.getElementById('nexus-profile-modal-close');
  const closeModal = () => closeNexusProfileModal();
  if (closeX) closeX.onclick = closeModal;
  if (closeBtn) closeBtn.onclick = closeModal;

  // Initial tab reset to overview
  switchTab('overview');

  modal.classList.add('visible');
}

export function closeNexusProfileModal(): void {
  const modal = document.getElementById('nexus-profile-modal');
  if (modal) modal.classList.remove('visible');
}
