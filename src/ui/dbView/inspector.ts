import { invoke } from '@tauri-apps/api/core';
import { dbDom } from '../../framework';
import { showToast } from '../toast';
import { t } from '../../utils/i18n';
import { dbState } from './state';
import { maskSensitiveData } from './tables';

export function selectRecord(type: 'mod' | 'profile' | 'settings', id: string, data: unknown): void {
  dbState.selectedRecordType = type;
  dbState.selectedRecordId = id;
  dbState.selectedUsmapItem = null;

  const customContainer = document.getElementById('db-inspector-custom-container');
  const editor = dbDom.elMaybe('db-json-editor');
  const inspectorTitle = document.getElementById('db-inspector-title');
  const inspectorActions = document.getElementById('db-inspector-actions');

  if (customContainer) customContainer.style.display = 'none';
  if (editor) {
    editor.style.display = 'block';
    const maskedData = maskSensitiveData(data);
    editor.value = JSON.stringify(maskedData, null, 2);
    editor.disabled = false;
  }

  if (inspectorTitle) inspectorTitle.textContent = t('db.inspector_title');
  if (inspectorActions) {
    inspectorActions.innerHTML = `
      <span id="db-json-status" class="db-json-status"></span>
      <button id="db-save-btn" class="db-save-btn" disabled>${t('db.btn_save')}</button>
    `;
    dbDom.elMaybe('db-save-btn')?.addEventListener('click', handleSaveRecord);
  }

  updateJsonStatus(true);
}

export function clearInspector(): void {
  dbState.selectedRecordType = null;
  dbState.selectedRecordId = '';
  dbState.selectedUsmapItem = null;

  const customContainer = document.getElementById('db-inspector-custom-container');
  const editor = dbDom.elMaybe('db-json-editor');
  const inspectorTitle = document.getElementById('db-inspector-title');

  if (customContainer) customContainer.style.display = 'none';
  if (editor) {
    editor.style.display = 'block';
    editor.value = '';
    editor.disabled = true;
  }
  if (inspectorTitle) inspectorTitle.textContent = t('db.inspector_title');
  updateJsonStatus(null);
}

export function onJsonEditorInput(): void {
  const editor = dbDom.elMaybe('db-json-editor');
  if (!editor) return;
  try {
    JSON.parse(editor.value);
    updateJsonStatus(true);
  } catch {
    updateJsonStatus(false);
  }
}

export function updateJsonStatus(valid: boolean | null): void {
  const status = dbDom.elMaybe('db-json-status');
  const saveBtn = dbDom.elMaybe('db-save-btn');
  if (!status) return;

  if (valid === null) {
    status.textContent = '';
    status.className = 'db-json-status';
    if (saveBtn) saveBtn.disabled = true;
  } else if (valid) {
    status.textContent = t('db.valid_json') || '✓ Valid JSON';
    status.className = 'db-json-status valid';
    if (saveBtn) saveBtn.disabled = dbState.selectedRecordType === null;
  } else {
    status.textContent = t('db.invalid_json') || '✗ Invalid JSON';
    status.className = 'db-json-status invalid';
    if (saveBtn) saveBtn.disabled = true;
  }
}

export async function handleSaveRecord(): Promise<void> {
  if (!dbState.selectedRecordType) return;

  const editor = dbDom.elMaybe('db-json-editor');
  if (!editor) return;

  let parsed: unknown;
  try {
    parsed = JSON.parse(editor.value);
  } catch {
    showToast(t('toasts.export_failed', { error: 'Invalid JSON' }), 'error');
    return;
  }

  try {
    await invoke('db_write_record', {
      recordType: dbState.selectedRecordType,
      recordId: dbState.selectedRecordId,
      json: JSON.stringify(parsed),
    });

    showToast(t('toasts.settings_saved'), 'success');

    if (dbState.selectedRecordType === 'settings') {
      const { getSettings } = await import('../../api');
      const updatedSettings = await getSettings();
      const { updateState } = await import('../../state');
      updateState({ currentSettings: updatedSettings });
      if (updatedSettings.language) {
        const { initI18n } = await import('../../utils/i18n');
        initI18n(updatedSettings.language);
      }
    }

    // Ask if user wants to re-scan mods
    const { showConfirm } = await import('../confirm');
    const doRescan = await showConfirm(
      t('db.confirm_rescan')
    );
    if (doRescan) {
      const { loadMods } = await import('../modsView');
      await loadMods();
      showToast(t('dependencies.up_to_date'), 'info');
    }

    // Reload snapshot so the grid reflects changes
    const { loadSnapshot } = await import('./index');
    await loadSnapshot();
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  }
}
