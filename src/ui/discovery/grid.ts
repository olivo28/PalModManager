import { type DiscoveryModItem, openUrl } from '../../api';
import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';
import logoUrl from '../../assets/logo.png';
import { discState, isUserPremium } from './state';
import { formatDateDisplay } from './helpers';
import { openModDetails } from './detailsModal';
import { discoveryDom } from '../../framework';

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
      ? `<button class="discovery-card-btn-install" data-action="quick-install" data-mod-id="${m.modId}" title="${t('discovery.btn_quick_install')}">⚡ ${t('discovery.install_short')}</button>`
      : `<button class="discovery-card-btn-install" data-action="view-nexus" data-mod-id="${m.modId}" title="${t('discovery.view_on_nexus')}">🌐 ${t('discovery.view_on_nexus')}</button>`;

    return `
      <div class="discovery-card ${isBlurred ? 'nsfw-blurred' : ''}" data-mod-id="${m.modId}">
        <div class="discovery-card-cover-wrap">
          <img src="${escapeHtml(pic)}" alt="${escapeHtml(m.name)}" class="discovery-card-cover" loading="lazy" data-original-src="${escapeHtml(pic)}" onerror="window.handleDiscoveryImageError ? window.handleDiscoveryImageError(this, this.dataset.originalSrc || '${escapeHtml(pic)}') : (this.onerror=null, this.src='${logoUrl}');" />
          <div class="discovery-card-badges">
            <span class="discovery-badge-category">${escapeHtml(cat)}</span>
            <div class="discovery-card-right-badges">
              ${isNsfw ? '<span class="discovery-badge-nsfw">🔞 NSFW</span>' : ''}
              <span class="discovery-badge-version">v${escapeHtml(m.version)}</span>
            </div>
          </div>
        </div>
        <div class="discovery-card-content">
          <h3 class="discovery-card-title" title="${escapeHtml(m.name)}">${escapeHtml(m.name)}</h3>
          <div class="discovery-card-author">${t('discovery.by_author')} <strong>${escapeHtml(m.author)}</strong></div>
          <p class="discovery-card-summary">${escapeHtml(m.summary || 'No description.')}</p>
          
          <div class="discovery-card-dates">
            <span title="Uploaded date">📅 ${uploadedDate || 'N/A'}</span>
            <span title="Updated date">🔄 ${updatedDate || uploadedDate || 'N/A'}</span>
          </div>

          <div class="discovery-card-footer">
            <div class="discovery-card-stats">
              <span class="discovery-card-stat" title="Endorsements">👍 ${m.endorsements.toLocaleString()}</span>
              <span class="discovery-card-stat" title="Downloads">📥 ${m.downloads.toLocaleString()}</span>
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
