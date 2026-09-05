import { openUrl } from '../../api';
import { mainDom } from '../../framework';

export function openAboutModal(): void {
  const modal = mainDom.elMaybe('about-modal');
  if (!modal) return;
  modal.classList.add('visible');

  // Reset to first tab
  document.querySelectorAll('.about-tab-btn').forEach(b => b.classList.remove('active'));
  document.querySelectorAll('.about-tab-pane').forEach(p => (p as HTMLElement).style.display = 'none');
  
  const firstBtn = document.querySelector('.about-tab-btn[data-about-tab="overview"]');
  const firstPane = mainDom.elMaybe('about-pane-overview');
  if (firstBtn) firstBtn.classList.add('active');
  if (firstPane) firstPane.style.display = 'block';

  // Scroll to top
  const body = mainDom.elMaybe('about-modal-body');
  if (body) body.scrollTop = 0;
}

export function closeAboutModal(): void {
  const modal = mainDom.elMaybe('about-modal');
  if (modal) modal.classList.remove('visible');
}

export function setupAboutModal(): void {
  // Logo trigger
  const logoBtn = mainDom.elMaybe('sidebar-logo-btn');
  if (logoBtn) {
    logoBtn.addEventListener('click', () => {
      openAboutModal();
    });
  }

  // Version label in footer trigger as well
  const versionBtn = mainDom.elMaybe('sidebar-version');
  if (versionBtn) {
    versionBtn.style.cursor = 'pointer';
    versionBtn.title = 'About PalModManager';
    versionBtn.addEventListener('click', () => {
      openAboutModal();
    });
  }

  // Close buttons
  const closeX = mainDom.elMaybe('about-modal-close-x');
  const closeBtn = mainDom.elMaybe('about-modal-close');
  if (closeX) closeX.addEventListener('click', closeAboutModal);
  if (closeBtn) closeBtn.addEventListener('click', closeAboutModal);

  // Tabs navigation
  document.querySelectorAll('.about-tab-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const tabName = (btn as HTMLElement).dataset.aboutTab;
      if (!tabName) return;

      document.querySelectorAll('.about-tab-btn').forEach(b => b.classList.remove('active'));
      document.querySelectorAll('.about-tab-pane').forEach(p => (p as HTMLElement).style.display = 'none');

      btn.classList.add('active');
      const targetPane = document.getElementById(`about-pane-${tabName}`);
      if (targetPane) targetPane.style.display = 'block';
    });
  });

  // External links
  const githubBtn = mainDom.elMaybe('about-open-github');
  if (githubBtn) {
    githubBtn.addEventListener('click', () => {
      openUrl('https://github.com/olivo28/PalModManager').catch(() => {});
    });
  }

  const nexusBtn = mainDom.elMaybe('about-open-nexus');
  if (nexusBtn) {
    nexusBtn.addEventListener('click', () => {
      openUrl('https://www.nexusmods.com/palworld/mods/4549').catch(() => {});
    });
  }
}
