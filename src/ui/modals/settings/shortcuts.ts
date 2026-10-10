import { getAllShortcuts } from '../../../features/shortcuts/registry';
import { getEffectiveKey, getShortcutDisplay, eventToKeyCombination, formatKeyDisplay } from '../../../features/shortcuts/formatter';
import { saveCustomBinding, resetCustomBinding, resetAllCustomBindings } from '../../../features/shortcuts/storage';
import { bus } from '../../../framework/core/bus';
import { settingsDom } from '../../../framework';
import { showToast } from '../../toast';
import { showConfirm } from '../../confirm';
import { t } from '../../../utils/i18n';
import { escapeHtml } from '../../../utils/helpers';
import type { ShortcutCategory, ShortcutDefinition } from '../../../features/shortcuts/types';

let _activeRecordingId: string | null = null;
let _recordingKeydownHandler: ((e: KeyboardEvent) => void) | null = null;
let _searchFilter = '';
let _shortcutsInitialized = false;

const CATEGORIES: { id: ShortcutCategory; labelKey: string }[] = [
  { id: 'mods', labelKey: 'shortcuts.category_mods' },
  { id: 'editor', labelKey: 'shortcuts.category_editor' },
  { id: 'navigation', labelKey: 'shortcuts.category_navigation' },
  { id: 'general', labelKey: 'shortcuts.category_general' },
];

/**
 * Checks if a key combination conflicts with another shortcut within the same scope.
 */
function findConflict(actionId: string, combo: string): ShortcutDefinition | null {
  const target = getAllShortcuts().find(s => s.id === actionId);
  if (!target) return null;

  for (const other of getAllShortcuts()) {
    if (other.id === actionId) continue;
    const otherKey = getEffectiveKey(other.id);
    if (!otherKey) continue;

    if (otherKey.toLowerCase() === combo.toLowerCase()) {
      // Conflict exists if scopes overlap (either is global, or scopes match)
      if (other.scope === 'global' || target.scope === 'global' || other.scope === target.scope) {
        return other;
      }
    }
  }
  return null;
}

/**
 * Cancel any ongoing key recording session.
 */
function stopRecording(): void {
  if (_recordingKeydownHandler) {
    window.removeEventListener('keydown', _recordingKeydownHandler, { capture: true });
    _recordingKeydownHandler = null;
  }
  _activeRecordingId = null;
}

/**
 * Start listening for key presses to assign to a specific shortcut action.
 */
function startRecording(actionId: string, recordBtn: HTMLButtonElement): void {
  stopRecording();
  _activeRecordingId = actionId;

  recordBtn.classList.add('recording');
  recordBtn.textContent = t('shortcuts.recording_prompt') || 'Press keys...';

  _recordingKeydownHandler = async (e: KeyboardEvent) => {
    e.preventDefault();
    e.stopPropagation();

    // ESC cancels recording without modifying the binding
    if (e.key === 'Escape') {
      stopRecording();
      renderShortcutsTab();
      return;
    }

    const combination = eventToKeyCombination(e);
    if (!combination) {
      // Pressed a modifier alone, keep listening
      return;
    }

    stopRecording();

    // Check for conflicts
    const conflict = findConflict(actionId, combination);
    if (conflict) {
      const conflictName = t(conflict.labelKey) || conflict.id;
      const confirmOverride = await showConfirm(
        t('shortcuts.conflict_prompt', { action: conflictName, key: combination }) ||
        `'${combination}' is already assigned to '${conflictName}'. Do you want to reassign it?`
      );
      if (!confirmOverride) {
        renderShortcutsTab();
        return;
      }
      // Unbind the previous shortcut
      saveCustomBinding(conflict.id, '');
    }

    saveCustomBinding(actionId, combination);
    bus.emit('shortcuts:changed');
    showToast(t('shortcuts.saved_success') || 'Shortcut updated', 'success');
    renderShortcutsTab();
  };

  window.addEventListener('keydown', _recordingKeydownHandler, { capture: true });
}

/**
 * Renders the shortcuts tab list inside #settings-pane-shortcuts.
 */
