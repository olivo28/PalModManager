import { closeInstallModal, closeSettingsModal, closeAboutModal } from '../ui/modal';
import { closeDetailPanel } from '../ui/detailPanel';
import { mainDom, detailDom, discoveryDom, settingsDom, scannerDom, installerDom } from '../framework';

export function setupGlobalShortcuts(): void {
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      // 1. Confirm / Prompt custom overlays (highest z-index, handled by their own listeners)
      const confirmOverlay = document.querySelector('.confirm-overlay');
      if (confirmOverlay) {
        return;
      }

      // 2. Unreal Engine Asset Inspector modal (z-index: 10000)
      const uassetModal = mainDom.elMaybe('uasset-inspector-modal');
      if (uassetModal) {
        uassetModal.remove();
        return;
      }

      // 3. Save Health Doctor Comparison modal (z-index: 9999)
      const saveCompareModal = scannerDom.elMaybe('save-compare-modal');
      if (saveCompareModal) {
        saveCompareModal.remove();
        return;
      }

      // 4. Discovery Lightbox Image modal
      const discoveryImageModal = discoveryDom.elMaybe('discovery-image-modal');
      if (discoveryImageModal && (discoveryImageModal.classList.contains('visible') || (discoveryImageModal.style.display && discoveryImageModal.style.display !== 'none'))) {
        discoveryImageModal.classList.remove('visible');
        discoveryImageModal.style.display = 'none';
        return;
      }

      // 5. File Tree / Show Files Modal from installer or library (z-index: 4500)
      const fileTreeModal = installerDom.elMaybe('file-tree-modal');
      if (fileTreeModal) {
        fileTreeModal.remove();
        return;
      }

      // 6. Full files modal overlay / Archive view modals
      const fullFilesOverlay = installerDom.elMaybe('full-files-modal-overlay');
      if (fullFilesOverlay) {
        fullFilesOverlay.remove();
        return;
      }
      const archiveViewModal = installerDom.elMaybe('archive-view-modal');
      if (archiveViewModal) {
        archiveViewModal.remove();
        return;
      }
      const archiveStructureModal = installerDom.elMaybe('archive-structure-modal');
      if (archiveStructureModal) {
        archiveStructureModal.remove();
        return;
      }

      // 7. Patch Builder & Existing Patches modals
      const existingPatchesModal = scannerDom.elMaybe('pmm-existing-patches-modal');
      if (existingPatchesModal) {
        existingPatchesModal.remove();
        return;
      }
      const patchBuilderModal = scannerDom.elMaybe('pmm-patch-builder-modal-overlay');
      if (patchBuilderModal) {
        patchBuilderModal.remove();
        return;
      }

      // 8. Config Diff modal
      const configDiffOverlay = installerDom.elMaybe('config-diff-modal');
      if (configDiffOverlay) {
        configDiffOverlay.remove();
        return;
      }

      // 8. Install Modal (when open above Discovery or Mods View)
      const installModal = installerDom.elMaybe('install-modal');
      if (installModal?.classList.contains('visible')) {
        closeInstallModal();
        return;
      }

      // 9. Discovery Mod Details Modal
      const discoveryModModal = discoveryDom.elMaybe('discovery-mod-modal');
      if (discoveryModModal && (discoveryModModal.classList.contains('visible') || (discoveryModModal.style.display && discoveryModModal.style.display !== 'none'))) {
        import('../ui/discoveryView').then(({ closeDiscoveryModal }) => closeDiscoveryModal()).catch(() => {
          discoveryModModal.classList.remove('visible');
          discoveryModModal.style.display = 'none';
        });
        return;
      }

      // 10. Nexus Profile Modal
      const nexusProfileModal = settingsDom.elMaybe('nexus-profile-modal');
      if (nexusProfileModal?.classList.contains('visible')) {
        nexusProfileModal.classList.remove('visible');
        return;
      }

      // 11. Workshop Modal
      const workshopModal = mainDom.elMaybe('workshop-modal');
      if (workshopModal?.classList.contains('visible')) {
        workshopModal.classList.remove('visible');
        return;
      }

      // 12. Console Modal
      const consoleModal = mainDom.elMaybe('console-modal');
      if (consoleModal?.classList.contains('visible')) {
        consoleModal.classList.remove('visible');
        return;
      }

      // 13. Settings Modal
      const settingsModal = settingsDom.elMaybe('settings-modal');
      if (settingsModal?.classList.contains('visible')) {
        closeSettingsModal();
        return;
      }

      // 14. Profile Modal
      const profileModal = mainDom.elMaybe('profile-modal');
      if (profileModal?.classList.contains('visible')) {
        profileModal.classList.remove('visible');
        return;
      }

      // 15. About Modal
      const aboutModal = mainDom.elMaybe('about-modal');
      if (aboutModal?.classList.contains('visible')) {
        closeAboutModal();
        return;
      }

      // 16. Detail Overlay Panel
      const detailOverlay = detailDom.elMaybe('detail-overlay');
      if (detailOverlay?.classList.contains('visible')) {
        closeDetailPanel();
        return;
      }

      // 17. If no modals are open, clear mod selection on ESC
      import('../features/selection').then(({ clearSelection }) => clearSelection());
    }
  });
}
