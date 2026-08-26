import {
  getDiscoveryCategories,
  getDiscoveryMods,
  getDiscoveryModDetails,
  installDiscoveryFile,
  endorseNexusMod,
  abstainNexusMod,
  trackNexusMod,
  untrackNexusMod,
  openUrl,
  fetchAndCacheImage,
  type DiscoveryCategory,
  type DiscoveryModItem,
  type DiscoveryModDetails,
  type DiscoveryFileItem,
} from '../api';
import { convertFileSrc } from '@tauri-apps/api/core';
import { enqueueDiscoveryDownload } from '../features/nxm_queue';
import { getState } from '../state';
import { showToast } from './toast';
import { t } from '../utils/i18n';
import { escapeHtml } from '../utils/helpers';
import { descriptionToHtml } from '../utils/bbcode';
import logoUrl from '../assets/logo.png';

export async function handleDiscoveryImageError(img: HTMLImageElement, originalUrl: string): Promise<void> {
  img.onerror = null;
  if (!originalUrl || originalUrl === logoUrl || !originalUrl.startsWith('http')) {
    img.src = logoUrl;
    return;
  }
  try {
    const cachedPath = await fetchAndCacheImage(originalUrl);
    if (cachedPath) {
      img.src = convertFileSrc(cachedPath);
      return;
    }
  } catch (e) {
    console.error('DoH proxy fetch failed:', e);
  }
  img.src = logoUrl;
}

if (typeof window !== 'undefined') {
  (window as any).handleDiscoveryImageError = handleDiscoveryImageError;
}

function isUserPremium(): boolean {
  const account: any = getState().currentSettings?.nexusAccount;
  return !!(account?.isPremium || account?.is_premium);
}

let categories: DiscoveryCategory[] = [];
let currentQuery = '';
let currentCategoryId: number | null = null;
let currentSort = localStorage.getItem('pmm-discovery-sort') || 'date_published';
let currentTimeRange = localStorage.getItem('pmm-discovery-timerange') || 'all';
let currentNsfwFilter: 'hide' | 'blur' | 'show' = (localStorage.getItem('pmm-discovery-nsfw') as any) || 'hide';
let currentPage = 1;
const PAGE_SIZE = 36;
let totalCount = 0;
let isLoading = false;
let isInitialized = false;
let currentModalMod: DiscoveryModDetails | null = null;

// Lightbox state
let lightboxZoom = 1;
let lightboxPanX = 0;
let lightboxPanY = 0;
let isPanning = false;
let panStartX = 0;
let panStartY = 0;

const NEXUS_PALWORLD_TAGS = [
  "2 Players", "3-4 Players", "5-6 Players", "7-8 Players", "9+ Players",
  "AI Assisted", "AI Media", "AI-Generated Content",
  "Animation - Modified", "Animation - New", "Bug Fixes", "Camera",
  "Character Preset", "Cheating", "Chinese", "Collection Asset",
  "Compatibility Patch", "Compilation", "Dutch", "English",
  "Extreme violence", "Face", "Fair and balanced", "Foliage (Plants)",
  "French", "Gameplay", "German", "Hair", "Horror",
  "Humour, Joke or Just for Fun", "ini tweak", "Italian", "Japanese",
  "Korean", "Mandarin", "Polish", "Portuguese", "Russian", "Spanish", "Ukrainian"
];

let selectedIncludeTags: Set<string> = new Set();
let selectedExcludeTags: Set<string> = new Set();

function renderTagChips(type: 'include' | 'exclude'): void {
  const container = document.getElementById(type === 'include' ? 'tags-include-chips' : 'tags-exclude-chips');
  if (!container) return;

  const set = type === 'include' ? selectedIncludeTags : selectedExcludeTags;
  if (set.size === 0) {
    container.innerHTML = '';
    return;
  }

  container.innerHTML = Array.from(set).map((tag) => `
    <span class="discovery-tag-chip ${type === 'exclude' ? 'exclude' : ''}" data-tag="${escapeHtml(tag)}">
      ${escapeHtml(tag)}
      <span class="discovery-tag-chip-remove" data-type="${type}" data-tag="${escapeHtml(tag)}">×</span>
    </span>
  `).join('');

  container.querySelectorAll('.discovery-tag-chip-remove').forEach((btn) => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const t = (btn as HTMLElement).dataset.type as 'include' | 'exclude';
      const tag = (btn as HTMLElement).dataset.tag;
      if (tag) {
        if (t === 'include') selectedIncludeTags.delete(tag);
        else selectedExcludeTags.delete(tag);
        renderTagChips(t);
        renderTagMenu(t);
      }
    });
  });
}

