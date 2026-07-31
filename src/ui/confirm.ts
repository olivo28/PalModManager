import { escapeHtml } from '../utils/helpers';

export function showConfirm(message: string): Promise<boolean> {
  return new Promise((resolve) => {
    const overlay = document.createElement('div');
    overlay.className = 'confirm-overlay';
    overlay.innerHTML = `
      <div class="confirm-box">
        <p>${escapeHtml(message)}</p>
        <div class="confirm-actions">
          <button class="confirm-cancel">Cancel</button>
          <button class="confirm-danger">Confirm</button>
        </div>
      </div>
    `;
    document.body.appendChild(overlay);

    overlay.querySelector('.confirm-cancel')!.addEventListener('click', () => {
      overlay.remove();
      resolve(false);
    });
    overlay.querySelector('.confirm-danger')!.addEventListener('click', () => {
      overlay.remove();
      resolve(true);
    });
  });
}