export function renderShortcutsTab(): void {
  const container = settingsDom.elMaybe('settings-shortcuts-list');
  if (!container) return;

  const all = getAllShortcuts();
  const filter = _searchFilter.trim().toLowerCase();

  const filtered = all.filter(s => {
    if (!filter) return true;
    const label = (t(s.labelKey) || s.id).toLowerCase();
    const desc = (s.descriptionKey ? t(s.descriptionKey) : '').toLowerCase();
    const key = getEffectiveKey(s.id).toLowerCase();
    return label.includes(filter) || desc.includes(filter) || key.includes(filter);
  });

  let html = '';

  for (const cat of CATEGORIES) {
    const items = filtered.filter(s => s.category === cat.id);
    if (items.length === 0) continue;

    html += `
      <div class="shortcuts-category-card">
        <div class="shortcuts-category-header">
          <span>${escapeHtml(t(cat.labelKey) || cat.id)}</span>
          <span style="margin-left:auto;opacity:0.7;font-weight:400;">(${items.length})</span>
        </div>
        <div class="shortcuts-category-items">
    `;

    for (const item of items) {
      const boundKey = getEffectiveKey(item.id);
      const isDefault = boundKey === item.defaultKey;
      const displayKey = boundKey ? formatKeyDisplay(boundKey) : (t('shortcuts.unassigned') || 'None');
      const isRecordingThis = _activeRecordingId === item.id;

      html += `
        <div class="shortcuts-item-row" data-action-id="${item.id}">
          <div class="shortcuts-item-info">
            <div class="shortcuts-item-title-row">
              <span class="shortcuts-item-title">${escapeHtml(t(item.labelKey) || item.id)}</span>
              <span class="shortcuts-item-scope">${escapeHtml(item.scope)}</span>
            </div>
            ${item.descriptionKey ? `<div class="shortcuts-item-desc">${escapeHtml(t(item.descriptionKey))}</div>` : ''}
          </div>
          <div class="shortcuts-item-controls">
            <button type="button" class="shortcuts-record-btn ${isRecordingThis ? 'recording' : ''}" data-action="record" data-id="${item.id}" title="${escapeHtml(t('shortcuts.click_to_record') || 'Click to change shortcut')}">
              ${isRecordingThis ? (escapeHtml(t('shortcuts.recording_prompt') || 'Press keys...')) : escapeHtml(displayKey)}
            </button>
            ${boundKey ? `
              <button type="button" class="shortcuts-action-btn" data-action="clear" data-id="${item.id}" title="${escapeHtml(t('shortcuts.btn_clear') || 'Clear')}">
                ✕
              </button>
            ` : ''}
            ${!isDefault ? `
              <button type="button" class="shortcuts-action-btn" data-action="reset" data-id="${item.id}" title="${escapeHtml(t('shortcuts.btn_reset') || 'Reset to default')}">
                ↺
              </button>
            ` : ''}
          </div>
        </div>
      `;
    }

    html += `
        </div>
      </div>
    `;
  }

  if (filtered.length === 0) {
    html = `
      <div style="text-align:center;padding:calc(24px * var(--ui-scale, 1));color:var(--text-muted);font-size:var(--text-sm, 12px);">
        ${escapeHtml(t('shortcuts.no_results') || 'No shortcuts found matching search.')}
      </div>
    `;
  }

  container.innerHTML = html;

  // Event delegations for controls
  container.querySelectorAll<HTMLButtonElement>('.shortcuts-record-btn').forEach(btn => {
    btn.onclick = (e) => {
      e.stopPropagation();
      const id = btn.dataset.id;
      if (!id) return;
      if (_activeRecordingId === id) {
        stopRecording();
        renderShortcutsTab();
      } else {
        startRecording(id, btn);
      }
    };
  });

  container.querySelectorAll<HTMLButtonElement>('.shortcuts-action-btn').forEach(btn => {
    btn.onclick = (e) => {
      e.stopPropagation();
      stopRecording();
      const action = btn.dataset.action;
      const id = btn.dataset.id;
      if (!id) return;

      if (action === 'clear') {
        saveCustomBinding(id, '');
        bus.emit('shortcuts:changed');
        renderShortcutsTab();
      } else if (action === 'reset') {
        resetCustomBinding(id);
        bus.emit('shortcuts:changed');
        renderShortcutsTab();
      }
    };
  });
}

/**
 * Initializes the shortcuts hub tab listeners and controls.
 */
export function setupShortcutsTab(): void {
  if (_shortcutsInitialized) return;
  _shortcutsInitialized = true;

  const searchInput = settingsDom.elMaybe('settings-shortcuts-search');
  if (searchInput) {
    searchInput.addEventListener('input', () => {
      _searchFilter = searchInput.value;
      renderShortcutsTab();
    });
  }

  const resetAllBtn = settingsDom.elMaybe('settings-shortcuts-reset-all');
  if (resetAllBtn) {
    resetAllBtn.addEventListener('click', async () => {
      stopRecording();
      const confirmed = await showConfirm(
        t('shortcuts.confirm_reset_all') || 'Reset all shortcuts to their default values?'
      );
      if (!confirmed) return;

      resetAllCustomBindings();
      bus.emit('shortcuts:changed');
      showToast(t('shortcuts.reset_all_success') || 'All shortcuts reset to defaults', 'success');
      renderShortcutsTab();
    });
  }
}
