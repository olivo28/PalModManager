import { escapeHtml } from '../utils/helpers';
import { t } from '../utils/i18n';

export function showConfirm(
  titleOrMessage: string,
  message?: string,
  confirmText?: string,
  cancelText?: string
): Promise<boolean> {
  return new Promise((resolve) => {
    // Remove any leftover or duplicate confirm overlays
    document.querySelectorAll('.confirm-overlay').forEach(el => el.remove());

    const hasTitle = !!message;
    const displayTitle = hasTitle ? titleOrMessage : t('common.confirm');
    const displayBody = hasTitle ? message : titleOrMessage;
    const btnConfirmText = confirmText || t('common.confirm');
    const btnCancelText = cancelText || t('common.cancel');
    const formattedBody = displayBody.split('\n\n').map(para => `<p style="margin:0 0 10px 0; font-size:12px; line-height:1.5; color:var(--text-muted);">${para.replace(/\n/g, '<br/>')}</p>`).join('');

    const overlay = document.createElement('div');
    overlay.className = 'confirm-overlay';
    overlay.style.zIndex = '99999';
    overlay.innerHTML = `
      <div class="confirm-box" style="min-width:320px; max-width:440px; background:var(--bg-secondary); border:1px solid var(--border); padding:20px; border-radius:8px; box-shadow:0 12px 36px rgba(0,0,0,0.5);">
        <h4 style="margin:0 0 12px 0; font-size:15px; font-weight:700; color:var(--text-primary); border-bottom:1px solid var(--border); padding-bottom:8px;">${escapeHtml(displayTitle)}</h4>
        <div class="confirm-body" style="margin-bottom:18px;">${formattedBody}</div>
        <div class="confirm-actions" style="display:flex; justify-content:flex-end; gap:8px;">
          <button class="confirm-cancel" style="padding:6px 12px; background:transparent; border:1px solid var(--border); border-radius:4px; color:var(--text-muted); font-size:11px; font-weight:600; cursor:pointer;">${escapeHtml(btnCancelText)}</button>
          <button class="confirm-danger" style="padding:6px 12px; background:var(--accent); border:none; border-radius:4px; color:#fff; font-size:11px; font-weight:600; cursor:pointer;">${escapeHtml(btnConfirmText)}</button>
        </div>
      </div>
    `;
    document.body.appendChild(overlay);

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        e.preventDefault();
        cleanup(false);
      } else if (e.key === 'Enter') {
        e.stopPropagation();
        e.preventDefault();
        cleanup(true);
      }
    };

    const cleanup = (result: boolean) => {
      document.removeEventListener('keydown', onKeyDown, true);
      overlay.remove();
      resolve(result);
    };

    document.addEventListener('keydown', onKeyDown, true);

    overlay.querySelector('.confirm-cancel')!.addEventListener('click', () => cleanup(false));
    overlay.querySelector('.confirm-danger')!.addEventListener('click', () => cleanup(true));
  });
}

export function showPrompt(message: string, defaultValue = ''): Promise<string | null> {
  return new Promise((resolve) => {
    const overlay = document.createElement('div');
    overlay.className = 'confirm-overlay';
    overlay.style.zIndex = '99999';
    overlay.innerHTML = `
      <div class="confirm-box">
        <p>${escapeHtml(message)}</p>
        <input type="text" class="confirm-input" value="${escapeHtml(defaultValue)}" style="width: 100%; padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius); color: var(--text-primary); font-size: 12px; margin-bottom: 12px; outline: none;" />
        <div class="confirm-actions">
          <button class="confirm-cancel">${escapeHtml(t('common.cancel'))}</button>
          <button class="confirm-danger" style="background:var(--accent);">${escapeHtml(t('common.ok'))}</button>
        </div>
      </div>
    `;
    document.body.appendChild(overlay);

    const input = overlay.querySelector('.confirm-input') as HTMLInputElement;
    input.focus();
    input.select();

    const cleanup = (val: string | null) => {
      document.removeEventListener('keydown', onKeyDown, true);
      overlay.remove();
      resolve(val);
    };

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        e.preventDefault();
        cleanup(null);
      } else if (e.key === 'Enter') {
        e.stopPropagation();
        e.preventDefault();
        cleanup(input.value);
      }
    };

    document.addEventListener('keydown', onKeyDown, true);

    overlay.querySelector('.confirm-cancel')!.addEventListener('click', () => cleanup(null));
    overlay.querySelector('.confirm-danger')!.addEventListener('click', () => cleanup(input.value));
  });
}

