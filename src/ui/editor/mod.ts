// Re-export all features from editor submodules.
export { renderEditorModTree, switchEditorMod, renderFileTree, populateEditorModSelect, refreshEditorFileTree, findBestConfigFile, revealAndSelectFile } from './tree';
export { _originalContent, _lastFilePerMod, clearOriginalContent, syncHighlight, loadFileContent, stripJsonComments, handleEditorSave, handleEditorFormat, handleEditorPreview, loadEditorData } from './viewer';
export { findMatches, findCurrentMatch, resetFindMatches, openFind, closeFind, updateFindMatches, scrollToMatch, findNext, findPrev, setupEditorFindHandlers } from './search';
export { hasUnsavedChanges, confirmDiscardOrSave } from './unsaved';
export { setupEditorKeybindings, handleEditorModChange, switchTab, openFileAtLine, jumpToLineInEditor } from './keybindings';
export { setupEditorFsWatcher } from './watcher';
export { openEditorDiffModal, hideDiffModal } from './diffModal';

// Originally openConfigEditor was defined in editorView.ts:
import { getState, updateState } from '../../state';
import { switchTab } from './keybindings';
import { renderEditorModTree, findBestConfigFile, revealAndSelectFile } from './tree';
import { loadEditorData, _lastFilePerMod } from './viewer';

export async function openConfigEditor(modId: string): Promise<void> {
  updateState({ activeTab: 'editor', editorModId: modId, editorSelectedFile: null });
  switchTab('editor');
  renderEditorModTree();
  await loadEditorData(modId);

  const state = getState();
  const files = state.editorFiles || [];

  // 1. Search for the best configuration / main script file
  const bestFile = findBestConfigFile(files);
  if (bestFile && revealAndSelectFile(bestFile)) {
    return;
  }

  // 2. Fallback to last visited file for this mod
  const lastFile = _lastFilePerMod[modId];
  if (lastFile && revealAndSelectFile(lastFile)) {
    return;
  }

  // 3. Fallback to first available file in the tree
  const firstFile = document.querySelector('.editor-file-item') as HTMLElement | null;
  if (firstFile) {
    const path = firstFile.dataset.path;
    if (path) revealAndSelectFile(path);
    else firstFile.click();
  }
}
