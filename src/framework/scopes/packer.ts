import { createScope } from '../core/query';

export interface PackerDomMap {
  'build-view': HTMLElement;
  'packer-projects-hub': HTMLElement;
  'packer-hub-new-btn': HTMLButtonElement;
  'packer-hub-create-card': HTMLElement;
  'packer-projects-grid': HTMLElement;
  'packer-workspace-view': HTMLElement;
  'packer-workspace-back-btn': HTMLButtonElement;
  'packer-workspace-title': HTMLElement;
  'packer-layout-toggle-bar': HTMLElement;
  'packer-view-list-btn': HTMLButtonElement;
  'packer-view-tree-btn': HTMLButtonElement;
  'packer-add-files-btn': HTMLButtonElement;
  'packer-add-folder-btn': HTMLButtonElement;
  'packer-new-virtual-folder-btn': HTMLButtonElement;
  'packer-autostruct-btn': HTMLButtonElement;
  'packer-preview-install-btn': HTMLButtonElement;
  'packer-clear-btn': HTMLButtonElement;
  'packer-files-container': HTMLElement;
  'packer-list-table': HTMLElement;
  'packer-files-body': HTMLElement;
  'packer-tree-view': HTMLElement;
  'packer-empty-state': HTMLElement;
  'packer-drag-overlay': HTMLElement;
  'packer-workspace-delete-btn': HTMLButtonElement;
  'packer-project-name': HTMLInputElement;
  'packer-project-save-btn': HTMLButtonElement;
  'packer-meta-name': HTMLInputElement;
  'packer-meta-version': HTMLInputElement;
  'packer-meta-author': HTMLInputElement;
  'packer-meta-nexus-id': HTMLInputElement;
  'packer-meta-type': HTMLSelectElement;
  'packer-meta-desc': HTMLTextAreaElement;
  'packer-preset-vortex': HTMLInputElement;
  'packer-preset-dual': HTMLInputElement;
  'packer-format-select': HTMLSelectElement;
  'packer-build-btn': HTMLButtonElement;
}

export const packerDom = createScope<PackerDomMap>('packer');
