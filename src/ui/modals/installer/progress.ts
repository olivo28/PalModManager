import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { InstallProgressPayload } from '../../../types';
import { t } from '../../../utils/i18n';
import { escapeHtml } from '../../../utils/helpers';

export type InstallProgressCallback = (payload: InstallProgressPayload) => void;

/**
 * Executes an async installation/update action while listening for backend
 * "install-progress" events. Guaranteed cleanup of event listener in finally block.
 */
export async function withInstallProgress<T>(
  action: () => Promise<T>,
  onProgress?: InstallProgressCallback
): Promise<T> {
  let unlisten: UnlistenFn | null = null;
  if (onProgress) {
    try {
      unlisten = await listen<InstallProgressPayload>('install-progress', (event) => {
        try {
          onProgress(event.payload);
        } catch (e) {
          console.error('[InstallProgress] Callback error:', e);
        }
      });
    } catch (err) {
      console.warn('[InstallProgress] Failed to attach progress listener:', err);
    }
  }

  try {
    return await action();
  } finally {
    if (unlisten) {
      unlisten();
    }
  }
}

/**
 * Helper to update the last log line in an installation progress console in real time.
 */
export function createProgressLogUpdater(
  listEl: HTMLElement,
  lines: string[],
  itemClass = '',
  itemSuffix = ''
): InstallProgressCallback {
  return (p: InstallProgressPayload) => {
    const stageKey = `installer.install_progress_${p.stage}`;
    const stageLabel = t(stageKey) || p.stage;
    const text = `&gt; ${escapeHtml(stageLabel)} (${p.percent}%)${itemSuffix ? ' - ' + escapeHtml(itemSuffix) : ''}...`;
    lines[lines.length - 1] = itemClass
      ? `<div class="${itemClass}" style="color:#e0af68;font-style:italic;">${text}</div>`
      : `<div style="color:#e0af68;">${text}</div>`;
    listEl.innerHTML = lines.join('');
    listEl.scrollTop = listEl.scrollHeight;
  };
}
