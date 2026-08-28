import { getDiscoveryCategories, getDiscoveryMods } from '../../api';
import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';
import logoUrl from '../../assets/logo.png';
import { discState } from './state';
import { scrollToTop } from './helpers';
import { openLightbox, closeLightbox, updateLightboxTransform, setupLightboxPanZoom } from './lightbox';
import { renderTagChips, renderTagMenu, setupTagPickers } from './tags';
import { renderGrid } from './grid';
import { closeDiscoveryModal } from './detailsModal';
import { discoveryDom } from '../../framework';

export async function renderDiscoveryView(): Promise<void> {
  setupEventListeners();

  if (discState.categories.length === 0) {
    await loadCategories();
  }

  await loadMods();
}

export function setupEventListeners(): void {
  if (discState.isInitialized) return;
  discState.isInitialized = true;

  // Search input debounce
  const searchInput = discoveryDom.elMaybe('discovery-search-input');
  const searchClear = discoveryDom.elMaybe('discovery-search-clear');
  let debounceTimeout: any = null;

  if (searchInput) {
    searchInput.addEventListener('input', () => {
      const val = searchInput.value.trim();
      if (searchClear) {
        searchClear.style.display = val ? 'block' : 'none';
      }
      clearTimeout(debounceTimeout);
      debounceTimeout = setTimeout(() => {
        discState.currentQuery = val;
        discState.currentPage = 1;
        loadMods();
      }, 350);
    });
  }

  if (searchClear && searchInput) {
    searchClear.addEventListener('click', () => {
      searchInput.value = '';
      searchClear.style.display = 'none';
      discState.currentQuery = '';
      discState.currentPage = 1;
      loadMods();
    });
  }

  // Category select
  const catSelect = discoveryDom.elMaybe('discovery-category-select');
  if (catSelect) {
    catSelect.addEventListener('change', () => {
      const val = catSelect.value;
      discState.currentCategoryId = val ? parseInt(val, 10) : null;
      discState.currentPage = 1;
      loadMods();
    });
  }

  // Time Range select
  const timeSelect = discoveryDom.elMaybe('discovery-time-select');
  if (timeSelect) {
    timeSelect.value = discState.currentTimeRange;
    timeSelect.addEventListener('change', () => {
      discState.currentTimeRange = timeSelect.value;
      localStorage.setItem('pmm-discovery-timerange', discState.currentTimeRange);
      discState.currentPage = 1;
      loadMods();
    });
  }

  // Sort select
  const sortSelect = discoveryDom.elMaybe('discovery-sort-select');
  if (sortSelect) {
    sortSelect.value = discState.currentSort;
    sortSelect.addEventListener('change', () => {
      discState.currentSort = sortSelect.value;
      localStorage.setItem('pmm-discovery-sort', discState.currentSort);
      discState.currentPage = 1;
      loadMods();
    });
  }

  // NSFW select
  const nsfwSelect = discoveryDom.elMaybe('discovery-nsfw-select');
  if (nsfwSelect) {
    nsfwSelect.value = discState.currentNsfwFilter;
    nsfwSelect.addEventListener('change', () => {
      discState.currentNsfwFilter = nsfwSelect.value as 'hide' | 'blur' | 'show';
      localStorage.setItem('pmm-discovery-nsfw', discState.currentNsfwFilter);
      discState.currentPage = 1;
      loadMods();
    });
  }

  // Refresh btn
  discoveryDom.elMaybe('discovery-refresh-btn')?.addEventListener('click', () => {
    loadMods();
  });

  // Pagination buttons
  discoveryDom.elMaybe('discovery-prev-page')?.addEventListener('click', () => {
    if (discState.currentPage > 1) {
      discState.currentPage--;
      loadMods();
      scrollToTop();
    }
  });

  discoveryDom.elMaybe('discovery-next-page')?.addEventListener('click', () => {
    if (discState.currentPage * discState.PAGE_SIZE < discState.totalCount) {
      discState.currentPage++;
      loadMods();
      scrollToTop();
    }
  });

  // Modal close
  discoveryDom.elMaybe('discovery-modal-close')?.addEventListener('click', () => {
    closeDiscoveryModal();
  });

  const modalOverlay = discoveryDom.elMaybe('discovery-mod-modal');
  if (modalOverlay) {
    modalOverlay.addEventListener('click', (e) => {
      if (e.target === modalOverlay) {
        closeDiscoveryModal();
      }
    });
  }

  // Modal sub-tabs
  discoveryDom.queryAll('.discovery-modal-tab').forEach((tabBtn) => {
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
  discoveryDom.elMaybe('discovery-lightbox-close')?.addEventListener('click', () => {
    closeLightbox();
  });

  const lightboxOverlay = discoveryDom.elMaybe('discovery-image-modal');
  if (lightboxOverlay) {
    lightboxOverlay.addEventListener('click', (e) => {
      if (e.target === lightboxOverlay) {
        closeLightbox();
      }
    });
  }

  discoveryDom.elMaybe('discovery-lightbox-zoom-in')?.addEventListener('click', () => {
    updateLightboxTransform(discState.lightboxZoom + 0.3, discState.lightboxPanX, discState.lightboxPanY);
  });
  discoveryDom.elMaybe('discovery-lightbox-zoom-out')?.addEventListener('click', () => {
    updateLightboxTransform(discState.lightboxZoom - 0.3, discState.lightboxPanX, discState.lightboxPanY);
  });
  discoveryDom.elMaybe('discovery-lightbox-zoom-reset')?.addEventListener('click', () => {
    updateLightboxTransform(1, 0, 0);
  });

  // Mouse wheel zoom & Drag pan setup
  setupLightboxPanZoom();

  // Cover image click opens lightbox
  discoveryDom.elMaybe('discovery-modal-img')?.addEventListener('click', () => {
    const img = discoveryDom.elMaybe('discovery-modal-img');
    if (img && img.src && !img.src.includes('logo') && img.src !== logoUrl) {
      openLightbox(img.src);
    }
  });

  setupTagPickers();

  // Advanced Filter Sidebar setup
  const toggleSidebarBtn = discoveryDom.elMaybe('discovery-toggle-filters-btn');
  const sidebar = discoveryDom.elMaybe('discovery-filters-sidebar');
  const closeSidebarBtn = discoveryDom.elMaybe('discovery-sidebar-close-btn');
  const applySidebarBtn = discoveryDom.elMaybe('discovery-sidebar-apply-btn');
  const searchApplySidebarBtn = discoveryDom.elMaybe('discovery-sidebar-search-apply-btn');
  const resetSidebarBtn = discoveryDom.elMaybe('discovery-sidebar-reset-btn');

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
      discState.currentPage = 1;
      loadMods();
    });
  }

  if (searchApplySidebarBtn) {
    searchApplySidebarBtn.addEventListener('click', () => {
      discState.currentPage = 1;
      loadMods();
    });
  }

  if (resetSidebarBtn) {
    resetSidebarBtn.addEventListener('click', () => {
      const titleInp = discoveryDom.elMaybe('filter-title-contains');
      const descInp = discoveryDom.elMaybe('filter-desc-contains');
      const authorInp = discoveryDom.elMaybe('filter-author-contains');
      const uploaderInp = discoveryDom.elMaybe('filter-uploader-contains');
      const vortexCb = discoveryDom.elMaybe('filter-supports-vortex');
      const updatedCb = discoveryDom.elMaybe('filter-has-updated');
      const hideAdultCb = discoveryDom.elMaybe('filter-hide-adult');
      const onlyAdultCb = discoveryDom.elMaybe('filter-only-adult');
      const hideTransCb = discoveryDom.elMaybe('filter-hide-translations');
      const incSearch = discoveryDom.elMaybe('tags-include-search');
      const excSearch = discoveryDom.elMaybe('tags-exclude-search');

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

      discState.selectedIncludeTags.clear();
      discState.selectedExcludeTags.clear();
      renderTagChips('include');
      renderTagChips('exclude');

      discoveryDom.queryAll<HTMLInputElement>('.filter-lang-cb').forEach((cb) => (cb.checked = false));

      discState.currentPage = 1;
      loadMods();
    });
  }

  // Global ESC key listener for Discovery modals
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      // Do not intercept if a higher overlay is currently open on top of Discovery
      const higherModal = document.querySelector(
        '.confirm-overlay, #uasset-inspector-modal, #save-compare-modal, #file-tree-modal, #full-files-modal-overlay, #archive-view-modal, #archive-structure-modal, #config-diff-modal, #install-modal.visible, #settings-modal.visible, #profile-modal.visible, #about-modal.visible, #nexus-profile-modal.visible, #workshop-modal.visible'
      );
      if (higherModal) return;

      const lightbox = discoveryDom.elMaybe('discovery-image-modal');
      if (lightbox && (lightbox.classList.contains('visible') || (lightbox.style.display && lightbox.style.display !== 'none'))) {
        e.preventDefault();
        e.stopPropagation();
        closeLightbox();
        return;
      }
      const modal = discoveryDom.elMaybe('discovery-mod-modal');
      if (modal && (modal.classList.contains('visible') || (modal.style.display && modal.style.display !== 'none'))) {
        e.preventDefault();
        e.stopPropagation();
        closeDiscoveryModal();
      }
    }
  }, true);
}

