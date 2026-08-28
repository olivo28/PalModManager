/**
 * PMM Internal Framework - Declarative State-to-DOM Binder
 * Permite proyectar el estado de la aplicación hacia el DOM de forma reactiva y sin Virtual DOM.
 */

import type { ScopeAccessor } from './query';

export type BinderSpec<TState> = {
  text?: (state: TState) => string | number;
  html?: (state: TState) => string;
  className?: (state: TState) => string;
  visible?: (state: TState) => boolean;
  disabled?: (state: TState) => boolean;
  value?: (state: TState) => string;
  attr?: Record<string, (state: TState) => string | null>;
};

/**
 * Declara una regla de binding unidireccional (State -> Elemento DOM).
 * Retorna una función aplicadora que actualiza el nodo cuando el estado cambia.
 */
export function bind<TMap extends object, TState, K extends keyof TMap & string>(
  scope: ScopeAccessor<TMap>,
  id: K,
  spec: BinderSpec<TState>
): (state: TState) => void {
  return (state: TState) => {
    const node = scope.elMaybe(id);
    if (!node) return;

    if (spec.text !== undefined) {
      node.textContent = String(spec.text(state));
    }
    if (spec.html !== undefined) {
      node.innerHTML = spec.html(state);
    }
    if (spec.className !== undefined) {
      node.className = spec.className(state);
    }
    if (spec.visible !== undefined) {
      node.style.display = spec.visible(state) ? '' : 'none';
    }
    if (spec.disabled !== undefined && 'disabled' in node) {
      (node as unknown as { disabled: boolean }).disabled = spec.disabled(state);
    }
    if (spec.value !== undefined && 'value' in node) {
      (node as unknown as { value: string }).value = spec.value(state);
    }
    if (spec.attr) {
      for (const [attrName, fn] of Object.entries(spec.attr)) {
        const val = fn(state);
        if (val === null) {
          node.removeAttribute(attrName);
        } else {
          node.setAttribute(attrName, val);
        }
      }
    }
  };
}

/**
 * Agrupa múltiples bindings en un único despachador reactivo.
 */
export function createBinderGroup<TState>(
  ...binders: Array<(state: TState) => void>
): (state: TState) => void {
  return (state: TState) => {
    for (let i = 0; i < binders.length; i++) {
      binders[i](state);
    }
  };
}
