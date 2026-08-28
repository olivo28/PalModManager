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
  const catSelect = document.getElementById('discovery-category-select') as HTMLSelectElement | null;
  if (catSelect) {
    catSelect.addEventListener('change', () => {
      const val = catSelect.value;
      discState.currentCategoryId = val ? parseInt(val, 10) : null;
      discState.currentPage = 1;
      loadMods();
    });
  }

  // Time Range select
  const timeSelect = document.getElementById('discovery-time-select') as HTMLSelectElement | null;
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
  const sortSelect = document.getElementById('discovery-sort-select') as HTMLSelectElement | null;
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
  const nsfwSelect = document.getElementById('discovery-nsfw-select') as HTMLSelectElement | null;
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
  document.getElementById('discovery-refresh-btn')?.addEventListener('click', () => {
    loadMods();
  });

  // Pagination buttons
  document.getElementById('discovery-prev-page')?.addEventListener('click', () => {
    if (discState.currentPage > 1) {
      discState.currentPage--;
      loadMods();
      scrollToTop();
    }
  });

  document.getElementById('discovery-next-page')?.addEventListener('click', () => {
    if (discState.currentPage * discState.PAGE_SIZE < discState.totalCount) {
      discState.currentPage++;
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
    updateLightboxTransform(discState.lightboxZoom + 0.3, discState.lightboxPanX, discState.lightboxPanY);
  });
  document.getElementById('discovery-lightbox-zoom-out')?.addEventListener('click', () => {
    updateLightboxTransform(discState.lightboxZoom - 0.3, discState.lightboxPanX, discState.lightboxPanY);
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

      discState.selectedIncludeTags.clear();
      discState.selectedExcludeTags.clear();
      renderTagChips('include');
      renderTagChips('exclude');

      document.querySelectorAll<HTMLInputElement>('.filter-lang-cb').forEach((cb) => (cb.checked = false));

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

export async function loadCategories(): Promise<void> {
  try {
    discState.categories = await getDiscoveryCategories();
    const select = document.getElementById('discovery-category-select') as HTMLSelectElement | null;
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

  const grid = document.getElementById('discovery-grid');
  const loadingEl = document.getElementById('discovery-loading');
  const emptyEl = document.getElementById('discovery-empty');
  const paginationEl = document.getElementById('discovery-pagination');

  if (loadingEl) loadingEl.style.display = 'flex';
  if (emptyEl) emptyEl.style.display = 'none';
  if (grid) grid.style.display = 'none';

  try {
    const selectedCat = discState.currentCategoryId ? discState.categories.find((c) => c.categoryId === discState.currentCategoryId) : undefined;
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
        const prevBtn = document.getElementById('discovery-prev-page') as HTMLButtonElement | null;
        const nextBtn = document.getElementById('discovery-next-page') as HTMLButtonElement | null;
        const pageIndicator = document.getElementById('discovery-page-indicator');

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