function renderTagMenu(type: 'include' | 'exclude', filterText: string = ''): void {
  const menu = document.getElementById(type === 'include' ? 'tags-include-menu' : 'tags-exclude-menu');
  if (!menu) return;

  const set = type === 'include' ? selectedIncludeTags : selectedExcludeTags;
  const filtered = NEXUS_PALWORLD_TAGS.filter((t) => t.toLowerCase().includes(filterText.toLowerCase()));

  if (filtered.length === 0) {
    menu.innerHTML = `<div style="padding: 6px 8px; font-size: 11px; color: var(--text-muted);">No matching tags</div>`;
    return;
  }

  menu.innerHTML = filtered.map((tag) => {
    const isSelected = set.has(tag);
    return `
      <div class="discovery-tag-menu-item ${isSelected ? `selected ${type === 'exclude' ? 'exclude' : ''}` : ''}" data-type="${type}" data-tag="${escapeHtml(tag)}">
        <span>${escapeHtml(tag)}</span>
        ${isSelected ? `<span class="discovery-tag-menu-item-check">✓</span>` : ''}
      </div>
    `;
  }).join('');

  menu.querySelectorAll('.discovery-tag-menu-item').forEach((item) => {
    item.addEventListener('click', (e) => {
      e.stopPropagation();
      const t = (item as HTMLElement).dataset.type as 'include' | 'exclude';
      const tag = (item as HTMLElement).dataset.tag;
      if (tag) {
        const targetSet = t === 'include' ? selectedIncludeTags : selectedExcludeTags;
        if (targetSet.has(tag)) {
          targetSet.delete(tag);
        } else {
          targetSet.add(tag);
        }
        renderTagChips(t);
        renderTagMenu(t, filterText);
      }
    });
  });
}

function setupTagPickers(): void {
  const incSearch = document.getElementById('tags-include-search') as HTMLInputElement | null;
  const incMenu = document.getElementById('tags-include-menu');
  const excSearch = document.getElementById('tags-exclude-search') as HTMLInputElement | null;
  const excMenu = document.getElementById('tags-exclude-menu');

  if (incSearch && incMenu) {
    incSearch.addEventListener('focus', () => {
      renderTagMenu('include', incSearch.value);
      incMenu.style.display = 'flex';
    });
    incSearch.addEventListener('input', () => {
      renderTagMenu('include', incSearch.value);
      incMenu.style.display = 'flex';
    });
  }

  if (excSearch && excMenu) {
    excSearch.addEventListener('focus', () => {
      renderTagMenu('exclude', excSearch.value);
      excMenu.style.display = 'flex';
    });
    excSearch.addEventListener('input', () => {
      renderTagMenu('exclude', excSearch.value);
      excMenu.style.display = 'flex';
    });
  }

  // Close menus when clicking outside
  document.addEventListener('click', (e) => {
    if (incMenu && !incMenu.contains(e.target as Node) && e.target !== incSearch) {
      incMenu.style.display = 'none';
    }
    if (excMenu && !excMenu.contains(e.target as Node) && e.target !== excSearch) {
      excMenu.style.display = 'none';
    }
  });
}

export async function renderDiscoveryView(): Promise<void> {
  setupEventListeners();

  if (categories.length === 0) {
    await loadCategories();
  }

  await loadMods();
}

