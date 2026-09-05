/**
 * PMM Internal Framework - Strongly-Typed Event Bus (Event Mesh)
 * Proporciona comunicación desacoplada y libre de dependencias circulares entre vistas y subsistemas.
 */

import type { ModInfo } from '../../types';
import type { AppState } from '../../state';

export interface AppEventMap {
  // Eventos de Mods
  'mods:refresh': void;
  'mods:loaded': ModInfo[];
  'mod:toggled': { id: string; enabled: boolean };
  'mod:installed': { id: string };
  'mod:removed': { id: string };
  'mod:renamed': { modId: string; newName: string };
  'mod:selected': ModInfo | null;
  'mod:detailsOpened': { mod: ModInfo };
  'mod:detailsClosed': void;

  // Eventos de Perfiles
  'profile:changed': { profileId: string };
  'profile:switched': { profileId: string; profileName: string };
  'profile:loaded': void;

  // Eventos de Navegación y Vistas
  'app:ready': void;
  'tab:switched': { tab: AppState['activeTab'] };
  'toast:show': { message: string; type?: 'info' | 'success' | 'warning' | 'error' };

  // Eventos de Editor
  'editor:save': void;
  'editor:saved': { filePath: string };
  'editor:fileSelected': { filePath: string };
  'editor:externalChange': { filePath: string };

  // Eventos de Discovery, Nexus & Queue
  'discovery:opened': { modId: number };
  'nexus:authChanged': { loggedIn: boolean; username?: string };
  'queue:speed': { downloadId: string; speedBytesPerSec: number; etaSeconds?: number };

  // Eventos de Backups & Operaciones
  'backup:created': { path: string; sizeBytes?: number };
  'backup:restored': { path: string };
  'workshop:updated': { count: number };
  'project:packed': { outputPath: string; format: string; modName: string };
  'hotkey:updated': { modId: string; newHotkey: string };
}

export type BusHandler<T> = (payload: T) => void;

class EventBus {
  private handlers = new Map<string, BusHandler<any>[]>();

  /**
   * Suscribe un callback a un evento específico.
   * Retorna una función de desuscripción (cleanup).
   */
  on<K extends keyof AppEventMap>(event: K, handler: BusHandler<AppEventMap[K]>): () => void {
    const list = this.handlers.get(event) ?? [];
    list.push(handler);
    this.handlers.set(event, list);
    return () => {
      const currentList = this.handlers.get(event);
      if (!currentList) return;
      const idx = currentList.indexOf(handler);
      if (idx !== -1) {
        currentList.splice(idx, 1);
      }
    };
  }

  /**
   * Emite un evento a todos los suscriptores registrados.
   */
  emit<K extends keyof AppEventMap>(event: K, payload: AppEventMap[K]): void {
    const list = this.handlers.get(event);
    if (!list || list.length === 0) return;
    for (let i = 0; i < list.length; i++) {
      try {
        list[i](payload);
      } catch (err) {
        console.error(`[PMM Bus] Error procesando evento '${event}':`, err);
      }
    }
  }

  /**
   * Limpia todos los suscriptores (útil para tests o reinicios).
   */
  clear(): void {
    this.handlers.clear();
  }
}

export const bus = new EventBus();
