import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';
import { editorDom } from '../../framework';
import { getState } from '../../state';
import { handleEditorSave, _originalContent, clearOriginalContent, getDirtyBufferCount, clearBufferCache, _fileBufferCache } from './viewer';
import { getMonacoContent, getCurrentMonacoFilePath } from './monaco/instance';

export function hasUnsavedChanges(): boolean {
  if (getDirtyBufferCount() > 0) return true;
  if (_originalContent === null) return false;
  const current = getMonacoContent();
  const normalize = (str: string) => str.replace(/\r\n/g, '\n');
  return normalize(current) !== normalize(_originalContent);
}

export async function confirmDiscardOrSave(): Promise<boolean> {
  const state = getState();
  const currentPath = state.editorSelectedFile || getCurrentMonacoFilePath();

  // Make sure active Monaco buffer is synced into cache
  if (currentPath && _originalContent !== null) {
    const currentText = getMonacoContent();
    const normalize = (str: string) => str.replace(/\r\n/g, '\n');
    const isDirty = normalize(currentText) !== normalize(_originalContent);
    _fileBufferCache.set(currentPath, {
      current: currentText,
      original: _originalContent,
      isDirty,
    });
  }

  const dirtyBuffers = Array.from(_fileBufferCache.entries()).filter(([_, b]) => b.isDirty);
  if (dirtyBuffers.length === 0) return true;

  const choice = await showUnsavedChangesModal(dirtyBuffers, currentPath);

  if (choice === 'save') {
    await handleEditorSave();
    clearOriginalContent();
    clearBufferCache();
    return true;
  } else if (choice === 'discard') {
    clearOriginalContent();
    clearBufferCache();
    return true;
  }

  return false;
}

function generateLineDiff(original: string, current: string): string {
  const normOriginal = original.replace(/\r\n/g, '\n');
  const normCurrent = current.replace(/\r\n/g, '\n');
  const origLines = normOriginal.split('\n');
  const currLines = normCurrent.split('\n');

  interface DiffItem {
    type: 'added' | 'deleted' | 'unchanged';
    text: string;
  }

  const rawDiff: DiffItem[] = [];
  let i = 0, j = 0;

  while (i < origLines.length || j < currLines.length) {
    if (i < origLines.length && j < currLines.length) {
      if (origLines[i] === currLines[j]) {
        rawDiff.push({ type: 'unchanged', text: origLines[i] });
        i++; j++;
      } else {
        let foundMatch = false;
        for (let look = 1; look < 5; look++) {
          if (i + look < origLines.length && origLines[i + look] === currLines[j]) {
            for (let d = 0; d < look; d++) {
              rawDiff.push({ type: 'deleted', text: origLines[i + d] });
            }
            i += look;
            foundMatch = true;
            break;
          }
          if (j + look < currLines.length && origLines[i] === currLines[j + look]) {
            for (let a = 0; a < look; a++) {
              rawDiff.push({ type: 'added', text: currLines[j + a] });
            }
            j += look;
            foundMatch = true;
            break;
          }
        }
        if (!foundMatch) {
          rawDiff.push({ type: 'deleted', text: origLines[i] });
          rawDiff.push({ type: 'added', text: currLines[j] });
          i++; j++;
        }
      }
    } else if (i < origLines.length) {
      rawDiff.push({ type: 'deleted', text: origLines[i] });
      i++;
    } else if (j < currLines.length) {
      rawDiff.push({ type: 'added', text: currLines[j] });
      j++;
    }
  }

  const contextSize = 2;
  const showFlags = new Array(rawDiff.length).fill(false);

  for (let k = 0; k < rawDiff.length; k++) {
    if (rawDiff[k].type !== 'unchanged') {
      showFlags[k] = true;
      for (let c = 1; c <= contextSize; c++) {
        if (k - c >= 0) showFlags[k - c] = true;
      }
      for (let c = 1; c <= contextSize; c++) {
        if (k + c < rawDiff.length) showFlags[k + c] = true;
      }
    }
  }

  const htmlLines: string[] = [];
  let inCollapse = false;
  let collapsedCount = 0;

  for (let k = 0; k < rawDiff.length; k++) {
    if (showFlags[k]) {
      if (inCollapse) {
        htmlLines.push(`
          <div class="diff-line-collapsed" style="color:var(--text-muted);font-family:monospace;padding:6px 12px;background:rgba(0,0,0,0.15);border-top:1px dashed var(--border);border-bottom:1px dashed var(--border);font-size:10px;text-align:center;user-select:none;">
            --- ${escapeHtml(t('editor.diff_collapsed_lines', { count: collapsedCount }))} ---
          </div>
        `);
        inCollapse = false;
        collapsedCount = 0;
      }

      const item = rawDiff[k];
      if (item.type === 'unchanged') {
        htmlLines.push(`<div class="diff-line unchanged" style="color:var(--text-muted);font-family:monospace;white-space:pre-wrap;padding:2px 8px;">  ${escapeHtml(item.text)}</div>`);
      } else if (item.type === 'deleted') {
        htmlLines.push(`<div class="diff-line deleted" style="background:rgba(232, 17, 35, 0.15);color:#f1707b;font-family:monospace;white-space:pre-wrap;padding:2px 8px;">- ${escapeHtml(item.text)}</div>`);
      } else if (item.type === 'added') {
        htmlLines.push(`<div class="diff-line added" style="background:rgba(16, 124, 65, 0.15);color:#57cf84;font-family:monospace;white-space:pre-wrap;padding:2px 8px;">+ ${escapeHtml(item.text)}</div>`);
      }
    } else {
      inCollapse = true;
      collapsedCount++;
    }
  }

  if (inCollapse) {
    htmlLines.push(`
      <div class="diff-line-collapsed" style="color:var(--text-muted);font-family:monospace;padding:6px 12px;background:rgba(0,0,0,0.15);border-top:1px dashed var(--border);border-bottom:1px dashed var(--border);font-size:10px;text-align:center;user-select:none;">
        --- ${escapeHtml(t('editor.diff_collapsed_lines', { count: collapsedCount }))} ---
      </div>
    `);
  }

  return `<div style="max-height:280px;overflow-y:auto;border:1px solid var(--border);border-radius:4px;background:var(--bg-secondary);padding:4px;font-size:11px;line-height:1.4;">${htmlLines.join('')}</div>`;
}

