import { escapeHtml } from '../../rendering';
import { t } from '../../../../utils/i18n';

export function getHealthBadge(status: string, hasExternalEdits?: boolean): string {
  let html = '';
  if (status === 'corrupt') {
    html = `<span style="font-size: 10px; font-weight: 700; color: #ff5f56; background: rgba(255, 95, 86, 0.15); border: 1px solid rgba(255, 95, 86, 0.3); border-radius: 12px; padding: 2px 8px; text-transform: uppercase;">🔴 ${escapeHtml(t('scanner.status_corrupt') || 'Corrupted')}</span>`;
  } else if (status === 'warning') {
    html = `<span style="font-size: 10px; font-weight: 700; color: #ffaa00; background: rgba(255, 170, 0, 0.15); border: 1px solid rgba(255, 170, 0, 0.3); border-radius: 12px; padding: 2px 8px; text-transform: uppercase;">🟡 ${escapeHtml(t('scanner.status_warning') || 'Mod Issues')}</span>`;
  } else if (status === 'external_edits' || (!status && hasExternalEdits)) {
    html = `<span style="font-size: 10px; font-weight: 700; color: #ffd166; background: rgba(255, 209, 102, 0.15); border: 1px solid rgba(255, 209, 102, 0.3); border-radius: 12px; padding: 2px 8px; text-transform: uppercase;">⚠️ ${escapeHtml(t('scanner.status_external_edits') || 'External Edits')}</span>`;
  } else {
    html = `<span style="font-size: 10px; font-weight: 700; color: #4af626; background: rgba(74, 246, 38, 0.15); border: 1px solid rgba(74, 246, 38, 0.3); border-radius: 12px; padding: 2px 8px; text-transform: uppercase;">🟢 ${escapeHtml(t('scanner.status_healthy') || 'Healthy')}</span>`;
  }

  if (hasExternalEdits && status === 'warning') {
    html += ` <span style="font-size: 9.5px; font-weight: 700; color: #ffd166; background: rgba(255, 209, 102, 0.15); border: 1px solid rgba(255, 209, 102, 0.3); border-radius: 12px; padding: 2px 6px; text-transform: uppercase;">⚠️ ${escapeHtml(t('scanner.badge_external_edits') || 'External Edits')}</span>`;
  }

  return html;
}