function setupEventListeners(): void {
  if (isInitialized) return;
  isInitialized = true;

  // Search input debounce
  const searchInput = document.getElementById('discovery-search-input') as HTMLInputElement | null;
  const searchClear = document.getElementById('discovery-search-clear');
  let debounceTimeout: any = null;

  if (searchInput) {
    searchInput.addEventListener('input', () => {
      const val = searchInput.value.trim();
      if (searchClear) {
        searchClear.style.display = val ? 'block' : 'none';
      }
      clearTimeout(debounceTimeout);
      debounceTimeout = setTimeout(() => {
        currentQuery = val;
        currentPage = 1;
        loadMods();
      }, 350);
    });
  }

  if (searchClear && searchInput) {
    searchClear.addEventListener('click', () => {
      searchInput.value = '';
      searchClear.style.display = 'none';
      currentQuery = '';
      currentPage = 1;
      loadMods();
    });
  }

  // Category select
  const catSelect = document.getElementById('discovery-category-select') as HTMLSelectElement | null;
  if (catSelect) {
    catSelect.addEventListener('change', () => {
      const val = catSelect.value;
      currentCategoryId = val ? parseInt(val, 10) : null;
      currentPage = 1;
      loadMods();
    });
  }

  // Time Range select
  const timeSelect = document.getElementById('discovery-time-select') as HTMLSelectElement | null;
  if (timeSelect) {
    timeSelect.value = currentTimeRange;
    timeSelect.addEventListener('change', () => {
      currentTimeRange = timeSelect.value;
      localStorage.setItem('pmm-discovery-timerange', currentTimeRange);
      currentPage = 1;
      loadMods();
    });
  }

  // Sort select
  const sortSelect = document.getElementById('discovery-sort-select') as HTMLSelectElement | null;
  if (sortSelect) {
    sortSelect.value = currentSort;
    sortSelect.addEventListener('change', () => {
      currentSort = sortSelect.value;
      localStorage.setItem('pmm-discovery-sort', currentSort);
      currentPage = 1;
      loadMods();
    });
  }

  // NSFW select
  const nsfwSelect = document.getElementById('discovery-nsfw-select') as HTMLSelectElement | null;
  if (nsfwSelect) {
    nsfwSelect.value = currentNsfwFilter;
    nsfwSelect.addEventListener('change', () => {
      currentNsfwFilter = nsfwSelect.value as 'hide' | 'blur' | 'show';
      localStorage.setItem('pmm-discovery-nsfw', currentNsfwFilter);
      currentPage = 1;
      loadMods();
    });
  }

  // Refresh btn
  document.getElementById('discovery-refresh-btn')?.addEventListener('click', () => {
    loadMods();
  });

  // Pagination buttons
  document.getElementById('discovery-prev-page')?.addEventListener('click', () => {
    if (currentPage > 1) {
      currentPage--;
      loadMods();
      scrollToTop();
    }
  });

  document.getElementById('discovery-next-page')?.addEventListener('click', () => {
    if (currentPage * PAGE_SIZE < totalCount) {
      currentPage++;
      loadMods();
      scrollToTop();
    }
  });

  // Modal close
  document.getElementById('discovery-modal-close')?.addEventListener('click', () => {
    closeDiscoveryModal();
  });

  const modalOverlay = document.getElementById('discovery-mod-modal');
  if (modalOverlay) {
    modalOverlay.addEventListener('click', (e) => {
      if (e.target === modalOverlay) {
        closeDiscoveryModal();
      }
    });
  }

  // Global Escape key listener for Discovery modals
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      const imgModal = document.getElementById('discovery-image-modal');
      if (imgModal && imgModal.classList.contains('visible')) {
        closeLightbox();
        return;
      }
      const modModal = document.getElementById('discovery-mod-modal');
      if (modModal && modModal.classList.contains('visible')) {
        closeDiscoveryModal();
      }
    }
  });

  // Modal sub-tabs
  document.querySelectorAll('.discovery-modal-tab').forEach((tabBtn) => {
    tabBtn.addEventListener('click', () => {
      const tabName = (tabBtn as HTMLElement).dataset.tab;
      document.querySelectorAll('.discovery-modal-tab').forEach((b) => b.classList.remove('active'));
      tabBtn.classList.add('active');

      document.querySelectorAll('.discovery-tab-pane').forEach((p) => {
        (p as HTMLElement).style.display = 'none';
        p.classList.remove('active');
      });

      const pane = document.getElementById(`discovery-tab-${tabName}`);
      if (pane) {
        pane.style.display = 'block';
        pane.classList.add('active');
      }
    });
  });

  // Lightbox close & controls
  document.getElementById('discovery-lightbox-close')?.addEventListener('click', () => {
    closeLightbox();
  });

  const lightboxOverlay = document.getElementById('discovery-image-modal');
  if (lightboxOverlay) {
    lightboxOverlay.addEventListener('click', (e) => {
      if (e.target === lightboxOverlay) {
        closeLightbox();
      }
    });
  }

  document.getElementById('discovery-lightbox-zoom-in')?.addEventListener('click', () => {
    updateLightboxTransform(lightboxZoom + 0.3, lightboxPanX, lightboxPanY);
  });
  document.getElementById('discovery-lightbox-zoom-out')?.addEventListener('click', () => {
    updateLightboxTransform(lightboxZoom - 0.3, lightboxPanX, lightboxPanY);
  });
  document.getElementById('discovery-lightbox-zoom-reset')?.addEventListener('click', () => {
    updateLightboxTransform(1, 0, 0);
  });

  // Mouse wheel zoom & Drag pan setup
  setupLightboxPanZoom();

  // Cover image click opens lightbox
  document.getElementById('discovery-modal-img')?.addEventListener('click', () => {
    const img = document.getElementById('discovery-modal-img') as HTMLImageElement | null;
    if (img && img.src && !img.src.includes('logo') && img.src !== logoUrl) {
      openLightbox(img.src);
    }
  });

  setupTagPickers();

  // Advanced Filter Sidebar setup
  const toggleSidebarBtn = document.getElementById('discovery-toggle-filters-btn');
  const sidebar = document.getElementById('discovery-filters-sidebar');
  const closeSidebarBtn = document.getElementById('discovery-sidebar-close-btn');
  const applySidebarBtn = document.getElementById('discovery-sidebar-apply-btn');
  const searchApplySidebarBtn = document.getElementById('discovery-sidebar-search-apply-btn');
  const resetSidebarBtn = document.getElementById('discovery-sidebar-reset-btn');

  if (toggleSidebarBtn && sidebar) {
    toggleSidebarBtn.addEventListener('click', () => {
      const isVisible = sidebar.style.display !== 'none';
      sidebar.style.display = isVisible ? 'none' : 'flex';
      toggleSidebarBtn.classList.toggle('active', !isVisible);
    });
  }

  if (closeSidebarBtn && sidebar && toggleSidebarBtn) {
    closeSidebarBtn.addEventListener('click', () => {
      sidebar.style.display = 'none';
      toggleSidebarBtn.classList.remove('active');
    });
  }

  if (applySidebarBtn) {
    applySidebarBtn.addEventListener('click', () => {
      currentPage = 1;
      loadMods();
    });
  }

  if (searchApplySidebarBtn) {
    searchApplySidebarBtn.addEventListener('click', () => {
      currentPage = 1;
      loadMods();
    });
  }

  if (resetSidebarBtn) {
    resetSidebarBtn.addEventListener('click', () => {
      const titleInp = document.getElementById('filter-title-contains') as HTMLInputElement | null;
      const descInp = document.getElementById('filter-desc-contains') as HTMLInputElement | null;
      const authorInp = document.getElementById('filter-author-contains') as HTMLInputElement | null;
      const uploaderInp = document.getElementById('filter-uploader-contains') as HTMLInputElement | null;
      const vortexCb = document.getElementById('filter-supports-vortex') as HTMLInputElement | null;
      const updatedCb = document.getElementById('filter-has-updated') as HTMLInputElement | null;
      const hideAdultCb = document.getElementById('filter-hide-adult') as HTMLInputElement | null;
      const onlyAdultCb = document.getElementById('filter-only-adult') as HTMLInputElement | null;
      const hideTransCb = document.getElementById('filter-hide-translations') as HTMLInputElement | null;
      const incSearch = document.getElementById('tags-include-search') as HTMLInputElement | null;
      const excSearch = document.getElementById('tags-exclude-search') as HTMLInputElement | null;

      if (titleInp) titleInp.value = '';
      if (descInp) descInp.value = '';
      if (authorInp) authorInp.value = '';
      if (uploaderInp) uploaderInp.value = '';
      if (vortexCb) vortexCb.checked = false;
      if (updatedCb) updatedCb.checked = false;
      if (hideAdultCb) hideAdultCb.checked = false;
      if (onlyAdultCb) onlyAdultCb.checked = false;
      if (hideTransCb) hideTransCb.checked = false;
      if (incSearch) incSearch.value = '';
      if (excSearch) excSearch.value = '';

      selectedIncludeTags.clear();
      selectedExcludeTags.clear();
      renderTagChips('include');
      renderTagChips('exclude');

      document.querySelectorAll<HTMLInputElement>('.filter-lang-cb').forEach((cb) => (cb.checked = false));

      currentPage = 1;
      loadMods();
    });
  }

  // Global ESC key listener
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      const lightbox = document.getElementById('discovery-image-modal');
      if (lightbox && (lightbox.classList.contains('visible') || (lightbox.style.display && lightbox.style.display !== 'none'))) {
        e.preventDefault();
        e.stopPropagation();
        closeLightbox();
        return;
      }
      const modal = document.getElementById('discovery-mod-modal');
      if (modal && (modal.classList.contains('visible') || (modal.style.display && modal.style.display !== 'none'))) {
        e.preventDefault();
        e.stopPropagation();
        closeDiscoveryModal();
      }
    }
  }, true);
}