function showUnsavedChangesModal(
  dirtyBuffers: [string, { current: string; original: string; isDirty: boolean }][],
  activePath: string | null
): Promise<'save' | 'discard' | 'cancel'> {
  return new Promise((resolve) => {
    const isMulti = dirtyBuffers.length > 1;
    let selectedIndex = 0;
    if (activePath) {
      const idx = dirtyBuffers.findIndex(([p]) => p === activePath);
      if (idx >= 0) selectedIndex = idx;
    }

    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay visible';
    overlay.style.zIndex = '2000';
    overlay.style.position = 'fixed';
    overlay.style.top = '0';
    overlay.style.left = '0';
    overlay.style.right = '0';
    overlay.style.bottom = '0';
    overlay.style.background = 'rgba(0,0,0,0.6)';
    overlay.style.backdropFilter = 'blur(4px)';
    overlay.style.display = 'flex';
    overlay.style.alignItems = 'center';
    overlay.style.justifyContent = 'center';

    const descText = isMulti
      ? (t('editor.unsaved_multi_desc', { count: dirtyBuffers.length }) || `You have unsaved changes across ${dirtyBuffers.length} files. Review or choose an action:`)
      : (t('editor.unsaved_desc') || 'You have unsaved changes in this file. Review the changes below:');

    const discardBtnText = isMulti
      ? (t('editor.unsaved_discard_all') || 'Discard All')
      : (t('editor.unsaved_discard') || 'Discard Changes');

    const saveBtnText = isMulti
      ? (t('editor.unsaved_save_all_continue') || 'Save All & Continue')
      : (t('editor.unsaved_save_continue') || 'Save & Continue');

    const fileTabsHtml = isMulti
      ? `<div class="unsaved-file-tabs" style="display:flex;gap:6px;overflow-x:auto;padding-bottom:4px;border-bottom:1px solid var(--border);">
          ${dirtyBuffers.map(([p], idx) => {
            const fileName = p.replace(/^.*[/\\]/, '');
            const isActive = idx === selectedIndex;
            return `
              <button class="unsaved-tab-chip" data-idx="${idx}" style="display:inline-flex;align-items:center;gap:5px;padding:4px 10px;border-radius:6px;font-size:11px;font-weight:600;cursor:pointer;border:1px solid ${isActive ? 'var(--accent)' : 'var(--border)'};background:${isActive ? 'rgba(0,188,255,0.15)' : 'var(--bg-secondary)'};color:${isActive ? 'var(--text-primary)' : 'var(--text-secondary)'};user-select:none;">
                <span>📄</span>
                <span>${escapeHtml(fileName)}</span>
                <span style="color:#f1707b;font-size:9px;">●</span>
              </button>
            `;
          }).join('')}
        </div>`
      : '';

    const initialDiff = generateLineDiff(dirtyBuffers[selectedIndex][1].original, dirtyBuffers[selectedIndex][1].current);

    overlay.innerHTML = `
      <div class="modal" style="width: 750px; max-width: 90vw;">
        <div class="modal-header">
          <h3>${escapeHtml(t('editor.unsaved_title'))}</h3>
          <button class="modal-close-btn" id="unsaved-close-x">✕</button>
        </div>
        <div class="modal-body" style="gap:12px;padding:20px;">
          <div style="font-size:13px;color:var(--text-muted);">
            ${escapeHtml(descText)}
          </div>
          ${fileTabsHtml}
          <div id="unsaved-diff-container">
            ${initialDiff}
          </div>
        </div>
        <div class="modal-footer" style="padding:16px 20px;">
          <button id="unsaved-discard" class="btn-secondary" style="background:#a80000;color:white;border-color:#a80000;cursor:pointer;">${escapeHtml(discardBtnText)}</button>
          <button id="unsaved-cancel" class="btn-secondary" style="cursor:pointer;">${escapeHtml(t('common.cancel'))}</button>
          <button id="unsaved-save" class="btn-primary" style="cursor:pointer;">${escapeHtml(saveBtnText)}</button>
        </div>
      </div>
    `;

    overlay.id = 'unsaved-modal';
    document.body.appendChild(overlay);

    // Interactive tab switching between dirty files
    if (isMulti) {
      const diffContainer = overlay.querySelector('#unsaved-diff-container');
      const tabs = overlay.querySelectorAll('.unsaved-tab-chip');
      tabs.forEach(tab => {
        tab.addEventListener('click', () => {
          const idx = parseInt((tab as HTMLElement).dataset.idx || '0', 10);
          selectedIndex = idx;
          tabs.forEach((t, i) => {
            const active = i === idx;
            (t as HTMLElement).style.borderColor = active ? 'var(--accent)' : 'var(--border)';
            (t as HTMLElement).style.background = active ? 'rgba(0,188,255,0.15)' : 'var(--bg-secondary)';
            (t as HTMLElement).style.color = active ? 'var(--text-primary)' : 'var(--text-secondary)';
          });
          if (diffContainer) {
            diffContainer.innerHTML = generateLineDiff(dirtyBuffers[idx][1].original, dirtyBuffers[idx][1].current);
          }
        });
      });
    }

    const cleanUp = () => {
      if (document.body.contains(overlay)) {
        document.body.removeChild(overlay);
      }
    };

    editorDom.elMaybe('unsaved-close-x')?.addEventListener('click', () => {
      cleanUp();
      resolve('cancel');
    });
    editorDom.elMaybe('unsaved-cancel')?.addEventListener('click', () => {
      cleanUp();
      resolve('cancel');
    });
    editorDom.elMaybe('unsaved-discard')?.addEventListener('click', () => {
      cleanUp();
      resolve('discard');
    });
    editorDom.elMaybe('unsaved-save')?.addEventListener('click', () => {
      cleanUp();
      resolve('save');
    });
  });
}
