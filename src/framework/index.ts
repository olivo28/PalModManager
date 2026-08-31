/**
 * PMM Internal Framework
 * Punto de entrada único para el core del framework, reactividad y scopes del DOM.
 */

// Core Engine
export { createScope } from './core/query';
export type { BaseDomMap, ScopeAccessor } from './core/query';
export { bind, createBinderGroup } from './core/bind';
export type { BinderSpec } from './core/bind';
export { bus } from './core/bus';
export type { AppEventMap, BusHandler } from './core/bus';

// Scopes de Dominio
export { detailDom } from './scopes/detail';
export type { DetailDomMap } from './scopes/detail';

export { discoveryDom } from './scopes/discovery';
export type { DiscoveryDomMap } from './scopes/discovery';

export { settingsDom } from './scopes/settings';
export type { SettingsDomMap } from './scopes/settings';

export { editorDom } from './scopes/editor';
export type { EditorDomMap } from './scopes/editor';

export { packerDom } from './scopes/packer';
export type { PackerDomMap } from './scopes/packer';

export { libraryDom } from './scopes/library';
export type { LibraryDomMap } from './scopes/library';

export { scannerDom } from './scopes/scanner';
export type { ScannerDomMap } from './scopes/scanner';

export { installerDom } from './scopes/installer';
export type { InstallerDomMap } from './scopes/installer';

export { loadOrderDom } from './scopes/loadOrder';
export type { LoadOrderDomMap } from './scopes/loadOrder';

export { dbDom } from './scopes/db';
export type { DbDomMap } from './scopes/db';

export { dependencyDom } from './scopes/dependency';
export type { DependencyDomMap } from './scopes/dependency';

export { mainDom } from './scopes/main';
export type { MainDomMap } from './scopes/main';