function setupLightboxPanZoom(): void {
  const wrap = document.getElementById('discovery-lightbox-img-wrap');
  if (!wrap) return;

  let dragOriginX = 0;
  let dragOriginY = 0;
  let startPanX = 0;
  let startPanY = 0;

  // Mouse wheel zoom
  wrap.addEventListener('wheel', (e: WheelEvent) => {
    e.preventDefault();
    const factor = e.deltaY < 0 ? 1.15 : 0.87;
    const newZoom = Math.max(0.5, Math.min(6.0, lightboxZoom * factor));
    updateLightboxTransform(newZoom, lightboxPanX, lightboxPanY);
  }, { passive: false });

  // Drag to pan
  wrap.addEventListener('mousedown', (e: MouseEvent) => {
    if (e.button !== 0) return; // Only left click
    e.preventDefault();
    isPanning = true;
    dragOriginX = e.clientX;
    dragOriginY = e.clientY;
    startPanX = lightboxPanX;
    startPanY = lightboxPanY;
    wrap.classList.add('panning');
  });

  window.addEventListener('mousemove', (e: MouseEvent) => {
    if (!isPanning) return;
    e.preventDefault();
    const dx = e.clientX - dragOriginX;
    const dy = e.clientY - dragOriginY;
    lightboxPanX = startPanX + dx;
    lightboxPanY = startPanY + dy;
    updateLightboxTransform(lightboxZoom, lightboxPanX, lightboxPanY);
  });

  const stopPanning = () => {
    if (isPanning) {
      isPanning = false;
      wrap?.classList.remove('panning');
    }
  };

  window.addEventListener('mouseup', stopPanning);
  window.addEventListener('pointerup', stopPanning);
  window.addEventListener('mouseleave', stopPanning);
  window.addEventListener('dragend', stopPanning);

  // Double click toggle zoom
  wrap.addEventListener('dblclick', (e: MouseEvent) => {
    e.preventDefault();
    if (lightboxZoom > 1.2) {
      updateLightboxTransform(1, 0, 0);
    } else {
      updateLightboxTransform(2.2, 0, 0);
    }
  });
}

