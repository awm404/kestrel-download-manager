// Kestrel Companion - Background Service Worker (Manifest V3)

const SECRET_URL = 'http://127.0.0.1:6801/kestrel-rpc-secret';

function normalizeSecret(value) {
  let secret = (value || '').trim();
  if (secret.toLowerCase().startsWith('token:')) secret = secret.slice(6).trim();
  return secret;
}

async function pullSecretFromKestrel() {
  const res = await fetch(SECRET_URL, { cache: 'no-store' });
  if (!res.ok) return '';
  return normalizeSecret(await res.text());
}

const DEFAULT_SETTINGS = {
  enabled: true,
  rpcUrl: 'http://127.0.0.1:6800/jsonrpc',
  rpcSecret: '',
  interceptExtensions: [
    'mkv', 'mp4', 'avi', 'mov', 'wmv', 'flv', 'webm', 'ts', 'm4v',
    'zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'xz', 'iso', 'bin',
    'exe', 'msi', 'apk', 'dmg', 'pkg', 'deb', 'rpm'
  ]
};

async function getSettings() {
  return new Promise((resolve) => {
    chrome.storage.local.get(DEFAULT_SETTINGS, (items) => {
      resolve(items);
    });
  });
}

// Setup Context Menus
function setupContextMenus() {
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({
      id: 'kestrel_download_link',
      title: 'Download with Kestrel',
      contexts: ['link', 'video', 'audio', 'image']
    });
    chrome.contextMenus.create({
      id: 'kestrel_download_page',
      title: 'Download Current Page Media with Kestrel',
      contexts: ['page']
    });
  });
}

chrome.runtime.onInstalled.addListener(setupContextMenus);
chrome.runtime.onStartup.addListener(setupContextMenus);
setupContextMenus();

// Handle Context Menu Clicks
chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  let targetUrl = info.linkUrl || info.srcUrl;
  if (info.menuItemId === 'kestrel_download_page' && tab) {
    targetUrl = tab.url;
  }
  if (!targetUrl) return;

  const referer = tab ? tab.url : targetUrl;
  await sendUrlToKestrel(targetUrl, referer, null);
});

// Intercept Browser Downloads (Non-Intrusive)
chrome.downloads.onDeterminingFilename.addListener((downloadItem, suggest) => {
  handleDownloadInterception(downloadItem, suggest);
  // Return true to indicate we will call suggest() asynchronously if not intercepted
  return true;
});

async function handleDownloadInterception(downloadItem, suggest) {
  const settings = await getSettings();

  // If auto-intercept is disabled, let browser handle it
  if (!settings.enabled) {
    suggest();
    return;
  }

  // Prioritize final redirected URL over initial request URL (CRUCIAL for mirror redirects like TechPowerUp)
  const actualUrl = downloadItem.finalUrl || downloadItem.url || '';
  const initialUrl = downloadItem.url || '';

  // Determine file extension from suggested filename or actual target URL
  const filename = downloadItem.filename || '';
  const ext = getFileExtension(filename) || getFileExtension(actualUrl) || getFileExtension(initialUrl);

  // Non-intrusive filter: only intercept whitelist extensions
  const shouldIntercept = settings.interceptExtensions.some(e => e.toLowerCase() === ext.toLowerCase());

  if (!shouldIntercept) {
    suggest();
    return;
  }

  // Safety check: If the URL is actually a landing webpage or POST endpoint without direct binary file path,
  // do NOT intercept! Let the browser finish downloading it natively with its active HTTP session!
  const isWebpage = actualUrl.endsWith('/') || actualUrl.endsWith('.html') || actualUrl.endsWith('.htm') || actualUrl.endsWith('.php');
  if (isWebpage && !actualUrl.includes('/files/') && !actualUrl.includes('/dl.')) {
    console.log('[Kestrel Companion] Skipping interception: URL is a webpage, letting browser download directly:', actualUrl);
    suggest();
    return;
  }

  // Attempt to dispatch to Kestrel with actual redirected URL
  const referer = downloadItem.referrer || initialUrl;
  const cleanName = filename ? filename.split('\\').pop().split('/').pop() : null;

  const success = await sendUrlToKestrel(actualUrl, referer, cleanName);

  if (success) {
    // Cancel browser download
    try {
      chrome.downloads.cancel(downloadItem.id, () => {
        chrome.downloads.erase({ id: downloadItem.id });
      });
    } catch (e) {
      console.warn('Could not cancel browser download:', e);
    }
  } else {
    // If Kestrel was offline, let the browser download normally as fallback!
    suggest();
  }
}

function getFileExtension(path) {
  if (!path) return '';
  const clean = path.split('?')[0].split('#')[0];
  const lastDot = clean.lastIndexOf('.');
  if (lastDot > 0 && lastDot < clean.length - 1) {
    return clean.substring(lastDot + 1).toLowerCase();
  }
  return '';
}

// Send Download to Kestrel via Aria2 JSON-RPC
async function sendUrlToKestrel(url, referer, customFilename) {
  const settings = await getSettings();

  // Forward only the cookies the browser itself would send to this URL. This already includes
  // parent-domain cookies (e.g. `.example.com` for dl.example.com); cookies from other sites,
  // such as the referring page, must never be sent to the download host.
  let cookieString = '';
  try {
    const cookies = await chrome.cookies.getAll({ url: url });
    if (cookies && cookies.length > 0) {
      cookieString = cookies.map(c => `${c.name}=${c.value}`).join('; ');
    }
  } catch (err) {
    console.warn('Could not extract cookies for URL:', url, err);
  }

  const options = {};
  if (customFilename) {
    options.out = customFilename;
  }
  if (referer) {
    options.referer = referer;
  }
  options['user-agent'] = navigator.userAgent;

  if (cookieString) {
    options.header = [`Cookie: ${cookieString}`];
  }

  const payload = {
    jsonrpc: '2.0',
    id: 'kestrel_ext_' + Date.now(),
    method: 'aria2.addUri',
    params: settings.rpcSecret ? [`token:${settings.rpcSecret}`, [url], options] : [[url], options]
  };

  try {
    const res = await fetch(settings.rpcUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });

    const data = await res.json();
    if (data.result) {
      const displayName = customFilename || (url.split('?')[0].split('/').pop()) || 'File';
      showNotification('🦅 Sent to Kestrel', `${displayName}\nDownloading with authenticated browser session.`);
      return true;
    }
    if (data.error && data.error.message === 'Unauthorized' && !sendUrlToKestrel.retried) {
      const pulled = await pullSecretFromKestrel().catch(() => '');
      if (pulled && pulled !== settings.rpcSecret) {
        await chrome.storage.local.set({ rpcSecret: pulled });
        sendUrlToKestrel.retried = true;
        try {
          return await sendUrlToKestrel(url, referer, customFilename);
        } finally {
          sendUrlToKestrel.retried = false;
        }
      }
      showNotification('Kestrel needs the RPC secret', 'Open Kestrel, then reload Kestrel Companion at chrome://extensions. Your browser downloaded this file instead.');
      return false;
    }
    console.error('Aria2 RPC error response:', data.error);
    return false;
  } catch (err) {
    console.warn('Could not connect to Kestrel Aria2 daemon:', err);
    return false;
  }
}

function showNotification(title, message) {
  try {
    chrome.notifications.create({
      type: 'basic',
      iconUrl: 'icons/icon-128.png',
      title: title,
      message: message
    });
  } catch (e) {
    console.log(`[Notification] ${title}: ${message}`);
  }
}
