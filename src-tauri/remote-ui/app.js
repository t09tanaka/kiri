// kiri remote - PWA application logic

// ── State ─────────────────────────────────────────────
let token = localStorage.getItem('kiri-remote-token') || '';
let ws = null;
let reconnectTimer = null;

// ── DOM elements ──────────────────────────────────────
const authScreen = document.getElementById('auth-screen');
const dashboardScreen = document.getElementById('dashboard-screen');
const tokenInput = document.getElementById('token-input');
const authBtn = document.getElementById('auth-btn');
const authError = document.getElementById('auth-error');
const statusDot = document.getElementById('status-dot');
const statusText = document.getElementById('status-text');
const openProjectsEl = document.getElementById('open-projects');
const recentProjectsEl = document.getElementById('recent-projects');
const terminalsEl = document.getElementById('terminals');

// ── Initialize ────────────────────────────────────────
function init() {
  authBtn.addEventListener('click', authenticate);
  tokenInput.addEventListener('keydown', function (e) {
    if (e.key === 'Enter') authenticate();
  });

  // Register service worker
  if ('serviceWorker' in navigator) {
    navigator.serviceWorker.register('/sw.js').catch(function () {
      // Service worker registration failed -- not critical
    });
  }

  if (token) {
    verifyToken();
  } else {
    showAuth();
  }
}

// ── Auth ──────────────────────────────────────────────
async function authenticate() {
  var inputToken = tokenInput.value.trim();
  if (!inputToken) return;

  token = inputToken;
  await verifyToken();
}

async function verifyToken() {
  try {
    var res = await fetch('/api/auth/verify', {
      method: 'POST',
      headers: { Authorization: 'Bearer ' + token },
    });
    if (res.ok) {
      localStorage.setItem('kiri-remote-token', token);
      showDashboard();
      connectWebSocket();
    } else {
      showAuth();
      showError('Invalid token');
    }
  } catch (e) {
    showAuth();
    showError('Connection failed');
  }
}

function showAuth() {
  authScreen.hidden = false;
  dashboardScreen.hidden = true;
  setStatus('disconnected', 'Disconnected');
}

function showDashboard() {
  authScreen.hidden = true;
  dashboardScreen.hidden = false;
}

function showError(msg) {
  authError.textContent = msg;
  authError.hidden = false;
  setTimeout(function () {
    authError.hidden = true;
  }, 3000);
}

// ── WebSocket ─────────────────────────────────────────
function connectWebSocket() {
  if (ws) ws.close();

  var protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
  ws = new WebSocket(
    protocol + '//' + location.host + '/ws/status?token=' + encodeURIComponent(token)
  );

  ws.onopen = function () {
    setStatus('connected', 'Connected');
    clearTimeout(reconnectTimer);
  };

  ws.onmessage = function (e) {
    try {
      var data = JSON.parse(e.data);
      renderDashboard(data);
    } catch (err) {
      console.error('Failed to parse WS message:', err);
    }
  };

  ws.onclose = function () {
    setStatus('disconnected', 'Disconnected');
    // Auto-reconnect after 3s
    reconnectTimer = setTimeout(connectWebSocket, 3000);
  };

  ws.onerror = function () {
    setStatus('disconnected', 'Error');
  };
}

function setStatus(state, text) {
  statusDot.className = 'status-dot ' + state;
  statusText.textContent = text;
}

// ── Render ────────────────────────────────────────────
function renderDashboard(data) {
  renderOpenProjects(data.openProjects || []);
  renderTerminals(data.terminals || []);
  renderRecentProjects(data.recentProjects || []);
}

function renderOpenProjects(projects) {
  if (projects.length === 0) {
    openProjectsEl.innerHTML = '<p class="empty-state">No open projects</p>';
    return;
  }

  openProjectsEl.innerHTML = projects
    .map(function (p) {
      return (
        '<div class="project-card open">' +
        '<div class="card-header">' +
        '<span class="project-name">' +
        escapeHtml(p.name) +
        '</span>' +
        (p.branch ? '<span class="branch-badge">' + escapeHtml(p.branch) + '</span>' : '') +
        '</div>' +
        '<div class="card-path">' +
        escapeHtml(p.path) +
        '</div>' +
        '<div class="card-actions">' +
        '<button class="btn btn-danger btn-sm" onclick="closeProject(\'' +
        escapeAttr(p.path) +
        '\')">Close</button>' +
        '</div>' +
        '</div>'
      );
    })
    .join('');
}

function renderTerminals(terminals) {
  if (terminals.length === 0) {
    terminalsEl.innerHTML = '<p class="empty-state">No terminals</p>';
    return;
  }

  terminalsEl.innerHTML = terminals
    .map(function (t) {
      return (
        '<div class="terminal-item">' +
        '<span class="terminal-dot ' +
        (t.isAlive ? 'active' : 'idle') +
        '"></span>' +
        '<span class="terminal-process">' +
        (t.processName ? escapeHtml(t.processName) : 'idle') +
        '</span>' +
        '<span class="terminal-id">#' +
        t.id +
        '</span>' +
        '</div>'
      );
    })
    .join('');
}

function renderRecentProjects(projects) {
  if (projects.length === 0) {
    recentProjectsEl.innerHTML = '<p class="empty-state">No recent projects</p>';
    return;
  }

  recentProjectsEl.innerHTML = projects
    .map(function (p) {
      return (
        '<div class="project-card recent">' +
        '<div class="card-header">' +
        '<span class="project-name">' +
        escapeHtml(p.name) +
        '</span>' +
        (p.gitBranch ? '<span class="branch-badge">' + escapeHtml(p.gitBranch) + '</span>' : '') +
        '</div>' +
        '<div class="card-meta">' +
        timeAgo(p.lastOpened) +
        '</div>' +
        '<div class="card-actions">' +
        '<button class="btn btn-primary btn-sm" onclick="openProject(\'' +
        escapeAttr(p.path) +
        '\')">Open</button>' +
        '</div>' +
        '</div>'
      );
    })
    .join('');
}

// ── Actions ───────────────────────────────────────────
async function openProject(path) {
  try {
    await fetch('/api/projects/open', {
      method: 'POST',
      headers: {
        Authorization: 'Bearer ' + token,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ path: path }),
    });
  } catch (e) {
    console.error('Failed to open project:', e);
  }
}

async function closeProject(path) {
  try {
    await fetch('/api/projects/close', {
      method: 'POST',
      headers: {
        Authorization: 'Bearer ' + token,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ path: path }),
    });
  } catch (e) {
    console.error('Failed to close project:', e);
  }
}

// ── Utilities ─────────────────────────────────────────
function escapeHtml(str) {
  var div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

function escapeAttr(str) {
  return str.replace(/\\/g, '\\\\').replace(/'/g, "\\'").replace(/"/g, '\\"');
}

function timeAgo(timestamp) {
  var seconds = Math.floor(Date.now() / 1000 - timestamp);
  if (seconds < 60) return 'Just now';
  if (seconds < 3600) return Math.floor(seconds / 60) + 'm ago';
  if (seconds < 86400) return Math.floor(seconds / 3600) + 'h ago';
  return Math.floor(seconds / 86400) + 'd ago';
}

// ── Start ─────────────────────────────────────────────
init();
