// Console overlay debug log buffer
export let _logBuffer: string[] = [];

export function openConsoleModal(): void {
  const modal = document.getElementById('console-modal')!;
  modal.classList.add('visible');

  const closeX = document.getElementById('console-modal-close-x')!;
  const closeBtn = document.getElementById('console-modal-close')!;
  const clearBtn = document.getElementById('console-clear-btn')!;

  const close = () => {
    modal.classList.remove('visible');
  };

  closeX.onclick = close;
  closeBtn.onclick = close;

  clearBtn.onclick = () => {
    _logBuffer = [];
    const list = document.getElementById('console-logs-list');
    if (list) list.innerHTML = '';
  };

  const list = document.getElementById('console-logs-list');
  if (list) {
    list.innerHTML = _logBuffer.map(log => {
      let colorClass = '';
      if (log.includes('[ERROR]') || log.toLowerCase().includes('failed') || log.toLowerCase().includes('error')) {
        colorClass = 'log-error';
      } else if (log.includes('[WARN]')) {
        colorClass = 'log-warn';
      }
      return `<div class="console-log-item ${colorClass}">${log}</div>`;
    }).join('');
    list.scrollTop = list.scrollHeight;
  }
}

export function pushToLogBuffer(message: string): void {
  _logBuffer.push(message);
  if (_logBuffer.length > 2000) {
    _logBuffer.shift();
  }

  const list = document.getElementById('console-logs-list');
  if (list && document.getElementById('console-modal')!.classList.contains('visible')) {
    const div = document.createElement('div');
    div.className = 'console-log-item';
    if (message.includes('[ERROR]') || message.toLowerCase().includes('failed') || message.toLowerCase().includes('error')) {
      div.className += ' log-error';
    } else if (message.includes('[WARN]')) {
      div.className += ' log-warn';
    }
    div.textContent = message;
    list.appendChild(div);
    list.scrollTop = list.scrollHeight;
  }
}
