import { fomodDom } from '../../../framework';
import { closeFomodModal, ensureFomodModalInDom, handleNextClick, handlePrevClick } from './wizard';

let isInitialized = false;

export function initFomodListeners(): void {
  if (isInitialized) return;

  ensureFomodModalInDom();

  const modal = fomodDom.elMaybe('fomod-modal');
  if (!modal) return;

  const closeX = fomodDom.elMaybe('fomod-modal-close-x');
  if (closeX) {
    closeX.addEventListener('click', closeFomodModal);
  }

  const cancelBtn = fomodDom.elMaybe('fomod-btn-cancel');
  if (cancelBtn) {
    cancelBtn.addEventListener('click', closeFomodModal);
  }

  const prevBtn = fomodDom.elMaybe('fomod-btn-prev');
  if (prevBtn) {
    prevBtn.addEventListener('click', handlePrevClick);
  }

  const nextBtn = fomodDom.elMaybe('fomod-btn-next');
  if (nextBtn) {
    nextBtn.addEventListener('click', () => {
      void handleNextClick();
    });
  }

  modal.addEventListener('click', (e) => {
    if (e.target === modal) {
      closeFomodModal();
    }
  });

  isInitialized = true;
}
