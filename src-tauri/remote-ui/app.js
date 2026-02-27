// kiri remote - PWA application logic (WebSocket-only, optimistic UI)

// ── State ─────────────────────────────────────────────
var ws = null;
var reconnectTimer = null;
var lastStatus = null;

// ── DOM elements ──────────────────────────────────────
var statusDot = document.getElementById('status-dot');
var statusText = document.getElementById('status-text');
var openProjectsEl = document.getElementById('open-projects');
var recentProjectsEl = document.getElementById('recent-projects');
var terminalsEl = document.getElementById('terminals');

// ── Initialize ────────────────────────────────────────
function init() {
  if ('serviceWorker' in navigator) {
    var basePath = getBasePath();
    navigator.serviceWorker.register(basePath + 'sw.js', { scope: basePath }).catch(function () {});
  }
  connectWebSocket();
}

function getBasePath() {
  var path = location.pathname;
  var parts = path.split('/').filter(Boolean);
  if (parts.length > 0) {
    return '/' + parts[0] + '/';
  }
  return '/';
}

// ── WebSocket ─────────────────────────────────────────
function connectWebSocket() {
  if (ws) ws.close();

  var protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
  var basePath = getBasePath();
  ws = new WebSocket(protocol + '//' + location.host + basePath + 'ws');

  ws.onopen = function () {
    setStatus('connected', 'Connected');
    clearTimeout(reconnectTimer);
  };

  ws.onmessage = function (e) {
    try {
      var data = JSON.parse(e.data);
      lastStatus = data;
      renderDashboard(data);
    } catch (err) {
      console.error('Failed to parse WS message:', err);
    }
  };

  ws.onclose = function () {
    setStatus('disconnected', 'Disconnected');
    reconnectTimer = setTimeout(connectWebSocket, 3000);
  };

  ws.onerror = function () {
    setStatus('disconnected', 'Error');
  };
}

function sendAction(action) {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(action));
  }
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

// ── Actions (Optimistic UI) ──────────────────────────
function openProject(path) {
  if (lastStatus) {
    var project = null;
    var newRecent = [];
    (lastStatus.recentProjects || []).forEach(function (p) {
      if (p.path === path) {
        project = p;
      } else {
        newRecent.push(p);
      }
    });
    if (project) {
      var openList = lastStatus.openProjects || [];
      openList.push({ path: project.path, name: project.name, branch: project.gitBranch || null });
      lastStatus.openProjects = openList;
      lastStatus.recentProjects = newRecent;
      renderDashboard(lastStatus);
    }
  }
  sendAction({ action: 'openProject', path: path });
}

function closeProject(path) {
  if (lastStatus) {
    var closedProject = null;
    var newOpen = [];
    (lastStatus.openProjects || []).forEach(function (p) {
      if (p.path === path) {
        closedProject = p;
      } else {
        newOpen.push(p);
      }
    });
    lastStatus.openProjects = newOpen;
    if (closedProject) {
      var recentList = lastStatus.recentProjects || [];
      recentList.unshift({
        path: closedProject.path,
        name: closedProject.name,
        lastOpened: Math.floor(Date.now() / 1000),
        gitBranch: closedProject.branch,
      });
      lastStatus.recentProjects = recentList;
    }
    renderDashboard(lastStatus);
  }
  sendAction({ action: 'closeProject', path: path });
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