function updateLightboxTransform(zoom: number, panX: number, panY: number): void {
  lightboxZoom = Math.max(0.5, Math.min(6.0, zoom));
  lightboxPanX = panX;
  lightboxPanY = panY;

  const img = document.getElementById('discovery-lightbox-img');
  if (img) {
    img.style.transform = `translate(${lightboxPanX}px, ${lightboxPanY}px) scale(${lightboxZoom})`;
  }

  const resetBtn = document.getElementById('discovery-lightbox-zoom-reset');
  if (resetBtn) {
    resetBtn.textContent = `${Math.round(lightboxZoom * 100)}%`;
  }
}

function scrollToTop(): void {
  const body = document.getElementById('discovery-body');
  if (body) body.scrollTop = 0;
}

async function loadCategories(): Promise<void> {
  try {
    categories = await getDiscoveryCategories();
    const select = document.getElementById('discovery-category-select') as HTMLSelectElement | null;
    if (select && categories.length > 0) {
      select.innerHTML = `<option value="">${t('discovery.all_categories')}</option>` +
        categories.map((c) => `<option value="${c.categoryId}">${escapeHtml(c.name)}</option>`).join('');
    }
  } catch (err) {
    console.warn('[Discovery] Failed to load categories:', err);
  }
}

async function loadMods(): Promise<void> {
  if (isLoading) return;
  isLoading = true;

  const grid = document.getElementById('discovery-grid');
  const loadingEl = document.getElementById('discovery-loading');
  const emptyEl = document.getElementById('discovery-empty');
  const paginationEl = document.getElementById('discovery-pagination');

  if (loadingEl) loadingEl.style.display = 'flex';
  if (emptyEl) emptyEl.style.display = 'none';
  if (grid) grid.style.display = 'none';

  try {
    const selectedCat = currentCategoryId ? categories.find((c) => c.categoryId === currentCategoryId) : undefined;
    const titleVal = (document.getElementById('filter-title-contains') as HTMLInputElement)?.value.trim();
    const descVal = (document.getElementById('filter-desc-contains') as HTMLInputElement)?.value.trim();
    const authorVal = (document.getElementById('filter-author-contains') as HTMLInputElement)?.value.trim();
    const uploaderVal = (document.getElementById('filter-uploader-contains') as HTMLInputElement)?.value.trim();
    const vortexChecked = (document.getElementById('filter-supports-vortex') as HTMLInputElement)?.checked;
    const updatedChecked = (document.getElementById('filter-has-updated') as HTMLInputElement)?.checked;
    const hideAdultChecked = (document.getElementById('filter-hide-adult') as HTMLInputElement)?.checked;
    const onlyAdultChecked = (document.getElementById('filter-only-adult') as HTMLInputElement)?.checked;
    const hideTransChecked = (document.getElementById('filter-hide-translations') as HTMLInputElement)?.checked;

    // Read selected languages
    const selectedLangs = Array.from(document.querySelectorAll<HTMLInputElement>('.filter-lang-cb:checked')).map((cb) => cb.value);

    let adultFilter = currentNsfwFilter;
    if (onlyAdultChecked) {
      adultFilter = 'show';
    } else if (hideAdultChecked) {
      adultFilter = 'hide';
    }

    const res = await getDiscoveryMods({
      query: titleVal || currentQuery || undefined,
      descriptionQuery: descVal || undefined,
      authorQuery: authorVal || undefined,
      uploaderQuery: uploaderVal || undefined,
      categoryId: currentCategoryId || undefined,
      categoryName: selectedCat ? selectedCat.name : undefined,
      supportsVortex: vortexChecked ? true : undefined,
      hasUpdated: updatedChecked ? true : undefined,
      languageNames: selectedLangs.length > 0 ? selectedLangs : undefined,
      includeTags: selectedIncludeTags.size > 0 ? Array.from(selectedIncludeTags) : undefined,
      excludeTags: selectedExcludeTags.size > 0 ? Array.from(selectedExcludeTags) : undefined,
      hideTranslations: hideTransChecked ? true : undefined,
      sortBy: currentSort,
      timeRange: currentTimeRange !== 'all' ? currentTimeRange : undefined,
      includeAdult: adultFilter !== 'hide',
      adultFilter: adultFilter,
      page: currentPage,
      pageSize: PAGE_SIZE,
    });

    totalCount = res.totalCount;
    if (loadingEl) loadingEl.style.display = 'none';

    if (!res.mods || res.mods.length === 0) {
      if (emptyEl) emptyEl.style.display = 'flex';
      if (paginationEl) paginationEl.style.display = 'none';
      return;
    }

    if (grid) {
      grid.style.display = 'grid';
      renderGrid(res.mods);
    }

    // Update pagination
    if (paginationEl) {
      const totalPages = Math.max(1, Math.ceil(totalCount / PAGE_SIZE));
      if (totalCount > PAGE_SIZE) {
        paginationEl.style.display = 'flex';
        const prevBtn = document.getElementById('discovery-prev-page') as HTMLButtonElement | null;
        const nextBtn = document.getElementById('discovery-next-page') as HTMLButtonElement | null;
        const pageIndicator = document.getElementById('discovery-page-indicator');

        if (prevBtn) prevBtn.disabled = currentPage <= 1;
        if (nextBtn) nextBtn.disabled = currentPage >= totalPages;
        if (pageIndicator) pageIndicator.textContent = `${t('discovery.page_label', { current: currentPage, total: totalPages })}`;
      } else {
        paginationEl.style.display = 'none';
      }
    }
  } catch (err: any) {
    console.error('[Discovery] Failed to load mods:', err);
    if (loadingEl) loadingEl.style.display = 'none';
    if (emptyEl) {
      emptyEl.style.display = 'flex';
      const desc = emptyEl.querySelector('p');
      if (desc) desc.textContent = String(err);
    }
  } finally {
    isLoading = false;
  }
}

