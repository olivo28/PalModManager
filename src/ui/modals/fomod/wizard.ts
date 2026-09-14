import { fomodDom } from '../../../framework';
import { bus } from '../../../framework';
import { t } from '../../../utils/i18n';
import { buildFomodManifest, installModWithManifest } from '../../../api/mods';
import { parseModFilename } from '../../mods/library/helpers';
import { resolveFomodVersionConsensus } from './version';
import {
  collectAllSelectedFiles,
  getCurrentVisibleStep,
  getFomodState,
  getVisibleSteps,
  isCurrentStepValid,
  isGroupValid,
} from './state';
import {
  renderGroupsFlow,
  renderMainHeader,
  renderSidebarSteps,
  scrollToOpenAccordion,
  scrollToTopContent,
} from './render';

import fomodModalHtml from '../../../templates/modals/fomodModal.html?raw';

export function ensureFomodModalInDom(): void {
  if (!document.getElementById('fomod-modal')) {
    const root = document.getElementById('modals-root') || document.body;
    root.insertAdjacentHTML('beforeend', fomodModalHtml);
  }
}

export function openFomodModal(): void {
  ensureFomodModalInDom();
  const modal = fomodDom.el('fomod-modal');
  if (modal) {
    modal.classList.add('visible', 'active');
  }
  renderCurrentStep();
}

export function closeFomodModal(): void {
  const modal = fomodDom.el('fomod-modal');
  modal.classList.remove('visible', 'active');
}

export function renderCurrentStep(): void {
  const state = getFomodState();
  const visibleSteps = getVisibleSteps();
  const step = getCurrentVisibleStep();

  if (!state.config || !step) {
    return;
  }

  // Progress Fill
  const progressFill = fomodDom.el('fomod-progress-fill');
  const pct = Math.round(((state.currentStepIndex + 1) / visibleSteps.length) * 100);
  progressFill.style.width = `${pct}%`;

  // Header Title & Version
  const titleEl = fomodDom.el('fomod-mod-title');
  const verEl = fomodDom.el('fomod-mod-version');
  const authorEl = fomodDom.el('fomod-mod-author');

  titleEl.textContent = state.customName || state.config.info?.name || state.config.moduleName || 'Mod';
  const displayVer = state.version || state.config.info?.version || '1.0.0';
  verEl.textContent = displayVer.startsWith('v') || displayVer.startsWith('V') ? displayVer : `v${displayVer}`;

  if (state.config.info?.author) {
    authorEl.textContent = `${t('common.author')}: ${state.config.info.author}`;
    authorEl.style.display = 'inline-block';
  } else {
    authorEl.style.display = 'none';
  }

  // Step Badge
  const stepBadge = fomodDom.el('fomod-step-badge');
  stepBadge.textContent = t('fomod.step_counter', {
    current: String(state.currentStepIndex + 1),
    total: String(visibleSteps.length),
  });

  // Banner
  const bannerWrap = fomodDom.el('fomod-banner-container');
  const bannerImg = fomodDom.el('fomod-banner-img');
  if (state.config.bannerBase64) {
    bannerImg.src = state.config.bannerBase64;
    bannerWrap.style.display = 'block';
  } else {
    bannerWrap.style.display = 'none';
  }

  // Render Sidebar Navigation
  renderSidebarSteps(visibleSteps, state.currentStepIndex, (idx) => {
    if (idx !== state.currentStepIndex) {
      state.currentStepIndex = idx;
      state.openAccordionGroup = null;
      renderCurrentStep();
    }
  });

  // Render Main Panel Header
  renderMainHeader(step);

  // Render Groups Flow (Accordion or Direct List)
  renderGroupsFlow(step, () => {
    renderCurrentStep();
  });

  // Navigation Buttons
  updateNavigationButtons(visibleSteps.length);
}

export function updateNavigationButtons(totalSteps: number): void {
  const state = getFomodState();
  const step = getCurrentVisibleStep();
  const prevBtn = fomodDom.el('fomod-btn-prev');
  const nextBtn = fomodDom.el('fomod-btn-next');
  const nextLabel = fomodDom.el('fomod-next-label');

  const groups = step?.groups || [];
  const openIdx = groups.findIndex((g) => g.name === state.openAccordionGroup);
  const currentGroup = openIdx >= 0 ? groups[openIdx] : groups[0];

  const isMultiGroup = groups.length > 1;
  const isLastGroupInStep = !isMultiGroup || openIdx >= groups.length - 1;
  const isFirstGroupInStep = !isMultiGroup || openIdx <= 0;
  const isLastStep = state.currentStepIndex >= totalSteps - 1;

  // Prev button visible if not on first group of first step
  if (state.currentStepIndex > 0 || !isFirstGroupInStep) {
    prevBtn.style.display = 'block';
  } else {
    prevBtn.style.display = 'none';
  }

  // Next / Install button
  if (isLastStep && isLastGroupInStep) {
    nextLabel.textContent = t('fomod.btn_install');
    nextBtn.style.background = 'linear-gradient(135deg, #10b981 0%, #059669 100%)';
    nextBtn.style.borderColor = '#10b981';
  } else {
    nextLabel.textContent = t('fomod.btn_next');
    nextBtn.style.background = '';
    nextBtn.style.borderColor = '';
  }

  const isValid = currentGroup ? isGroupValid(currentGroup) : isCurrentStepValid();
  nextBtn.disabled = !isValid;
  nextBtn.style.opacity = isValid ? '1' : '0.5';
  nextBtn.style.cursor = isValid ? 'pointer' : 'not-allowed';
}

