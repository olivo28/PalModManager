// Re-export all features from editor submodules.
export { renderEditorModTree, switchEditorMod, renderFileTree, populateEditorModSelect } from './tree';
export { _originalContent, _lastFilePerMod, clearOriginalContent, syncHighlight, loadFileContent, stripJsonComments, handleEditorSave, handleEditorFormat, handleEditorPreview, loadEditorData } from './viewer';
export { findMatches, findCurrentMatch, resetFindMatches, openFind, closeFind, updateFindMatches, scrollToMatch, findNext, findPrev, setupEditorFindHandlers } from './search';
export { hasUnsavedChanges, confirmDiscardOrSave } from './unsaved';
export { setupEditorKeybindings, handleEditorModChange, switchTab, openFileAtLine } from './keybindings';

// Originally openConfigEditor was defined in editorView.ts:
import { updateState } from '../../state';
import { switchTab } from './keybindings';
import { renderEditorModTree } from './tree';
import { loadEditorData, _lastFilePerMod } from './viewer';

export async function openConfigEditor(modId: string): Promise<void> {
  updateState({ activeTab: 'editor', editorModId: modId, editorSelectedFile: null });
  switchTab('editor');
  renderEditorModTree();
  await loadEditorData(modId);
  const lastFile = _lastFilePerMod[modId];
  if (lastFile) {
    const item = document.querySelector(`.editor-file-item[data-path="${CSS.escape(lastFile)}"]`) as HTMLElement | null;
    if (item) { item.click(); return; }
  }
  const firstFile = document.querySelector('.editor-file-item') as HTMLElement | null;
  if (firstFile) firstFile.click();
}
