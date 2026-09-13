import { fomodDom } from '../../../framework';
import { t } from '../../../utils/i18n';
import type { FomodGroup, FomodPlugin, FomodStep } from './types';
import {
  getFomodState,
  isGroupValid,
  isStepValid,
  setPluginSelection,
} from './state';

export interface ParsedStepHeader {
  sidebarLabel: string;
  heading: string;
  subtitle: string;
}

export function parseStepHeader(rawName: string, stepIndex: number): ParsedStepHeader {
  const trimmed = (rawName || '').trim();
  if (!trimmed) {
    return {
      sidebarLabel: `Step ${stepIndex + 1}`,
      heading: `Step ${stepIndex + 1}`,
      subtitle: '',
    };
  }

  // Step 1 pattern: "(If you want to use a preset, ignore these pages and go straight to the last page!)"
  if (trimmed.startsWith('(') && /preset/i.test(trimmed)) {
    return {
      sidebarLabel: 'Base Dimensions',
      heading: 'Base Dimensions & Range Settings',
      subtitle: `💡 Note: ${trimmed.slice(1, -1).trim() || trimmed}`,
    };
  }

  // Presets pattern: "Presets! (Selecting one of these will OVERRIDE any choices made on the previous screens!)"
  if (/^presets?!/i.test(trimmed)) {
    const noteMatch = trimmed.match(/\(([^)]+)\)/);
    return {
      sidebarLabel: 'Presets & Overrides',
      heading: 'Presets & Quick Configs',
      subtitle: noteMatch ? `⚠️ Note: ${noteMatch[1].trim()}` : '',
    };
  }

  // Generic pattern with parentheses: "Main Title (Additional notes)"
  const match = trimmed.match(/^([^()]+)\s*\(([^)]+)\)/);
  if (match) {
    const mainTitle = match[1].trim();
    const note = match[2].trim();
    return {
      sidebarLabel: mainTitle,
      heading: mainTitle,
      subtitle: note ? `💡 ${note}` : '',
    };
  }

  return {
    sidebarLabel: trimmed,
    heading: trimmed,
    subtitle: '',
  };
}

export interface ParsedGroupHeader {
  cleanTitle: string;
  instruction: string;
}

export function parseGroupHeader(rawName: string): ParsedGroupHeader {
  const trimmed = (rawName || '').trim();
  if (!trimmed) {
    return { cleanTitle: 'Settings', instruction: '' };
  }

  const match = trimmed.match(/^([^()]+)\s*\((.+)\)$/);
  if (match) {
    return {
      cleanTitle: match[1].trim(),
      instruction: match[2].trim(),
    };
  }

  return {
    cleanTitle: trimmed,
    instruction: '',
  };
}

export function scrollToOpenAccordion(): void {
  setTimeout(() => {
    const openEl = document.querySelector('.fomod-accordion-group.open');
    if (openEl) {
      openEl.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    }
  }, 40);
}

export function scrollToTopContent(): void {
  setTimeout(() => {
    const mainPanel = document.querySelector('.fomod-main-panel');
    if (mainPanel) {
      mainPanel.scrollTop = 0;
    }
  }, 40);
}

export function renderSidebarSteps(
  steps: FomodStep[],
  currentIndex: number,
  onStepSelected: (idx: number) => void
): void {
  const navEl = fomodDom.el('fomod-steps-nav');
  navEl.innerHTML = '';

  steps.forEach((s, idx) => {
    const btn = document.createElement('button');
    btn.type = 'button';
    const isActive = idx === currentIndex;
    const isValid = isStepValid(s);

    btn.className = `fomod-step-nav-btn ${isActive ? 'active' : ''} ${isValid ? 'valid' : ''}`;

    const left = document.createElement('div');
    left.className = 'step-nav-left';

    const badge = document.createElement('span');
    badge.className = 'step-num-badge';
    badge.textContent = String(idx + 1);

    const label = document.createElement('span');
    label.className = 'step-nav-label';
    const parsed = parseStepHeader(s.name, idx);
    label.textContent = parsed.sidebarLabel;
    label.title = s.name;

    left.appendChild(badge);
    left.appendChild(label);
    btn.appendChild(left);

    if (isValid) {
      const check = document.createElement('span');
      check.className = 'step-status-icon';
      check.textContent = '✓';
      btn.appendChild(check);
    }

    btn.addEventListener('click', () => {
      onStepSelected(idx);
    });

    navEl.appendChild(btn);
  });
}

export function renderMainHeader(step: FomodStep): void {
  const state = getFomodState();
  const headingEl = fomodDom.el('fomod-step-heading');
  const subtitleEl = fomodDom.el('fomod-step-subtitle');
  const modDescEl = fomodDom.el('fomod-mod-desc');

  const parsed = parseStepHeader(step.name, state.currentStepIndex);
  headingEl.textContent = parsed.heading;

  // Mod description from info.xml — sticky across all steps
  const modDesc = state.config?.info?.description?.trim();
  if (modDesc) {
    modDescEl.textContent = modDesc;
    modDescEl.style.display = 'block';
  } else {
    modDescEl.style.display = 'none';
  }

  // Subtitle / Note
  if (parsed.subtitle) {
    subtitleEl.textContent = parsed.subtitle;
    subtitleEl.style.display = 'block';
  } else {
    subtitleEl.style.display = 'none';
  }
}