export async function handleNextClick(): Promise<void> {
  const state = getFomodState();
  const visibleSteps = getVisibleSteps();
  const step = getCurrentVisibleStep();
  if (!step) return;

  const groups = step.groups || [];
  const openIdx = groups.findIndex((g) => g.name === state.openAccordionGroup);
  const currentGroup = openIdx >= 0 ? groups[openIdx] : groups[0];

  // If current group is not valid, do not advance
  if (currentGroup && !isGroupValid(currentGroup)) {
    return;
  }

  // If there are more groups within this step, advance to next group!
  if (groups.length > 1 && openIdx < groups.length - 1) {
    state.openAccordionGroup = groups[openIdx + 1].name;
    renderCurrentStep();
    scrollToOpenAccordion();
    return;
  }

  // End of groups in current step: check that the whole step is valid
  if (!isCurrentStepValid()) return;

  // If more steps remain, advance to next step
  if (state.currentStepIndex < visibleSteps.length - 1) {
    state.currentStepIndex++;
    state.openAccordionGroup = null;
    renderCurrentStep();
    scrollToTopContent();
    return;
  }

  // Last Step & Last Group: Install!
  await performFomodInstall();
}

export function handlePrevClick(): void {
  const state = getFomodState();
  const visibleSteps = getVisibleSteps();
  const step = getCurrentVisibleStep();
  if (!step) return;

  const groups = step.groups || [];
  const openIdx = groups.findIndex((g) => g.name === state.openAccordionGroup);

  // If there are previous groups in this step, go to previous group!
  if (groups.length > 1 && openIdx > 0) {
    state.openAccordionGroup = groups[openIdx - 1].name;
    renderCurrentStep();
    scrollToOpenAccordion();
    return;
  }

  // Otherwise go to previous step
  if (state.currentStepIndex > 0) {
    state.currentStepIndex--;
    const prevStep = visibleSteps[state.currentStepIndex];
    if (prevStep?.groups && prevStep.groups.length > 1) {
      // Open the last group of that previous step
      state.openAccordionGroup = prevStep.groups[prevStep.groups.length - 1].name;
    } else {
      state.openAccordionGroup = null;
    }
    renderCurrentStep();
    scrollToOpenAccordion();
  }
}

async function performFomodInstall(): Promise<void> {
  const state = getFomodState();
  if (!state.config || !state.zipPath) return;

  const nextBtn = fomodDom.el('fomod-btn-next');
  const nextLabel = fomodDom.el('fomod-next-label');

  nextBtn.disabled = true;
  nextLabel.textContent = t('fomod.installing');

  try {
    const selectedFiles = collectAllSelectedFiles();
    console.log('[FOMOD] Compiling manifest with files count:', selectedFiles.length);

    const modName = state.customName || state.config.info?.name || state.config.moduleName;
    const folderName = state.config.moduleName;
    const author = state.config.info?.author;
    const xmlVer = state.config.info?.version;
    const zipVer = parseModFilename(state.zipPath).version || (state.version !== state.existingMod?.version ? state.version : null);
    const apiVer = state.version !== state.existingMod?.version ? state.version : null;
    const version = resolveFomodVersionConsensus(xmlVer, zipVer, apiVer, state.config.info?.version || state.version || '1.0.0');
    const summary = state.config.info?.description;

    const fomodChoices: Record<string, string[]> = {};
    for (const [grpName, pluginSet] of state.selections.entries()) {
      if (pluginSet && pluginSet.size > 0) {
        fomodChoices[grpName] = Array.from(pluginSet);
      }
    }

    const manifest = await buildFomodManifest({
      zipPath: state.zipPath,
      selectedFiles,
      customName: state.customName,
      modName,
      customFolder: folderName,
      author,
      version,
      summary,
      fomodChoices,
    });

    console.log('[FOMOD] Manifest compiled:', manifest);
    const installedMod = await installModWithManifest(manifest, state.zipPath);
    console.log('[FOMOD] Installed successfully:', installedMod.name);

    closeFomodModal();

    bus.emit('toast:show', {
      message: t('fomod.install_success', { name: installedMod.name }),
      type: 'success',
    });

    bus.emit('mods:refresh', undefined);

    // If config diffs were detected from a previous installation of the exact same file, show config recovery modal
    const diffs = (installedMod as any).configDiffs;
    if (diffs && Array.isArray(diffs) && diffs.length > 0) {
      try {
        const { showConfigDiffModal } = await import('../installer/diffModal');
        showConfigDiffModal(diffs, installedMod.id);
      } catch (diffErr) {
        console.warn('[FOMOD] Could not open config diff modal:', diffErr);
      }
    }
  } catch (err: any) {
    console.error('[FOMOD] Installation failed:', err);
    bus.emit('toast:show', {
      message: t('fomod.install_error', { error: String(err) }),
      type: 'error',
    });
    nextBtn.disabled = false;
    nextLabel.textContent = t('fomod.btn_install');
  }
}

export function setupFomodListeners(): void {
  ensureFomodModalInDom();

  const closeX = fomodDom.el('fomod-modal-close-x');
  const cancelBtn = fomodDom.el('fomod-btn-cancel');
  const prevBtn = fomodDom.el('fomod-btn-prev');
  const nextBtn = fomodDom.el('fomod-btn-next');

  closeX.addEventListener('click', closeFomodModal);
  cancelBtn.addEventListener('click', closeFomodModal);
  prevBtn.addEventListener('click', handlePrevClick);
  nextBtn.addEventListener('click', () => {
    handleNextClick().catch((err) => console.error('[FOMOD] handleNextClick error:', err));
  });
}
