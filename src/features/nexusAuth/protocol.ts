import {
  checkNexusProtocolStatus,
  registerNexusProtocol,
  unregisterNexusProtocol,
  type DetailedProtocolInfo,
} from '../../api';
import { showToast } from '../../ui/toast';
import { t } from '../../utils/i18n';

export async function getProtocolState(): Promise<DetailedProtocolInfo> {
  try {
    return await checkNexusProtocolStatus();
  } catch {
    return {
      palmodmanager: { status: 'NotRegistered' },
      nxm: { status: 'NotRegistered' },
    };
  }
}

export async function registerProtocolScheme(): Promise<boolean> {
  try {
    await registerNexusProtocol();
    showToast(t('toasts.nexus_protocol_registered'), 'success');
    const { renderNexusAccountUI } = await import('./ui');
    renderNexusAccountUI();
    return true;
  } catch (err: any) {
    console.error('[NexusAuth] Protocol registration failed:', err);
    showToast(t('toasts.nexus_protocol_register_failed', { error: String(err) }), 'error');
    return false;
  }
}

export async function updateProtocolStatusUI(): Promise<void> {
  const pmmStatusBadge = document.getElementById('nexus-protocol-status-badge');
  const pmmActionContainer = document.getElementById('nexus-protocol-actions');

  const nxmStatusBadge = document.getElementById('nexus-nxm-status-badge');
  const nxmCurrentHandler = document.getElementById('nexus-nxm-current-handler');
  const nxmActionContainer = document.getElementById('nexus-nxm-actions');

  try {
    const details = await checkNexusProtocolStatus();

    // 1. PalModManager login protocol UI
    if (pmmStatusBadge && pmmActionContainer) {
      if (details.palmodmanager.status === 'Registered') {
        pmmStatusBadge.className = 'badge badge-success';
        pmmStatusBadge.textContent = t('settings.nexus_proto_registered');
        pmmActionContainer.innerHTML = `
          <button type="button" class="btn btn-secondary btn-sm" id="btn-re-register-pmm-proto" style="font-size:11px;">
            ${t('settings.nexus_proto_update_btn')}
          </button>
          <button type="button" class="btn btn-danger btn-sm" id="btn-unregister-pmm-proto" style="font-size:11px;">
            ${t('settings.nexus_proto_unregister_btn')}
          </button>
        `;
        document.getElementById('btn-re-register-pmm-proto')?.addEventListener('click', async () => {
          await registerNexusProtocol('palmodmanager');
          showToast(t('toasts.saved'), 'success');
          updateProtocolStatusUI();
        });
        document.getElementById('btn-unregister-pmm-proto')?.addEventListener('click', async () => {
          await unregisterNexusProtocol('palmodmanager');
          showToast(t('toasts.saved'), 'info');
          updateProtocolStatusUI();
        });
      } else if (details.palmodmanager.status === 'OutdatedPath') {
        pmmStatusBadge.className = 'badge badge-warning';
        pmmStatusBadge.textContent = t('settings.nexus_proto_outdated');
        pmmActionContainer.innerHTML = `
          <button type="button" class="btn btn-warning btn-sm" id="btn-fix-pmm-protocol" style="font-size:11px;">
            ${t('settings.nexus_proto_update_btn')}
          </button>
        `;
        document.getElementById('btn-fix-pmm-protocol')?.addEventListener('click', async () => {
          await registerNexusProtocol('palmodmanager');
          showToast(t('toasts.saved'), 'success');
          updateProtocolStatusUI();
        });
      } else {
        pmmStatusBadge.className = 'badge badge-secondary';
        pmmStatusBadge.textContent = t('settings.nexus_proto_not_registered');
        pmmActionContainer.innerHTML = `
          <button type="button" class="btn btn-secondary btn-sm" id="btn-register-pmm-protocol" style="font-size:11px;">
            ${t('settings.nexus_proto_register_btn')}
          </button>
        `;
        document.getElementById('btn-register-pmm-protocol')?.addEventListener('click', async () => {
          await registerNexusProtocol('palmodmanager');
          showToast(t('toasts.saved'), 'success');
          updateProtocolStatusUI();
        });
      }
    }

    // 2. NXM 1-Click Downloads UI
    if (nxmStatusBadge && nxmActionContainer && nxmCurrentHandler) {
      const isPmm = details.nxm.status === 'Registered';
      const handlerName = details.nxmHandlerName || (details.nxm.status !== 'NotRegistered' ? 'Unknown App' : null);

      if (handlerName) {
        nxmCurrentHandler.innerHTML = `
          <span>${t('settings.nexus_nxm_current_app')}:</span> 
          <strong style="color: ${isPmm ? 'var(--success)' : 'var(--accent)'};">${handlerName}</strong>
          ${details.nxmHandlerPath ? `<span style="font-size:10px; font-weight:normal; opacity:0.75; display:block; word-break:break-all; margin-top:2px;">(${details.nxmHandlerPath})</span>` : ''}
        `;
      } else {
        nxmCurrentHandler.innerHTML = `<span>${t('settings.nexus_nxm_none')}</span>`;
      }

      if (isPmm) {
        nxmStatusBadge.className = 'badge badge-success';
        nxmStatusBadge.textContent = t('settings.nexus_nxm_active_badge');
        nxmActionContainer.innerHTML = `
          <button type="button" class="btn btn-danger btn-sm" id="btn-release-nxm" style="font-size:11px;">
            ${t('settings.nexus_nxm_release_btn')}
          </button>
        `;
        document.getElementById('btn-release-nxm')?.addEventListener('click', async () => {
          await unregisterNexusProtocol('nxm');
          showToast(t('toasts.saved'), 'info');
          updateProtocolStatusUI();
        });
      } else {
        nxmStatusBadge.className = 'badge badge-secondary';
        nxmStatusBadge.textContent = handlerName ? handlerName : t('settings.nexus_proto_not_registered');
        nxmActionContainer.innerHTML = `
          <button type="button" class="btn btn-secondary btn-sm" id="btn-claim-nxm" style="font-size:11px;">
            ${t('settings.nexus_nxm_claim_btn')}
          </button>
        `;
        document.getElementById('btn-claim-nxm')?.addEventListener('click', async () => {
          await registerNexusProtocol('nxm');
          showToast(t('toasts.saved'), 'success');
          updateProtocolStatusUI();
        });
      }
    }
  } catch (err) {
    console.warn('Failed to update protocol status UI:', err);
  }
}
