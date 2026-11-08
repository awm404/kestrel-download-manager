// Kestrel Companion - Popup Script

const DEFAULT_EXTENSIONS = [
  'mkv', 'mp4', 'avi', 'mov', 'wmv', 'flv', 'webm', 'ts', 'm4v',
  'zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'xz', 'iso', 'bin',
  'exe', 'msi', 'apk', 'dmg', 'pkg', 'deb', 'rpm'
];

const DEFAULT_RPC_URL = 'http://127.0.0.1:6800/jsonrpc';
const SECRET_URL = 'http://127.0.0.1:6801/kestrel-rpc-secret';

function normalizeSecret(value) {
  let secret = (value || '').trim();
  if (secret.toLowerCase().startsWith('token:')) secret = secret.slice(6).trim();
  return secret;
}

document.addEventListener('DOMContentLoaded', async () => {
  const badge = document.getElementById('engineStatusBadge');
  const dot = document.getElementById('engineDot');
  const text = document.getElementById('engineText');
  const toggle = document.getElementById('autoInterceptToggle');
  const extInput = document.getElementById('extensionsInput');
  const secretInput = document.getElementById('rpcSecretInput');
  const manualInput = document.getElementById('manualUrlInput');
  const sendForm = document.getElementById('sendForm');
  const sendBtn = document.getElementById('sendBtn');
  const sendStatus = document.getElementById('sendStatus');

  const settings = await chrome.storage.local.get({
    enabled: true,
    interceptExtensions: DEFAULT_EXTENSIONS,
    rpcUrl: DEFAULT_RPC_URL,
    rpcSecret: ''
  });
  toggle.checked = settings.enabled;
  extInput.value = settings.interceptExtensions.join(', ');
  secretInput.value = settings.rpcSecret;

  function persistRpcSecret() {
    settings.rpcSecret = normalizeSecret(secretInput.value);
    if (secretInput.value !== settings.rpcSecret) secretInput.value = settings.rpcSecret;
    chrome.storage.local.set({ rpcSecret: settings.rpcSecret });
    checkEngine();
  }
  secretInput.addEventListener('input', persistRpcSecret);
  secretInput.addEventListener('change', persistRpcSecret);

  toggle.addEventListener('change', () => {
    chrome.storage.local.set({ enabled: toggle.checked });
  });

  extInput.addEventListener('change', () => {
    const list = extInput.value.split(',').map(s => s.trim().replace(/^\./, '')).filter(Boolean);
    chrome.storage.local.set({ interceptExtensions: list });
  });

  async function rpc(method, params = []) {
    const fullParams = settings.rpcSecret ? [`token:${settings.rpcSecret}`, ...params] : params;
    const res = await fetch(settings.rpcUrl || DEFAULT_RPC_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ jsonrpc: '2.0', id: 'kestrel_popup_' + Date.now(), method, params: fullParams })
    });
    return res.json();
  }

  function setEngineState(online, offlineLabel = 'Offline') {
    const label = online ? 'Online' : offlineLabel;
    badge.classList.toggle('online', online);
    badge.classList.toggle('offline', !online);
    dot.className = online ? 'status-dot online' : 'status-dot offline';
    text.textContent = label;
    badge.title = `Engine: ${label}`;
  }

  function setSendStatus(message, kind) {
    sendStatus.textContent = message;
    sendStatus.className = `send-status${kind ? ' ' + kind : ''}`;
    if (kind === 'error') {
      manualInput.setAttribute('aria-invalid', 'true');
    } else {
      manualInput.removeAttribute('aria-invalid');
    }
  }

  async function pullSecretFromKestrel() {
    const res = await fetch(SECRET_URL, { cache: 'no-store' });
    if (!res.ok) return '';
    return normalizeSecret(await res.text());
  }

  async function checkEngine() {
    try {
      let data = await rpc('aria2.getVersion');
      if (!data || !data.result) {
        const pulled = await pullSecretFromKestrel().catch(() => '');
        if (pulled && pulled !== settings.rpcSecret) {
          settings.rpcSecret = pulled;
          secretInput.value = pulled;
          await chrome.storage.local.set({ rpcSecret: pulled });
          data = await rpc('aria2.getVersion');
        }
      }
      const online = !!(data && data.result);
      const unauthorized = !!(data && data.error && data.error.message === 'Unauthorized');
      const offlineLabel = unauthorized ? (settings.rpcSecret ? 'Wrong secret' : 'Secret required') : 'Offline';
      setEngineState(online, offlineLabel);
    } catch (e) {
      setEngineState(false);
    }
  }
  checkEngine();

  manualInput.addEventListener('input', () => {
    if (sendStatus.classList.contains('error')) setSendStatus('', '');
  });

  sendForm.addEventListener('submit', async (e) => {
    e.preventDefault();
    const url = manualInput.value.trim();
    if (!url) {
      setSendStatus('Paste a download link first.', 'error');
      manualInput.focus();
      return;
    }

    // Get current tab URL as referer
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    const referer = tab && tab.url ? tab.url : url;

    // Only the cookies the browser itself would send to this exact URL.
    let cookieString = '';
    try {
      const cookies = await chrome.cookies.getAll({ url });
      if (cookies && cookies.length > 0) {
        cookieString = cookies.map(c => `${c.name}=${c.value}`).join('; ');
      }
    } catch (err) {}

    const options = {};
    if (referer) options.referer = referer;
    options['user-agent'] = navigator.userAgent;
    if (cookieString) options.header = [`Cookie: ${cookieString}`];

    sendBtn.disabled = true;
    setSendStatus('Sending...', '');
    try {
      const data = await rpc('aria2.addUri', [[url], options]);
      if (data.result) {
        manualInput.value = '';
        setSendStatus('Sent to Kestrel.', 'ok');
        setEngineState(true);
      } else {
        if (data.error && data.error.message === 'Unauthorized') {
          setSendStatus('Kestrel rejected the RPC secret. Copy it again from Kestrel Preferences.', 'error');
          secretInput.focus();
          return;
        }
        const reason = data.error && data.error.message ? ` (${data.error.message})` : '';
        setSendStatus(`Kestrel could not add this link${reason}.`, 'error');
        manualInput.focus();
      }
    } catch (err) {
      setEngineState(false);
      setSendStatus('Could not reach Kestrel. Make sure the app is running.', 'error');
      manualInput.focus();
    } finally {
      sendBtn.disabled = false;
    }
  });
});
