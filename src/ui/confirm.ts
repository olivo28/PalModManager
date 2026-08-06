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

export function showPrompt(message: string, defaultValue = ''): Promise<string | null> {
  return new Promise((resolve) => {
    const overlay = document.createElement('div');
    overlay.className = 'confirm-overlay';
    overlay.innerHTML = `
      <div class="confirm-box">
        <p>${escapeHtml(message)}</p>
        <input type="text" class="confirm-input" value="${escapeHtml(defaultValue)}" style="width: 100%; padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius); color: var(--text-primary); font-size: 12px; margin-bottom: 12px; outline: none;" />
        <div class="confirm-actions">
          <button class="confirm-cancel">Cancel</button>
          <button class="confirm-danger" style="background:var(--accent);">OK</button>
        </div>
      </div>
    `;
    document.body.appendChild(overlay);

    const input = overlay.querySelector('.confirm-input') as HTMLInputElement;
    input.focus();
    input.select();

    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        overlay.remove();
        resolve(input.value);
      }
    });

    overlay.querySelector('.confirm-cancel')!.addEventListener('click', () => {
      overlay.remove();
      resolve(null);
    });
    overlay.querySelector('.confirm-danger')!.addEventListener('click', () => {
      overlay.remove();
      resolve(input.value);
    });
  });
}

