import { type DiscoveryModItem, openUrl } from '../../api';
import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';
import logoUrl from '../../assets/logo.png';
import { discState, isUserPremium } from './state';
import { formatDateDisplay } from './helpers';
import { openModDetails } from './detailsModal';
import { discoveryDom } from '../../framework';
import { getIconSvg } from '../icons/mod';

export function renderGrid(mods: DiscoveryModItem[]): void {
  const grid = discoveryDom.elMaybe('discovery-grid');
  if (!grid) return;

  const isPremium = isUserPremium();

  grid.innerHTML = mods.map((m) => {
    const pic = m.pictureUrl && m.pictureUrl.trim() !== '' ? m.pictureUrl : logoUrl;
    const cat = m.categoryName || (m.categoryId ? discState.categories.find((c) => c.categoryId === m.categoryId)?.name : '') || 'Mod';
    const isNsfw = m.containsAdultContent || false;
    const isBlurred = isNsfw && discState.currentNsfwFilter === 'blur';

    const uploadedDate = formatDateDisplay(m.createdAt);
    const updatedDate = formatDateDisplay(m.updatedAt);

    const actionBtnHtml = isPremium
      ? `<button class="discovery-card-btn-install" data-action="quick-install" data-mod-id="${m.modId}" title="${t('discovery.btn_quick_install')}">${getIconSvg('zap', 'pmm-icon-xs')} <span>${t('discovery.install_short')}</span></button>`
      : `<button class="discovery-card-btn-install" data-action="view-nexus" data-mod-id="${m.modId}" title="${t('discovery.view_on_nexus')}">${getIconSvg('externalLink', 'pmm-icon-xs')} <span>Nexus</span></button>`;

    return `
      <div class="discovery-card ${isBlurred ? 'nsfw-blurred' : ''}" data-mod-id="${m.modId}">
        <div class="discovery-card-cover-wrap">
          <img src="${escapeHtml(pic)}" alt="${escapeHtml(m.name)}" class="discovery-card-cover" loading="lazy" data-original-src="${escapeHtml(pic)}" onerror="window.handleDiscoveryImageError ? window.handleDiscoveryImageError(this, this.dataset.originalSrc || '${escapeHtml(pic)}') : (this.onerror=null, this.src='${logoUrl}');" />
          <div class="discovery-card-badges">
            <span class="discovery-badge-category">${escapeHtml(cat)}</span>
            <div class="discovery-card-right-badges">
              ${isNsfw ? `<span class="discovery-badge-nsfw">${getIconSvg('alertTriangle', 'pmm-icon-xs')} <span>NSFW</span></span>` : ''}
              <span class="discovery-badge-version">v${escapeHtml(m.version)}</span>
            </div>
          </div>
        </div>
        <div class="discovery-card-content">
          <h3 class="discovery-card-title" title="${escapeHtml(m.name)}">${escapeHtml(m.name)}</h3>
          <div class="discovery-card-author">${t('discovery.by_author')} <strong>${escapeHtml(m.author)}</strong></div>
          <p class="discovery-card-summary">${escapeHtml(m.summary || 'No description.')}</p>
          
          <div class="discovery-card-dates">
            <span title="Uploaded date">${getIconSvg('calendar', 'pmm-icon-xs')} <span>${uploadedDate || 'N/A'}</span></span>
            <span title="Updated date">${getIconSvg('clock', 'pmm-icon-xs')} <span>${updatedDate || uploadedDate || 'N/A'}</span></span>
          </div>

          <div class="discovery-card-footer">
            <div class="discovery-card-stats">
              <span class="discovery-card-stat" title="Endorsements">${getIconSvg('thumbsUp', 'pmm-icon-xs')} <span>${m.endorsements.toLocaleString()}</span></span>
              <span class="discovery-card-stat" title="Downloads">${getIconSvg('download', 'pmm-icon-xs')} <span>${m.downloads.toLocaleString()}</span></span>
            </div>
            ${actionBtnHtml}
          </div>
        </div>
      </div>
    `;
  }).join('');

  // Attach card click handlers
  grid.querySelectorAll('.discovery-card').forEach((card) => {
    const modId = parseInt((card as HTMLElement).dataset.modId || '0', 10);
    if (!modId) return;

    const modItem = mods.find((m) => m.modId === modId);

    card.addEventListener('click', (e) => {
      const target = e.target as HTMLElement;
      if (target.closest('[data-action="quick-install"]')) {
        e.stopPropagation();
        openModDetails(modId, true, modItem);
        return;
      }
      if (target.closest('[data-action="view-nexus"]')) {
        e.stopPropagation();
        openUrl(`https://www.nexusmods.com/palworld/mods/${modId}?tab=files`);
        return;
      }
      openModDetails(modId, false, modItem);
    });
  });
}
