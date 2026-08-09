import { escapeHtml } from '../utils/helpers';

export function showConfirm(
  titleOrMessage: string,
  message?: string,
  confirmText = 'Confirm',
  cancelText = 'Cancel'
): Promise<boolean> {
  return new Promise((resolve) => {
    const hasTitle = !!message;
    const displayTitle = hasTitle ? titleOrMessage : 'Confirm';
    const displayBody = hasTitle ? message : titleOrMessage;

    const overlay = document.createElement('div');
    overlay.className = 'confirm-overlay';
    overlay.innerHTML = `
      <div class="confirm-box" style="min-width:320px; max-width:440px; background:var(--bg-secondary); border:1px solid var(--border); padding:20px; border-radius:8px; box-shadow:0 12px 36px rgba(0,0,0,0.5);">
        <h4 style="margin:0 0 10px 0; font-size:15px; font-weight:700; color:var(--text-primary); border-bottom:1px solid var(--border); padding-bottom:8px;">${escapeHtml(displayTitle)}</h4>
        <p style="margin:0 0 20px 0; font-size:12px; line-height:1.5; color:var(--text-muted);">${displayBody}</p>
        <div class="confirm-actions" style="display:flex; justify-content:flex-end; gap:8px;">
          <button class="confirm-cancel" style="padding:6px 12px; background:transparent; border:1px solid var(--border); border-radius:4px; color:var(--text-muted); font-size:11px; font-weight:600; cursor:pointer;">${escapeHtml(cancelText)}</button>
          <button class="confirm-danger" style="padding:6px 12px; background:var(--accent); border:none; border-radius:4px; color:#fff; font-size:11px; font-weight:600; cursor:pointer;">${escapeHtml(confirmText)}</button>
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

