import { createScope } from '../core/query';

export interface EditorDomMap {
  'editor-view': HTMLElement;
  'editor-toolbar': HTMLElement;
  'editor-mod-select': HTMLSelectElement;
  'editor-current-mod-name': HTMLElement;
  'editor-format-btn': HTMLButtonElement;
  'editor-preview-btn': HTMLButtonElement;
  'editor-save-btn': HTMLButtonElement;
  'editor-cursor-pos': HTMLElement;
  'editor-status': HTMLElement;
  'editor-body': HTMLElement;
  'editor-mod-panel': HTMLElement;
  'editor-mod-tree': HTMLElement;
  'editor-file-tree': HTMLElement;
  'editor-content-area': HTMLElement;
  'editor-file-path': HTMLElement;
  'editor-input-wrap': HTMLElement;
  'editor-gutter': HTMLElement;
  'editor-highlight': HTMLElement;
  'editor-highlight-code': HTMLElement;
  'editor-content': HTMLTextAreaElement;
  'editor-preview': HTMLElement;
  'editor-find-bar': HTMLElement;
  'editor-find-input': HTMLInputElement;
  'editor-find-count': HTMLElement;
  'editor-find-prev': HTMLButtonElement;
  'editor-find-next': HTMLButtonElement;
  'editor-find-close': HTMLButtonElement;
}

export const editorDom = createScope<EditorDomMap>('editor');
