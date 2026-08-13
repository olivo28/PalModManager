import { escapeHtml } from '../../utils/helpers';
import { handleEditorSave, _originalContent, clearOriginalContent } from './viewer';

export function hasUnsavedChanges(): boolean {
  if (_originalContent === null) return false;
  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement | null;
  if (!editorContent) return false;

  const normalize = (str: string) => str.replace(/\r\n/g, '\n');
  return normalize(editorContent.value) !== normalize(_originalContent);
}

export async function confirmDiscardOrSave(): Promise<boolean> {
  if (!hasUnsavedChanges()) return true;

  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;
  const current = editorContent.value;
  const original = _originalContent || '';

  const choice = await showUnsavedChangesModal(original, current);

  if (choice === 'save') {
    await handleEditorSave();
    clearOriginalContent();
    return true;
  } else if (choice === 'discard') {
    clearOriginalContent();
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
            --- Colapsadas ${collapsedCount} líneas sin cambios ---
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
        --- Colapsadas ${collapsedCount} líneas sin cambios ---
      </div>
    `);
  }

  return `<div style="max-height:280px;overflow-y:auto;border:1px solid var(--border);border-radius:4px;background:var(--bg-secondary);padding:4px;font-size:11px;line-height:1.4;">${htmlLines.join('')}</div>`;
}

function showUnsavedChangesModal(original: string, current: string): Promise<'save' | 'discard' | 'cancel'> {
  return new Promise((resolve) => {
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

    const diffHtml = generateLineDiff(original, current);

    overlay.innerHTML = `
      <div class="modal" style="width: 750px; max-width: 90vw;">
        <div class="modal-header">
          <h3>Unsaved Changes</h3>
          <button class="modal-close-btn" id="unsaved-close-x">✕</button>
        </div>
        <div class="modal-body" style="gap:12px;padding:20px;">
          <div style="font-size:13px;color:var(--text-muted);">
            You have unsaved changes in this file. Review the changes below:
          </div>
          ${diffHtml}
        </div>
        <div class="modal-footer" style="padding:16px 20px;">
          <button id="unsaved-discard" class="btn-secondary" style="background:#a80000;color:white;border-color:#a80000;cursor:pointer;">Discard Changes</button>
          <button id="unsaved-cancel" class="btn-secondary" style="cursor:pointer;">Cancel</button>
          <button id="unsaved-save" class="btn-primary" style="cursor:pointer;">Save & Continue</button>
        </div>
      </div>
    `;

    document.body.appendChild(overlay);

    const cleanUp = () => {
      document.body.removeChild(overlay);
    };

    document.getElementById('unsaved-close-x')!.addEventListener('click', () => {
      cleanUp();
      resolve('cancel');
    });
    document.getElementById('unsaved-cancel')!.addEventListener('click', () => {
      cleanUp();
      resolve('cancel');
    });
    document.getElementById('unsaved-discard')!.addEventListener('click', () => {
      cleanUp();
      resolve('discard');
    });
    document.getElementById('unsaved-save')!.addEventListener('click', () => {
      cleanUp();
      resolve('save');
    });
  });
}