export async function loadCategories(): Promise<void> {
  try {
    discState.categories = await getDiscoveryCategories();
    const select = discoveryDom.elMaybe('discovery-category-select');
    if (select && discState.categories.length > 0) {
      select.innerHTML = `<option value="">${t('discovery.all_categories')}</option>` +
        discState.categories.map((c) => `<option value="${c.categoryId}">${escapeHtml(c.name)}</option>`).join('');
    }
  } catch (err) {
    console.warn('[Discovery] Failed to load categories:', err);
  }
}

export async function loadMods(): Promise<void> {
  if (discState.isLoading) return;
  discState.isLoading = true;

  const grid = discoveryDom.elMaybe('discovery-grid');
  const loadingEl = discoveryDom.elMaybe('discovery-loading');
  const emptyEl = discoveryDom.elMaybe('discovery-empty');
  const paginationEl = discoveryDom.elMaybe('discovery-pagination');

  if (loadingEl) loadingEl.style.display = 'flex';
  if (emptyEl) emptyEl.style.display = 'none';
  if (grid) grid.style.display = 'none';

  try {
    const selectedCat = discState.currentCategoryId ? discState.categories.find((c) => c.categoryId === discState.currentCategoryId) : undefined;
    const titleVal = discoveryDom.elMaybe('filter-title-contains')?.value.trim();
    const descVal = discoveryDom.elMaybe('filter-desc-contains')?.value.trim();
    const authorVal = discoveryDom.elMaybe('filter-author-contains')?.value.trim();
    const uploaderVal = discoveryDom.elMaybe('filter-uploader-contains')?.value.trim();
    const vortexChecked = discoveryDom.elMaybe('filter-supports-vortex')?.checked;
    const updatedChecked = discoveryDom.elMaybe('filter-has-updated')?.checked;
    const hideAdultChecked = discoveryDom.elMaybe('filter-hide-adult')?.checked;
    const onlyAdultChecked = discoveryDom.elMaybe('filter-only-adult')?.checked;
    const hideTransChecked = discoveryDom.elMaybe('filter-hide-translations')?.checked;

    // Read selected languages
    const selectedLangs = Array.from(document.querySelectorAll<HTMLInputElement>('.filter-lang-cb:checked')).map((cb) => cb.value);

    let adultFilter = discState.currentNsfwFilter;
    if (onlyAdultChecked) {
      adultFilter = 'show';
    } else if (hideAdultChecked) {
      adultFilter = 'hide';
    }

    const res = await getDiscoveryMods({
      query: titleVal || discState.currentQuery || undefined,
      descriptionQuery: descVal || undefined,
      authorQuery: authorVal || undefined,
      uploaderQuery: uploaderVal || undefined,
      categoryId: discState.currentCategoryId || undefined,
      categoryName: selectedCat ? selectedCat.name : undefined,
      supportsVortex: vortexChecked ? true : undefined,
      hasUpdated: updatedChecked ? true : undefined,
      languageNames: selectedLangs.length > 0 ? selectedLangs : undefined,
      includeTags: discState.selectedIncludeTags.size > 0 ? Array.from(discState.selectedIncludeTags) : undefined,
      excludeTags: discState.selectedExcludeTags.size > 0 ? Array.from(discState.selectedExcludeTags) : undefined,
      hideTranslations: hideTransChecked ? true : undefined,
      sortBy: discState.currentSort,
      timeRange: discState.currentTimeRange !== 'all' ? discState.currentTimeRange : undefined,
      includeAdult: adultFilter !== 'hide',
      adultFilter: adultFilter,
      page: discState.currentPage,
      pageSize: discState.PAGE_SIZE,
    });

    discState.totalCount = res.totalCount;
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
      const totalPages = Math.max(1, Math.ceil(discState.totalCount / discState.PAGE_SIZE));
      if (discState.totalCount > discState.PAGE_SIZE) {
        paginationEl.style.display = 'flex';
        const prevBtn = discoveryDom.elMaybe('discovery-prev-page');
        const nextBtn = discoveryDom.elMaybe('discovery-next-page');
        const pageIndicator = discoveryDom.elMaybe('discovery-page-indicator');

        if (prevBtn) prevBtn.disabled = discState.currentPage <= 1;
        if (nextBtn) nextBtn.disabled = discState.currentPage >= totalPages;
        if (pageIndicator) pageIndicator.textContent = `${t('discovery.page_label', { current: discState.currentPage, total: totalPages })}`;
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
    discState.isLoading = false;
  }
}