export function renderGroupsFlow(step: FomodStep, onStateChange: () => void): void {
  const state = getFomodState();
  const flowContainer = fomodDom.el('fomod-step-groups');
  flowContainer.innerHTML = '';

  const groups = step.groups || [];
  if (groups.length === 0) return;

  // Single Group: render directly without accordion wrap
  if (groups.length === 1) {
    const groupEl = renderSingleGroupView(groups[0], onStateChange);
    flowContainer.appendChild(groupEl);
    return;
  }

  // Multi-Group: render as Accordion
  if (!state.openAccordionGroup || !groups.some((g) => g.name === state.openAccordionGroup)) {
    const firstInvalid = groups.find((g) => !isGroupValid(g));
    state.openAccordionGroup = firstInvalid ? firstInvalid.name : groups[0].name;
  }

  groups.forEach((group) => {
    const accordionEl = renderAccordionGroup(
      group,
      group.name === state.openAccordionGroup,
      onStateChange
    );
    flowContainer.appendChild(accordionEl);
  });
}

export function renderSingleGroupView(group: FomodGroup, onStateChange: () => void): HTMLElement {
  const card = document.createElement('div');
  card.className = 'fomod-single-group-card';

  const { instruction } = parseGroupHeader(group.name);
  if (instruction) {
    const instrEl = document.createElement('div');
    instrEl.className = 'fomod-group-instruction';
    instrEl.textContent = `💡 Setting Note: ${instruction}`;
    card.appendChild(instrEl);
  }

  const list = document.createElement('div');
  list.className = 'fomod-options-list';
  list.style.display = 'flex';
  list.style.flexDirection = 'column';
  list.style.gap = '6px';

  const isRadio = group.groupType === 'SelectExactlyOne' || group.groupType === 'SelectAtMostOne';
  const state = getFomodState();
  const selectedSet = state.selections.get(group.name) || new Set<string>();

  for (const plugin of group.plugins) {
    const isSelected = selectedSet.has(plugin.name);
    const item = renderPluginOption(group, plugin, isRadio, isSelected, onStateChange);
    list.appendChild(item);
  }

  card.appendChild(list);
  return card;
}

export function renderAccordionGroup(
  group: FomodGroup,
  isOpen: boolean,
  onStateChange: () => void
): HTMLElement {
  const state = getFomodState();
  const selectedSet = state.selections.get(group.name) || new Set<string>();
  const isValid = isGroupValid(group);
  const { cleanTitle, instruction } = parseGroupHeader(group.name);

  const card = document.createElement('div');
  card.className = `fomod-accordion-group ${isOpen ? 'open' : ''} ${isValid ? '' : 'invalid'}`;

  // Header
  const header = document.createElement('div');
  header.className = 'fomod-accordion-header';

  const left = document.createElement('div');
  left.className = 'accordion-header-left';

  const chevron = document.createElement('span');
  chevron.className = 'accordion-chevron';
  chevron.textContent = '▶';

  const title = document.createElement('span');
  title.className = 'accordion-title';
  title.textContent = cleanTitle;
  title.title = group.name;

  const badge = document.createElement('span');
  badge.className = `accordion-badge ${getGroupBadgeClass(group.groupType)}`;
  badge.textContent = getGroupTypeBadgeLabel(group.groupType);

  left.appendChild(chevron);
  left.appendChild(title);
  left.appendChild(badge);

  const right = document.createElement('div');
  right.className = 'accordion-header-right';

  // Selection summary text
  const summary = document.createElement('span');
  summary.className = 'accordion-summary';
  summary.textContent = getGroupSelectionSummary(group, selectedSet);
  right.appendChild(summary);

  if (isValid) {
    const check = document.createElement('span');
    check.className = 'accordion-check';
    check.textContent = '✓';
    right.appendChild(check);
  } else {
    const warn = document.createElement('span');
    warn.className = 'accordion-warn';
    warn.textContent = '⚠️';
    right.appendChild(warn);
  }

  header.appendChild(left);
  header.appendChild(right);

  header.addEventListener('click', () => {
    state.openAccordionGroup = isOpen ? null : group.name;
    onStateChange();
    if (!isOpen) {
      scrollToOpenAccordion();
    }
  });

  card.appendChild(header);

  // Body
  const body = document.createElement('div');
  body.className = 'fomod-accordion-body';

  if (instruction) {
    const instrEl = document.createElement('div');
    instrEl.className = 'fomod-group-instruction';
    instrEl.textContent = `💡 Setting Note: ${instruction}`;
    body.appendChild(instrEl);
  }

  const isRadio = group.groupType === 'SelectExactlyOne' || group.groupType === 'SelectAtMostOne';

  for (const plugin of group.plugins) {
    const isSelected = selectedSet.has(plugin.name);
    const item = renderPluginOption(group, plugin, isRadio, isSelected, onStateChange);
    body.appendChild(item);
  }

  card.appendChild(body);
  return card;
}

