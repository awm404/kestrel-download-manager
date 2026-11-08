
    // Universal Tauri Invoke Helper (v1 / v2 compatible)
    function tauriInvoke(cmd, args = {}) {
      if (window.__TAURI__) {
        if (window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
          return window.__TAURI__.core.invoke(cmd, args);
        }
        if (typeof window.__TAURI__.invoke === 'function') {
          return window.__TAURI__.invoke(cmd, args);
        }
      }
      return Promise.resolve(null);
    }

    // App State
    const state = {
      rpcUrl: 'http://127.0.0.1:6800/jsonrpc',
      secret: '',
      activeFilter: 'all',
      search: '',
      tasks: [],
      expandedGids: new Set(),
      pendingDelete: null,
      defaultDir: localStorage.getItem('aria2_default_dir') || 'C:/Users/Alief/Downloads',
      minimizeToTray: localStorage.getItem('aria2_min_tray') !== 'false',
      torrentBase64: null,
      pollTimer: null,
      clickTimer: null
    };

    window.addEventListener('DOMContentLoaded', () => {
      initAppConfig();
      startSync();
      initKeyboardShortcuts();
      initContextMenuAndHotkeys();
    });

    function initKeyboardShortcuts() {
      window.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
          closeAddModal();
          closeDeleteModal();
          closeSettingsModal();
        } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n') {
          e.preventDefault();
          openAddModal();
        } else if ((e.ctrlKey || e.metaKey) && e.key === ',') {
          e.preventDefault();
          openSettingsModal();
        } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
          e.preventDefault();
          document.getElementById('searchInput').focus();
        }
      });
    }

    async function initAppConfig() {
      const opts = await callAria2('aria2.getGlobalOption');
      if (opts && opts.dir) {
        state.defaultDir = opts.dir;
      }

      document.getElementById('modalDir').value = state.defaultDir;
      document.getElementById('settingsStorageDir').value = state.defaultDir;
      document.getElementById('storageTargetText').textContent = state.defaultDir;
      document.getElementById('storageTargetText').title = state.defaultDir;
      document.getElementById('minimizeToTrayCheckbox').checked = state.minimizeToTray;

      if (opts && opts['max-concurrent-downloads']) {
        document.getElementById('settingsMaxConcurrent').value = opts['max-concurrent-downloads'];
      }
      if (opts && opts['max-overall-download-limit']) {
        document.getElementById('settingsMaxDlLimit').value = opts['max-overall-download-limit'];
      }

      tauriInvoke('set_minimize_to_tray', { enabled: state.minimizeToTray });
    }

    async function callAria2(method, params = []) {
      const payload = {
        jsonrpc: '2.0',
        id: 'req_' + Date.now(),
        method: method,
        params: state.secret ? [`token:${state.secret}`, ...params] : params
      };

      const start = performance.now();
      try {
        const res = await fetch(state.rpcUrl, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(payload)
        });
        const latency = Math.max(1, Math.round(performance.now() - start));
        document.getElementById('rpcLatency').textContent = `${latency}ms`;
        document.getElementById('daemonDot').style.background = 'var(--success)';
        const data = await res.json();
        if (data.error) throw new Error(data.error.message);
        return data.result;
      } catch (err) {
        document.getElementById('daemonDot').style.background = 'var(--danger)';
        return null;
      }
    }

    function startSync() {
      fetchTasks();
      if (state.pollTimer) clearInterval(state.pollTimer);
      state.pollTimer = setInterval(fetchTasks, 1000);
    }

    async function fetchTasks() {
      const [active, waiting, stopped, globalStat] = await Promise.all([
        callAria2('aria2.tellActive'),
        callAria2('aria2.tellWaiting', [0, 100]),
        callAria2('aria2.tellStopped', [0, 100]),
        callAria2('aria2.getGlobalStat')
      ]);

      if (globalStat) {
        const dl = parseInt(globalStat.downloadSpeed || '0');
        const up = parseInt(globalStat.uploadSpeed || '0');
        const dlFmt = formatSpeed(dl);
        const upFmt = formatSpeed(up);
        document.getElementById('metricDl').textContent = dlFmt.val;
        document.getElementById('metricDlUnit').textContent = dlFmt.unit;
        document.getElementById('metricUp').textContent = upFmt.val;
        document.getElementById('metricUpUnit').textContent = upFmt.unit;
        document.getElementById('metricActiveStreams').textContent = `${globalStat.numActive || 0} active transfers`;
        document.getElementById('metricDone').textContent = globalStat.numStopped || 0;
      }

      const all = [...(active || []), ...(waiting || []), ...(stopped || [])];
      state.tasks = all;

      document.getElementById('countActive').textContent = (active || []).length;
      document.getElementById('countWaiting').textContent = (waiting || []).length;
      document.getElementById('countStopped').textContent = (stopped || []).length;
      document.getElementById('countAll').textContent = all.length;

      renderTaskRows();
    }

    // HIGH-PERFORMANCE IN-PLACE DOM UPDATER (ZERO JANK, ZERO FLICKER)
    function renderTaskRows() {
      const container = document.getElementById('taskListContainer');
      let filtered = state.tasks;

      if (state.activeFilter === 'active') {
        filtered = filtered.filter(t => t.status === 'active');
      } else if (state.activeFilter === 'waiting') {
        filtered = filtered.filter(t => t.status === 'waiting' || t.status === 'paused');
      } else if (state.activeFilter === 'stopped') {
        filtered = filtered.filter(t => t.status === 'complete' || t.status === 'error' || t.status === 'removed');
      }

      if (state.search) {
        const q = state.search.toLowerCase();
        filtered = filtered.filter(t => getFileName(t).toLowerCase().includes(q) || t.gid.includes(q));
      }

      const zeroEl = document.getElementById('zeroStateContainer');
      if (filtered.length === 0) {
        if (!zeroEl) {
          const z = document.createElement('div');
          z.id = 'zeroStateContainer';
          z.className = 'zero-state';
          z.innerHTML = `
            <div class="zero-icon">
              <svg width="24" height="24" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M7 10l5 5 5-5M12 15V3"/></svg>
            </div>
            <div style="font-size: 15px; font-weight: 600; color: #fff;">No downloads in this view</div>
            <div style="font-size: 13px; color: var(--text-muted); max-width: 340px;">Click "New Download" to paste a direct link or open a .torrent file.</div>
          `;
          container.appendChild(z);
        }
        // Remove old rows
        Array.from(container.children).forEach(child => {
          if (child.id !== 'zeroStateContainer') child.remove();
        });
        return;
      } else {
        if (zeroEl) zeroEl.remove();
      }

      const currentGids = new Set(filtered.map(t => t.gid));

      // Remove obsolete rows
      Array.from(container.children).forEach(child => {
        const gid = child.getAttribute('data-gid');
        if (gid && !currentGids.has(gid)) {
          child.remove();
        }
      });

      // Update or mount rows
      filtered.forEach((t) => {
        let row = container.querySelector(`[data-gid="${t.gid}"]`);
        if (!row) {
          row = createRowElement(t);
          container.appendChild(row);
        }
        updateRowInPlace(row, t);
      });
    }

    // Create Row DOM Once
    function createRowElement(t) {
      const row = document.createElement('div');
      row.className = 'task-row';
      row.setAttribute('data-gid', t.gid);

      // Debounced click to distinguish single-click (toggle drawer) from double-click (launch app)
      row.onclick = (e) => onRowClick(t.gid, e);
      row.ondblclick = (e) => onRowDblClick(t.gid, e);
      row.oncontextmenu = (e) => {
        e.preventDefault();
        e.stopPropagation();
        showTaskContextMenu(e.clientX, e.clientY, t.gid);
      };

      row.innerHTML = `
        <div class="task-top">
          <div class="task-icon">
            <svg width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path d="M13 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V9z"/><polyline points="13 2 13 9 20 9"/></svg>
          </div>
          <div class="task-meta">
            <div class="task-title-line">
              <div class="task-name" title=""></div>
              <div class="task-badge-container"><span class="status-badge"></span></div>
            </div>
            <div class="task-info">
              <span class="info-bytes">0 B of 0 B</span>
              <span>•</span>
              <span class="info-speed" style="font-weight: 600;">0%</span>
              <span class="info-eta"></span>
              <span class="info-conns"></span>
            </div>
          </div>
          <div class="task-actions" onclick="event.stopPropagation()">
            <button class="btn-action-open btn-open" title="Open downloaded file (or double-click card)" style="display: none;">
              <svg width="13" height="13" fill="none" stroke="currentColor" stroke-width="2.2" viewBox="0 0 24 24"><path d="M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6M15 3h6v6M10 14L21 3"/></svg>
              <span>Open</span>
            </button>
            <button class="icon-btn btn-pause" title="Pause" style="display: none;">
              <svg width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/></svg>
            </button>
            <button class="icon-btn btn-resume" title="Resume" style="display: none;">
              <svg width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><polygon points="5 3 19 12 5 21 5 3"/></svg>
            </button>
            <button class="icon-btn btn-folder" title="Show in File Explorer">
              <svg width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/></svg>
            </button>
            <button class="icon-btn danger btn-delete" title="Delete Transfer">
              <svg width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/></svg>
            </button>
          </div>
        </div>

        <div class="progress-bar-bg">
          <div class="progress-bar-fill"></div>
        </div>

        <div class="task-details">
          <div class="detail-card">
            <span class="detail-label">File Path / Target (Double-click card to open)</span>
            <span class="detail-val detail-path"></span>
          </div>
          <div class="detail-card">
            <span class="detail-label">Source URL</span>
            <span class="detail-val detail-source"></span>
          </div>
        </div>
      `;

      // Attach button events permanently
      row.querySelector('.btn-open').onclick = (e) => { e.stopPropagation(); handleOpenFile(t.gid); };
      row.querySelector('.btn-pause').onclick = (e) => { e.stopPropagation(); pauseJob(t.gid); };
      row.querySelector('.btn-resume').onclick = (e) => { e.stopPropagation(); resumeJob(t.gid); };
      row.querySelector('.btn-folder').onclick = (e) => { e.stopPropagation(); revealFolder(t.gid); };
      row.querySelector('.btn-delete').onclick = (e) => { e.stopPropagation(); requestDelete(t.gid); };

      return row;
    }

    // Granular in-place update (preserves hover, focus, and clicks)
    function updateRowInPlace(row, t) {
      const name = getFileName(t);
      const total = parseInt(t.totalLength || '0');
      const comp = parseInt(t.completedLength || '0');
      const speed = parseInt(t.downloadSpeed || '0');
      const percent = total > 0 ? Math.min(100, Math.round((comp / total) * 100)) : (t.status === 'complete' ? 100 : 0);

      const isAct = t.status === 'active';
      const isPau = t.status === 'paused' || t.status === 'waiting';
      const isDone = t.status === 'complete';

      let eta = '';
      if (isAct && speed > 0 && total > comp) {
        eta = formatEta(Math.round((total - comp) / speed));
      }

      const filePath = getFilePath(t);
      const uri = (t.files && t.files[0] && t.files[0].uris && t.files[0].uris[0]) ? t.files[0].uris[0].uri : 'Direct stream';

      // 1. Text & titles
      const nameEl = row.querySelector('.task-name');
      if (nameEl.textContent !== name) {
        nameEl.textContent = name;
        nameEl.title = name;
      }

      const badge = row.querySelector('.status-badge');
      badge.className = `status-badge badge-${t.status}`;
      badge.textContent = renderStatusText(t.status);

      row.querySelector('.info-bytes').textContent = `${formatBytes(comp)} of ${formatBytes(total)}`;
      
      const speedEl = row.querySelector('.info-speed');
      speedEl.style.color = isAct ? 'var(--primary)' : 'inherit';
      speedEl.textContent = isAct ? `${formatSpeed(speed).val} ${formatSpeed(speed).unit}` : `${percent}%`;

      const etaEl = row.querySelector('.info-eta');
      etaEl.textContent = eta ? `• ${eta}` : '';

      const connsEl = row.querySelector('.info-conns');
      connsEl.textContent = t.connections ? `• ${t.connections} conns` : '';

      // 2. Progress bar
      const fill = row.querySelector('.progress-bar-fill');
      fill.style.width = `${percent}%`;
      fill.className = `progress-bar-fill ${t.status}`;

      // 3. Buttons visibility
      row.querySelector('.btn-open').style.display = isDone ? 'flex' : 'none';
      row.querySelector('.btn-pause').style.display = isAct ? 'flex' : 'none';
      row.querySelector('.btn-resume').style.display = isPau ? 'flex' : 'none';

      // 4. Drawer & row state
      const pathEl = row.querySelector('.detail-path');
      pathEl.textContent = filePath;
      pathEl.title = filePath;

      const sourceEl = row.querySelector('.detail-source');
      sourceEl.textContent = uri;
      sourceEl.title = uri;

      row.classList.toggle('expanded', state.expandedGids.has(t.gid));
      row.classList.toggle('is-complete', isDone);
      row.title = isDone ? 'Double-click to open application / file' : 'Click to view details';
    }

    // Single-click vs Double-click arbitration
    function onRowClick(gid, e) {
      if (e.target.closest('.task-actions')) return;
      if (state.clickTimer) {
        clearTimeout(state.clickTimer);
        state.clickTimer = null;
        return; // Double-click detected, cancel single click!
      }
      state.clickTimer = setTimeout(() => {
        state.clickTimer = null;
        toggleDrawer(gid);
      }, 220);
    }

    function onRowDblClick(gid, e) {
      if (e.target.closest('.task-actions')) return;
      if (state.clickTimer) {
        clearTimeout(state.clickTimer);
        state.clickTimer = null;
      }
      handleOpenFile(gid);
    }

    function toggleDrawer(gid) {
      if (state.expandedGids.has(gid)) {
        state.expandedGids.delete(gid);
      } else {
        state.expandedGids.add(gid);
      }
      const row = document.querySelector(`[data-gid="${gid}"]`);
      if (row) {
        row.classList.toggle('expanded', state.expandedGids.has(gid));
      }
    }

    // Launch downloaded app/file
    async function handleOpenFile(gid) {
      const t = state.tasks.find(x => x.gid === gid);
      if (!t) return;
      const name = getFileName(t);
      const filePath = getFilePath(t);

      if (t.status === 'complete') {
        const res = await tauriInvoke('open_downloaded_file', { path: filePath });
        if (res === true) {
          showToast(`Opening ${name}...`, 'success');
        } else if (res === false) {
          // Explicit clear notice that the file has been moved, NO unwanted Explorer popups
          showToast('File has been moved or deleted and cannot be opened.', 'warning');
          markFileAsMoved(gid);
        } else {
          showToast(`File location: ${filePath}`, 'info');
        }
      } else if (t.status === 'active') {
        showToast(`"${name}" is still downloading (${getPercent(t)}%)`, 'info');
      } else {
        showToast(`"${name}" is paused or queued`, 'info');
      }
    }

    function markFileAsMoved(gid) {
      const row = document.querySelector(`[data-gid="${gid}"]`);
      if (row) {
        const btn = row.querySelector('.btn-open');
        if (btn) {
          btn.classList.add('missing');
          btn.title = 'File has been moved or deleted from disk';
        }
      }
    }

    function getPercent(t) {
      const total = parseInt(t.totalLength || '0');
      const comp = parseInt(t.completedLength || '0');
      return total > 0 ? Math.min(100, Math.round((comp / total) * 100)) : 0;
    }

    // Actions
    async function pauseJob(gid) {
      await callAria2('aria2.pause', [gid]);
      fetchTasks();
    }
    async function resumeJob(gid) {
      await callAria2('aria2.unpause', [gid]);
      fetchTasks();
    }

    // Delete Flow with Disk File Option
    function requestDelete(gid) {
      const t = state.tasks.find(x => x.gid === gid);
      if (!t) return;
      const name = getFileName(t);
      const filePath = getFilePath(t);
      const isDone = t.status === 'complete' || t.status === 'error' || t.status === 'removed';

      state.pendingDelete = { gid, isDone, filePath, name };
      document.getElementById('deletePromptText').textContent = `Are you sure you want to remove "${name}" from Aria2?`;
      document.getElementById('deleteFilePathHint').textContent = filePath;
      document.getElementById('alsoDeleteFileCheckbox').checked = false;
      document.getElementById('deleteModal').classList.add('open');
    }

    function closeDeleteModal() {
      document.getElementById('deleteModal').classList.remove('open');
      state.pendingDelete = null;
    }

    async function confirmDelete() {
      if (!state.pendingDelete) return;
      const { gid, isDone, filePath, name } = state.pendingDelete;
      const alsoDelete = document.getElementById('alsoDeleteFileCheckbox').checked;

      try {
        if (isDone) {
          await callAria2('aria2.removeDownloadResult', [gid]);
        } else {
          await callAria2('aria2.remove', [gid]);
        }
      } catch (err) {
        console.warn('RPC remove error:', err);
      }

      state.expandedGids.delete(gid);

      if (alsoDelete && filePath) {
        try {
          await tauriInvoke('delete_download_file', { path: filePath });
          showToast(`Deleted ${name} from disk`, 'warning');
        } catch(err) {
          console.warn('File delete error:', err);
        }
      } else {
        showToast(`Removed ${name} from transfer list`, 'info');
      }

      closeDeleteModal();
      fetchTasks();
    }

    async function pauseAllTransfers() {
      await callAria2('aria2.pauseAll');
      showToast('All active transfers paused', 'info');
      fetchTasks();
    }
    async function resumeAllTransfers() {
      await callAria2('aria2.unpauseAll');
      showToast('All transfers resumed', 'success');
      fetchTasks();
    }
    async function clearDoneTransfers() {
      await callAria2('aria2.purgeDownloadResult');
      showToast('Cleared completed items', 'info');
      fetchTasks();
    }

    function revealFolder(gid) {
      const t = state.tasks.find(x => x.gid === gid);
      const filePath = t ? getFilePath(t) : state.defaultDir;
      if (window.__TAURI__) {
        tauriInvoke('open_in_explorer', { path: filePath });
      } else {
        alert(`File is located at:\n${filePath}`);
      }
    }

    // Modals
    function openAddModal() {
      document.getElementById('modalDir').value = state.defaultDir;
      document.getElementById('addModal').classList.add('open');
      document.getElementById('modalUrls').focus();
    }
    function closeAddModal() {
      document.getElementById('addModal').classList.remove('open');
      state.torrentBase64 = null;
      document.getElementById('dropzoneLabel').textContent = 'Or click to select a .torrent file';
    }

    function onModalOverlayClick(e, modalId) {
      if (e.target.id === modalId) {
        if (modalId === 'addModal') closeAddModal();
        if (modalId === 'deleteModal') closeDeleteModal();
        if (modalId === 'settingsModal') closeSettingsModal();
      }
    }

    function onTorrentSelected(e) {
      if (e.target.files.length > 0) {
        const file = e.target.files[0];
        const reader = new FileReader();
        reader.onload = (ev) => {
          state.torrentBase64 = ev.target.result.split(',')[1];
          document.getElementById('dropzoneLabel').textContent = `Loaded: ${file.name}`;
        };
        reader.readAsDataURL(file);
      }
    }

    async function submitNewTask() {
      const urls = document.getElementById('modalUrls').value.trim();
      const split = document.getElementById('modalSplit').value || '16';
      const dir = document.getElementById('modalDir').value.trim() || state.defaultDir;

      const options = {
        'split': String(split),
        'max-connection-per-server': String(split),
        'dir': dir
      };

      if (state.torrentBase64) {
        await callAria2('aria2.addTorrent', [state.torrentBase64, [], options]);
        showToast('Torrent download started', 'success');
        closeAddModal();
        fetchTasks();
        return;
      }

      if (!urls) {
        showToast('Please enter at least one URL or choose a .torrent file', 'warning');
        return;
      }

      const list = urls.split('\n').map(u => u.trim()).filter(Boolean);
      for (const u of list) {
        await callAria2('aria2.addUri', [[u], options]);
      }

      showToast(`Added ${list.length} download(s)`, 'success');
      document.getElementById('modalUrls').value = '';
      closeAddModal();
      fetchTasks();
    }

    // Settings Modal
    function openSettingsModal() {
      document.getElementById('settingsStorageDir').value = state.defaultDir;
      document.getElementById('minimizeToTrayCheckbox').checked = state.minimizeToTray;
      document.getElementById('settingsModal').classList.add('open');
    }

    function closeSettingsModal() {
      document.getElementById('settingsModal').classList.remove('open');
    }

    function setPresetDir(path) {
      document.getElementById('settingsStorageDir').value = path;
    }

    async function saveSettings() {
      const newDir = document.getElementById('settingsStorageDir').value.trim() || 'C:/Users/Alief/Downloads';
      const minTray = document.getElementById('minimizeToTrayCheckbox').checked;
      const maxConcurrent = document.getElementById('settingsMaxConcurrent').value || '5';
      const maxDlLimit = document.getElementById('settingsMaxDlLimit').value || '0';

      state.defaultDir = newDir;
      state.minimizeToTray = minTray;

      localStorage.setItem('aria2_default_dir', newDir);
      localStorage.setItem('aria2_min_tray', String(minTray));

      document.getElementById('storageTargetText').textContent = newDir;
      document.getElementById('storageTargetText').title = newDir;
      document.getElementById('modalDir').value = newDir;

      await callAria2('aria2.changeGlobalOption', [{
        dir: newDir,
        'max-concurrent-downloads': String(maxConcurrent),
        'max-overall-download-limit': String(maxDlLimit)
      }]);

      tauriInvoke('save_default_dir', { dir: newDir });
      tauriInvoke('set_minimize_to_tray', { enabled: minTray });

      showToast('Settings saved & applied', 'success');
      closeSettingsModal();
    }

    function switchFilter(f) {
      state.activeFilter = f;
      document.querySelectorAll('.nav-item').forEach(el => el.classList.remove('active'));
      const activeEl = Array.from(document.querySelectorAll('.nav-item')).find(el => el.getAttribute('onclick')?.includes(f));
      if (activeEl) activeEl.classList.add('active');
      renderTaskRows();
    }

    function onSearchChange() {
      state.search = document.getElementById('searchInput').value.trim();
      renderTaskRows();
    }

    
    // Desktop Context Menu & Native Key Interceptors
    function initContextMenuAndHotkeys() {
      // 1. Prevent default browser reload and devtools keys
      window.addEventListener('keydown', (e) => {
        if (e.key === 'F5' || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'r')) {
          e.preventDefault(); // Prevent browser refresh
        }
        if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'u') {
          e.preventDefault(); // Prevent view-source
        }
        if (e.key === 'Escape') {
          closeContextMenu();
        }
      });

      // 2. Global Context Menu Interceptor
      window.addEventListener('contextmenu', (e) => {
        // Let standard input/textarea have native text editing menu (Cut/Copy/Paste)
        if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {
          return;
        }

        e.preventDefault(); // Kill the ugly browser menu!

        const row = e.target.closest('.task-row');
        if (row) {
          const gid = row.getAttribute('data-gid');
          showTaskContextMenu(e.clientX, e.clientY, gid);
        } else {
          showGlobalContextMenu(e.clientX, e.clientY);
        }
      });

      // Close menu on click anywhere or window blur/scroll
      window.addEventListener('click', () => closeContextMenu());
      window.addEventListener('scroll', () => closeContextMenu(), true);
      window.addEventListener('blur', () => closeContextMenu());
    }

    function showTaskContextMenu(x, y, gid) {
      const t = state.tasks.find(x => x.gid === gid);
      if (!t) return;

      const menu = document.getElementById('customContextMenu');
      const isComplete = t.status === 'complete';
      const isAct = t.status === 'active';
      const isPau = t.status === 'paused' || t.status === 'waiting';
      const filePath = getFilePath(t);
      const uri = (t.files && t.files[0] && t.files[0].uris && t.files[0].uris[0]) ? t.files[0].uris[0].uri : '';

      menu.innerHTML = `
        ${isComplete ? `
          <div class="context-menu-item" onclick="handleOpenFile('${t.gid}')">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6M15 3h6v6M10 14L21 3"/></svg>
            <span>Open / Run File</span>
            <span class="context-menu-shortcut">Double-click</span>
          </div>
        ` : ''}
        <div class="context-menu-item" onclick="revealFolder('${t.gid}')">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/></svg>
          <span>Show in File Explorer</span>
        </div>
        ${uri ? `
          <div class="context-menu-item" onclick="copyToClipboard('${escapeHtml(uri)}', 'Source URL copied')">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M10 13a5 5 0 007.54.54l3-3a5 5 0 00-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 00-7.54-.54l-3 3a5 5 0 007.07 7.07l1.71-1.71"/></svg>
            <span>Copy Download Link</span>
          </div>
        ` : ''}
        <div class="context-menu-item" onclick="copyToClipboard('${escapeHtml(filePath)}', 'File path copied')">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg>
          <span>Copy File Path</span>
        </div>
        <div class="context-menu-divider"></div>
        ${isAct ? `
          <div class="context-menu-item" onclick="pauseJob('${t.gid}')">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/></svg>
            <span>Pause Transfer</span>
          </div>
        ` : ''}
        ${isPau ? `
          <div class="context-menu-item" onclick="resumeJob('${t.gid}')">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="5 3 19 12 5 21 5 3"/></svg>
            <span>Resume Transfer</span>
          </div>
        ` : ''}
        <div class="context-menu-item danger" onclick="requestDelete('${t.gid}')">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/></svg>
          <span>Delete Transfer...</span>
        </div>
      `;

      positionAndOpenMenu(menu, x, y);
    }

    function showGlobalContextMenu(x, y) {
      const menu = document.getElementById('customContextMenu');
      menu.innerHTML = `
        <div class="context-menu-item" onclick="openAddModal()">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
          <span>New Download</span>
          <span class="context-menu-shortcut">Ctrl+N</span>
        </div>
        <div class="context-menu-item" onclick="openSettingsModal()">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-2 2 2 2 0 01-2-2v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83 0 2 2 0 010-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 01-2-2 2 2 0 012-2h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 010-2.83 2 2 0 012.83 0l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 012-2 2 2 0 012 2v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 0 2 2 0 010 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9a1.65 1.65 0 001.51 1H21a2 2 0 012 2 2 2 0 01-2 2h-.09a1.65 1.65 0 00-1.51 1z"/></svg>
          <span>Preferences & Storage</span>
          <span class="context-menu-shortcut">Ctrl+,</span>
        </div>
        <div class="context-menu-divider"></div>
        <div class="context-menu-item" onclick="clearDoneTransfers()">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/></svg>
          <span>Clear Completed Items</span>
        </div>
      `;

      positionAndOpenMenu(menu, x, y);
    }

    function positionAndOpenMenu(menu, x, y) {
      const maxX = window.innerWidth - 230;
      const maxY = window.innerHeight - 260;
      const posX = Math.min(x, maxX);
      const posY = Math.min(y, maxY);

      menu.style.left = `${posX}px`;
      menu.style.top = `${posY}px`;
      menu.classList.add('open');
    }

    function closeContextMenu() {
      const menu = document.getElementById('customContextMenu');
      if (menu) menu.classList.remove('open');
    }

    function copyToClipboard(text, toastMsg) {
      if (navigator.clipboard) {
        navigator.clipboard.writeText(text);
      } else {
        const inp = document.createElement('input');
        inp.value = text;
        document.body.appendChild(inp);
        inp.select();
        document.execCommand('copy');
        inp.remove();
      }
      showToast(toastMsg || 'Copied to clipboard', 'info');
      closeContextMenu();
    }

    // Toast Manager
    function showToast(message, type = 'info') {
      const container = document.getElementById('toastContainer');
      const toast = document.createElement('div');
      toast.className = `toast ${type}`;
      toast.textContent = message;
      container.appendChild(toast);

      requestAnimationFrame(() => {
        toast.classList.add('show');
      });

      setTimeout(() => {
        toast.classList.remove('show');
        setTimeout(() => toast.remove(), 250);
      }, 3000);
    }

    // Helpers
    function getFileName(t) {
      if (t.bittorrent && t.bittorrent.info && t.bittorrent.info.name) return t.bittorrent.info.name;
      if (t.files && t.files.length > 0) {
        const f = t.files[0];
        if (f.path) {
          const parts = f.path.replace(/\\/g, '/').split('/');
          return parts[parts.length - 1] || 'Download';
        }
        if (f.uris && f.uris.length > 0) {
          try {
            const u = new URL(f.uris[0].uri);
            const pathParts = u.pathname.split('/');
            return decodeURIComponent(pathParts[pathParts.length - 1]) || u.hostname;
          } catch(e) {
            return f.uris[0].uri;
          }
        }
      }
      return 'Transfer_' + t.gid;
    }

    function getFilePath(t) {
      if (t.files && t.files[0] && t.files[0].path && t.files[0].path.trim() !== '') return t.files[0].path;
      if (t.dir) {
        const name = getFileName(t);
        return `${t.dir.replace(/\\/g, '/')}/${name}`;
      }
      return state.defaultDir;
    }

    function renderStatusText(status) {
      switch(status) {
        case 'active': return 'Active';
        case 'complete': return 'Finished';
        case 'paused':
        case 'waiting': return 'Paused';
        case 'error': return 'Failed';
        default: return status;
      }
    }

    function formatBytes(b) {
      if (!b || b === 0) return '0 B';
      const k = 1024;
      const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
      const i = Math.floor(Math.log(b) / Math.log(k));
      return (b / Math.pow(k, i)).toFixed(i === 0 ? 0 : 1) + ' ' + sizes[i];
    }

    function formatSpeed(b) {
      if (!b || b === 0) return { val: '0.0', unit: 'KB/s' };
      if (b < 1024 * 1024) return { val: (b / 1024).toFixed(1), unit: 'KB/s' };
      return { val: (b / (1024 * 1024)).toFixed(1), unit: 'MB/s' };
    }

    function formatEta(s) {
      if (s < 60) return `${s}s left`;
      if (s < 3600) return `${Math.floor(s / 60)}m ${s % 60}s`;
      return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`;
    }
  