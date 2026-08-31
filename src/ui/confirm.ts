import { escapeHtml } from '../utils/helpers';
import { t } from '../utils/i18n';

function formatConfirmText(text: string): string {
  return text.split('\n\n').map(para => {
    const trimmed = para.trim();
    if (!trimmed) return '';

    let safe = escapeHtml(trimmed);

    // Parse **bold** into <strong>
    safe = safe.replace(/\*\*([^*]+)\*\*/g, '<strong style="color:var(--text-primary); font-weight:600;">$1</strong>');

    // Parse *italic* into <em>
    safe = safe.replace(/\*([^*]+)\*/g, '<em>$1</em>');

    // Parse `code` into <code>
    safe = safe.replace(/`([^`]+)`/g, '<code style="background:rgba(255,255,255,0.08); padding:1px 5px; border-radius:3px; font-size:11px; font-family:monospace; color:var(--text-primary);">$1</code>');

    // Handle single newlines inside paragraph
    safe = safe.replace(/\n/g, '<br/>');

    // Render alert / warning notices in dedicated callout cards
    if (/^(📢|⚠️|ℹ️|🚨)/u.test(trimmed)) {
      return `
        <div style="background:rgba(245, 158, 11, 0.09); border:1px solid rgba(245, 158, 11, 0.28); border-left:3px solid #f59e0b; border-radius:6px; padding:10px 12px; margin-bottom:14px; font-size:12px; line-height:1.55; color:var(--text-secondary);">
          ${safe}
        </div>
      `;
    }

    return `<p style="margin:0 0 10px 0; font-size:12px; line-height:1.5; color:var(--text-muted);">${safe}</p>`;
  }).join('');
}

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
    const formattedBody = formatConfirmText(displayBody);

    const overlay = document.createElement('div');
    overlay.className = 'confirm-overlay';
    overlay.style.cssText = 'position:fixed; inset:0; background:rgba(0,0,0,0.65); backdrop-filter:blur(4px); display:flex; align-items:center; justify-content:center; z-index:99999;';
    overlay.innerHTML = `
      <div class="confirm-box" style="min-width:340px; max-width:460px; background:var(--bg-secondary); border:1px solid var(--border); padding:22px; border-radius:10px; box-shadow:0 16px 40px rgba(0,0,0,0.6);">
        <h4 style="margin:0 0 14px 0; font-size:15px; font-weight:700; color:var(--text-primary); border-bottom:1px solid var(--border); padding-bottom:10px;">${escapeHtml(displayTitle)}</h4>
        <div class="confirm-body" style="margin-bottom:18px;">${formattedBody}</div>
        <div class="confirm-actions" style="display:flex; justify-content:flex-end; gap:8px;">
          <button class="confirm-cancel" style="padding:7px 14px; background:transparent; border:1px solid var(--border); border-radius:6px; color:var(--text-muted); font-size:12px; font-weight:600; cursor:pointer; transition:all 0.15s ease;">${escapeHtml(btnCancelText)}</button>
          <button class="confirm-danger" style="padding:7px 16px; background:var(--accent); border:none; border-radius:6px; color:#fff; font-size:12px; font-weight:600; cursor:pointer; transition:all 0.15s ease;">${escapeHtml(btnConfirmText)}</button>
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