export function renderPluginOption(
  group: FomodGroup,
  plugin: FomodPlugin,
  isRadio: boolean,
  isSelected: boolean,
  onStateChange: () => void
): HTMLElement {
  const isNotUsable = plugin.typeDescriptor === 'NotUsable';
  const state = getFomodState();
  const isRemembered = Boolean(state.rememberedSelections?.has(`${group.name}::${plugin.name}`));

  const row = document.createElement('label');
  row.className = `fomod-option-item ${isSelected ? 'selected' : ''} ${isRemembered ? 'remembered' : ''} ${isNotUsable ? 'disabled' : ''}`;

  const leftBox = document.createElement('div');
  leftBox.className = 'fomod-option-left';

  // Input
  const input = document.createElement('input');
  input.type = isRadio ? 'radio' : 'checkbox';
  input.name = `fomod-grp-${group.name}`;
  input.className = 'fomod-option-input';
  input.checked = isSelected;
  input.disabled = isNotUsable;

  input.addEventListener('change', () => {
    setPluginSelection(group, plugin.name, input.checked);
    onStateChange();
  });

  leftBox.appendChild(input);

  // Content Column
  const contentCol = document.createElement('div');
  contentCol.className = 'fomod-option-content';

  const labelRow = document.createElement('div');
  labelRow.className = 'fomod-option-label-row';

  // Full name of option
  const nameSpan = document.createElement('span');
  nameSpan.className = 'fomod-option-name';
  nameSpan.textContent = plugin.name;
  labelRow.appendChild(nameSpan);

  // Badges
  if (isRemembered) {
    const remTag = document.createElement('span');
    remTag.className = 'fomod-tag-remembered';
    remTag.textContent = `🟢 ${t('fomod.remembered_tag')}`;
    labelRow.appendChild(remTag);
  }

  const lowerName = plugin.name.toLowerCase();
  const isRec =
    plugin.typeDescriptor === 'Recommended' || lowerName.includes('(recommended)');
  const isVanilla = lowerName.includes('(vanilla');

  if (isRec) {
    const recTag = document.createElement('span');
    recTag.className = 'fomod-tag-rec';
    recTag.textContent = t('fomod.recommended_tag');
    labelRow.appendChild(recTag);
  }

  if (isVanilla) {
    const vanTag = document.createElement('span');
    vanTag.className = 'fomod-tag-vanilla';
    vanTag.textContent = t('fomod.vanilla_tag');
    labelRow.appendChild(vanTag);
  }

  if (plugin.typeDescriptor === 'Required') {
    const reqTag = document.createElement('span');
    reqTag.className = 'fomod-tag-rec';
    reqTag.style.background = 'rgba(239, 68, 68, 0.15)';
    reqTag.style.borderColor = 'rgba(239, 68, 68, 0.35)';
    reqTag.style.color = '#f87171';
    reqTag.textContent = 'REQUIRED';
    labelRow.appendChild(reqTag);
  }

  contentCol.appendChild(labelRow);

  // Always render description if present
  if (plugin.description && plugin.description.trim()) {
    const desc = document.createElement('p');
    desc.className = 'fomod-option-desc';
    desc.textContent = plugin.description.trim();
    contentCol.appendChild(desc);
  }

  leftBox.appendChild(contentCol);
  row.appendChild(leftBox);

  // Optional Image Thumbnail
  if (plugin.imageBase64) {
    const thumb = document.createElement('img');
    thumb.className = 'fomod-option-thumb';
    thumb.src = plugin.imageBase64;
    thumb.alt = plugin.name;
    row.appendChild(thumb);
  }

  return row;
}

export function getGroupSelectionSummary(group: FomodGroup, selectedSet: Set<string>): string {
  if (selectedSet.size === 0) return 'None';
  if (group.groupType === 'SelectExactlyOne' || group.groupType === 'SelectAtMostOne') {
    const name = Array.from(selectedSet)[0];
    return name ? cleanFomodName(name) : 'None';
  }
  return `${selectedSet.size} selected`;
}

export function getGroupBadgeClass(groupType: string): string {
  if (groupType === 'SelectExactlyOne' || groupType === 'SelectAtLeastOne') return 'single';
  if (groupType === 'SelectAll') return 'multiple';
  return 'optional';
}

export function getGroupTypeBadgeLabel(groupType: string): string {
  switch (groupType) {
    case 'SelectExactlyOne':
    case 'SelectAtLeastOne':
      return t('fomod.badge_single_choice');
    case 'SelectAtMostOne':
      return t('fomod.badge_at_most_one');
    case 'SelectAll':
      return t('fomod.badge_all_required');
    case 'SelectAny':
    default:
      return t('fomod.badge_optional');
  }
}

export function cleanFomodName(raw: string): string {
  if (!raw) return '';
  let text = raw.trim();
  text = text.replace(/^PalSchema\s*[-_:]?\s*/i, '');
  text = text.replace(/^BetterBaseBuilding\s*[-_:]?\s*/i, '');
  text = text.replace(/^BBB\s*[-_:]?\s*/i, '');
  return text.trim() || raw;
}
