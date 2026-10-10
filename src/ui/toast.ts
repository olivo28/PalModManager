import { mainDom } from '../framework';
import { getIconSvg } from './icons/mod';

function getToastContainer(): HTMLElement {
  let container = mainDom.elMaybe('toast-container');
  if (!container) {
    container = document.createElement('div');
    container.id = 'toast-container';
    document.body.appendChild(container);
  }
  return container;
}

export function showToast(message: string, type: 'success' | 'error' | 'info' | 'warning' = 'info'): void {
  const container = getToastContainer();
  const toast = document.createElement('div');
  toast.className = `toast toast-${type}`;

  const iconName = type === 'success' ? 'checkCircle' : type === 'error' ? 'xCircle' : type === 'warning' ? 'alertTriangle' : 'info';
  const iconSpan = document.createElement('span');
  iconSpan.className = 'toast-icon';
  iconSpan.innerHTML = getIconSvg(iconName, 'pmm-icon-sm');

  const textSpan = document.createElement('span');
  textSpan.className = 'toast-text';
  textSpan.textContent = message;

  toast.appendChild(iconSpan);
  toast.appendChild(textSpan);
  container.appendChild(toast);
  
  // Usar doble requestAnimationFrame para asegurar que el navegador detecte el cambio de clase para la transición
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      toast.classList.add('show');
    });
  });

  setTimeout(() => {
    toast.classList.remove('show');
    setTimeout(() => toast.remove(), 300);
  }, 3000);
}