function formatDateDisplay(dateStr: string): string {
  if (!dateStr) return '';
  try {
    const d = new Date(dateStr);
    if (isNaN(d.getTime())) return '';
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  } catch {
    return '';
  }
}

function renderGrid(mods: DiscoveryModItem[]): void {
  const grid = document.getElementById('discovery-grid');
  if (!grid) return;

  const isPremium = isUserPremium();

  grid.innerHTML = mods.map((m) => {
    const pic = m.pictureUrl && m.pictureUrl.trim() !== '' ? m.pictureUrl : logoUrl;
    const cat = m.categoryName || (m.categoryId ? categories.find((c) => c.categoryId === m.categoryId)?.name : '') || 'Mod';
    const isNsfw = m.containsAdultContent || false;
    const isBlurred = isNsfw && currentNsfwFilter === 'blur';

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

export async function openModDetails(
  modId: number,
  autoInstallFirstPrimary: boolean = false,
  previewData?: Partial<DiscoveryModItem>
): Promise<void> {
  setupEventListeners();

  const modal = document.getElementById('discovery-mod-modal');
  if (!modal) return;

  modal.classList.add('visible');
  modal.classList.add('active');
  modal.style.display = 'flex';

  resetModalUI(previewData);

  try {
    const details = await getDiscoveryModDetails(modId);
    currentModalMod = details;
    populateModalData(details);

    if (autoInstallFirstPrimary && details.files.length > 0) {
      if (isUserPremium()) {
        // Switch to files tab
        const filesTabBtn = document.querySelector('.discovery-modal-tab[data-tab="files"]') as HTMLElement | null;
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

function resetModalUI(previewData?: Partial<DiscoveryModItem>): void {
  const title = document.getElementById('discovery-modal-title');
  const author = document.getElementById('discovery-modal-author');
  const cat = document.getElementById('discovery-modal-category');
  const ver = document.getElementById('discovery-modal-version');
  const installedBadge = document.getElementById('discovery-modal-installed-badge');
  const updated = document.getElementById('discovery-modal-updated');
  const endorsements = document.getElementById('discovery-modal-endorsements');
  const downloads = document.getElementById('discovery-modal-downloads');
  const desc = document.getElementById('discovery-modal-description');
  const filesList = document.getElementById('discovery-files-list');
  const gallery = document.getElementById('discovery-media-gallery');
  const img = document.getElementById('discovery-modal-img') as HTMLImageElement | null;

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
  document.querySelectorAll('.discovery-modal-tab').forEach((b) => b.classList.remove('active'));
  document.querySelector('.discovery-modal-tab[data-tab="desc"]')?.classList.add('active');

  document.querySelectorAll('.discovery-tab-pane').forEach((p) => {
    (p as HTMLElement).style.display = 'none';
    p.classList.remove('active');
  });
  const descPane = document.getElementById('discovery-tab-desc');
  if (descPane) {
    descPane.style.display = 'block';
    descPane.classList.add('active');
  }
}

function populateModalData(details: DiscoveryModDetails): void {
  const title = document.getElementById('discovery-modal-title');
  const author = document.getElementById('discovery-modal-author');
  const cat = document.getElementById('discovery-modal-category');
  const ver = document.getElementById('discovery-modal-version');
  const installedBadge = document.getElementById('discovery-modal-installed-badge');
  const updated = document.getElementById('discovery-modal-updated');
  const endorsements = document.getElementById('discovery-modal-endorsements');
  const downloads = document.getElementById('discovery-modal-downloads');
  const desc = document.getElementById('discovery-modal-description');
  const filesList = document.getElementById('discovery-files-list');
  const gallery = document.getElementById('discovery-media-gallery');
  const img = document.getElementById('discovery-modal-img') as HTMLImageElement | null;

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
  const filesInstruction = document.getElementById('discovery-files-instruction-text');
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
  const filesContainer = document.getElementById('discovery-files-list');
  if (filesContainer) {
    renderFilesList(details, filesContainer);
  }

  // 3. Changelogs Tab
  const changelogsContainer = document.getElementById('discovery-modal-changelogs');
  if (changelogsContainer) {
    renderChangelogsList(details, changelogsContainer);
  }

  // 4. Media gallery (screenshots)
  const galleryContainer = document.getElementById('discovery-media-gallery');
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

function renderChangelogsList(details: DiscoveryModDetails, container: HTMLElement): void {
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

function renderFilesList(details: DiscoveryModDetails, container: HTMLElement): void {
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

function setupModalActions(details: DiscoveryModDetails): void {
  const endorseBtn = document.getElementById('discovery-modal-endorse-btn');
  const endorseText = document.getElementById('discovery-modal-endorse-text');
  const trackBtn = document.getElementById('discovery-modal-track-btn');
  const trackText = document.getElementById('discovery-modal-track-text');
  const communityBtn = document.getElementById('discovery-modal-community-btn');
  const bugsBtn = document.getElementById('discovery-modal-bugs-btn');
  const nexusLinkBtn = document.getElementById('discovery-modal-nexus-link-btn');

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

async function executeInstallFile(mod: DiscoveryModDetails, file: DiscoveryFileItem): Promise<void> {
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
      mod.pictureUrl
    );
  } catch (err: any) {
    console.error('[Discovery] Direct installation error:', err);
    showToast(t('discovery.install_failed', { error: String(err) }), 'error');
  }
}

export function closeDiscoveryModal(): void {
  const modal = document.getElementById('discovery-mod-modal');
  if (modal) {
    modal.classList.remove('visible');
    modal.classList.remove('active');
    modal.style.display = 'none';
  }
  currentModalMod = null;
}

/* LIGHTBOX IMAGE VIEWER WITH ZOOM & PAN */
function openLightbox(src: string): void {
  const lightbox = document.getElementById('discovery-image-modal');
  const img = document.getElementById('discovery-lightbox-img') as HTMLImageElement | null;
  if (!lightbox || !img) return;

  img.src = src;
  img.setAttribute('data-original-src', src);
  img.onerror = () => {
    if ((window as any).handleUniversalImageFallback) {
      (window as any).handleUniversalImageFallback(img);
    }
  };
  updateLightboxTransform(1, 0, 0);

  lightbox.classList.add('visible');
  lightbox.style.display = 'flex';
}

function closeLightbox(): void {
  const lightbox = document.getElementById('discovery-image-modal');
  if (lightbox) {
    lightbox.classList.remove('visible');
    lightbox.style.display = 'none';
  }
  const img = document.getElementById('discovery-lightbox-img') as HTMLImageElement | null;
  if (img) img.src = '';
  updateLightboxTransform(1, 0, 0);
}
