/**
 * PMM Internal Framework - Scoped DOM Accessor Engine
 * Proporciona acceso seguro, tipado y validado en tiempo de compilación al DOM.
 */

export type BaseDomMap = {
  [K in string]?: HTMLElement;
};

export interface ScopeAccessor<T extends object> {
  readonly scopeName: string;
  el<K extends keyof T & string>(id: K): T[K] extends HTMLElement ? T[K] : HTMLElement;
  elMaybe<K extends keyof T & string>(id: K): (T[K] extends HTMLElement ? T[K] : HTMLElement) | null;
  exists<K extends keyof T & string>(id: K): boolean;
  query<E extends HTMLElement = HTMLElement>(selector: string): E | null;
  queryAll<E extends HTMLElement = HTMLElement>(selector: string): E[];
}

/**
 * Crea un Scope tipado para un dominio/vista específica de la aplicación.
 * @param scopeName Nombre descriptivo del scope (para logs y advertencias)
 */
export function createScope<T extends object>(scopeName: string): ScopeAccessor<T> {
  return {
    scopeName,
    el<K extends keyof T & string>(id: K): any {
      const node = document.getElementById(id);
      if (!node && import.meta.env.DEV) {
        console.warn(`[PMM Framework:${scopeName}] DOM element #${id} no encontrado en la plantilla actual.`);
      }
      return node;
    },
    elMaybe<K extends keyof T & string>(id: K): any {
      return document.getElementById(id);
    },
    exists<K extends keyof T & string>(id: K): boolean {
      return document.getElementById(id) !== null;
    },
    query<E extends HTMLElement = HTMLElement>(selector: string): E | null {
      return document.querySelector<E>(selector);
    },
    queryAll<E extends HTMLElement = HTMLElement>(selector: string): E[] {
      return Array.from(document.querySelectorAll<E>(selector));
    },
  };
}
