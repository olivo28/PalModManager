import { escapeHtml } from '../../utils/helpers';
import { discState, NEXUS_PALWORLD_TAGS } from './state';
import { discoveryDom } from '../../framework';

export function renderTagChips(type: 'include' | 'exclude'): void {
  const container = discoveryDom.elMaybe(type === 'include' ? 'tags-include-chips' : 'tags-exclude-chips');
  if (!container) return;

  const set = type === 'include' ? discState.selectedIncludeTags : discState.selectedExcludeTags;
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
        if (t === 'include') discState.selectedIncludeTags.delete(tag);
        else discState.selectedExcludeTags.delete(tag);
        renderTagChips(t);
        renderTagMenu(t);
      }
    });
  });
}

export function renderTagMenu(type: 'include' | 'exclude', filterText: string = ''): void {
  const menu = discoveryDom.elMaybe(type === 'include' ? 'tags-include-menu' : 'tags-exclude-menu');
  if (!menu) return;

  const set = type === 'include' ? discState.selectedIncludeTags : discState.selectedExcludeTags;
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
        const targetSet = t === 'include' ? discState.selectedIncludeTags : discState.selectedExcludeTags;
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

export function setupTagPickers(): void {
  const incSearch = discoveryDom.elMaybe('tags-include-search');
  const incMenu = discoveryDom.elMaybe('tags-include-menu');
  const excSearch = discoveryDom.elMaybe('tags-exclude-search');
  const excMenu = discoveryDom.elMaybe('tags-exclude-menu');

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
