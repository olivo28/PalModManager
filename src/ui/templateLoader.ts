// Template Loader: Assembles granular HTML fragments into the DOM
import modsViewHtml from '../templates/views/modsView.html?raw';
import loadViewHtml from '../templates/views/loadView.html?raw';
import libraryViewHtml from '../templates/views/libraryView.html?raw';
import editorViewHtml from '../templates/views/editorView.html?raw';
import packerViewHtml from '../templates/views/packerView.html?raw';
import scannerViewHtml from '../templates/views/scannerView.html?raw';
import dbViewHtml from '../templates/views/dbView.html?raw';
import discoveryViewHtml from '../templates/views/discoveryView.html?raw';

import settingsModalHtml from '../templates/modals/settingsModal.html?raw';
import installModalHtml from '../templates/modals/installModal.html?raw';
import detailPanelHtml from '../templates/modals/detailPanel.html?raw';
import profileModalHtml from '../templates/modals/profileModal.html?raw';
import workshopModalHtml from '../templates/modals/workshopModal.html?raw';
import aboutModalHtml from '../templates/modals/aboutModal.html?raw';
import nexusProfileModalHtml from '../templates/modals/nexusProfileModal.html?raw';
import downloadQueuePanelHtml from '../templates/modals/downloadQueuePanel.html?raw';
import discoveryModModalHtml from '../templates/modals/discoveryModModal.html?raw';
import { mainDom } from '../framework';

export function loadAppTemplates(): void {
  const mainContent = mainDom.elMaybe('main-content');
  const modalsRoot = mainDom.elMaybe('modals-root');

  if (mainContent) {
    mainContent.innerHTML = [
      discoveryViewHtml,
      modsViewHtml,
      loadViewHtml,
      libraryViewHtml,
      editorViewHtml,
      packerViewHtml,
      scannerViewHtml,
      dbViewHtml,
    ].join('\n');
  }

  if (modalsRoot) {
    modalsRoot.innerHTML = [
      detailPanelHtml,
      installModalHtml,
      settingsModalHtml,
      profileModalHtml,
      workshopModalHtml,
      aboutModalHtml,
      nexusProfileModalHtml,
      downloadQueuePanelHtml,
      discoveryModModalHtml,
    ].join('\n');
  }
}
