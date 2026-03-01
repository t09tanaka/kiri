// kiri remote - PWA application logic (WebSocket-only, diff-based updates)

// ── State ─────────────────────────────────────────────
var ws = null;
var reconnectTimer = null;
var lastStatus = null;
var bottomSheetOpen = false;

// ── DOM elements ──────────────────────────────────────
var statusDot = document.getElementById('status-dot');
var statusText = document.getElementById('status-text');
var projectsEl = document.getElementById('projects');
var recentProjectsEl = document.getElementById('recent-projects');
var bottomSheetBackdrop = document.getElementById('bottom-sheet-backdrop');
var bottomSheet = document.getElementById('bottom-sheet');

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

// ── Render (diff-based, no innerHTML replacement) ─────
function renderDashboard(data) {
  renderProjects(data.openProjects || [], data.terminals || []);
  if (bottomSheetOpen) {
    renderRecentProjects(data.recentProjects || []);
  }
}

function renderProjects(projects, terminals) {
  if (projects.length === 0 && terminals.length === 0) {
    syncChildren(projectsEl, [
      { key: '__empty', html: '<p class="empty-state">No open projects</p>' },
    ]);
    return;
  }

  var items = [];

  projects.forEach(function (p) {
    // Find terminals associated with this project (by path prefix matching in cwd if available)
    var projectTerminals = [];
    var remainingTerminals = [];
    terminals.forEach(function (t) {
      if (t.cwd && t.cwd.indexOf(p.path) === 0) {
        projectTerminals.push(t);
      } else {
        remainingTerminals.push(t);
      }
    });
    terminals = remainingTerminals;

    items.push({
      key: 'project-' + p.path,
      html: buildProjectCard(p, projectTerminals),
    });
  });

  // Remaining terminals (not associated with any project)
  if (terminals.length > 0) {
    items.push({
      key: '__terminals',
      html: buildTerminalsCard(terminals),
    });
  }

  syncChildren(projectsEl, items);
}

function buildProjectCard(p, terminals) {
  var html =
    '<div class="card-header">' +
    '<span class="project-name">' +
    escapeHtml(p.name) +
    '</span>' +
    (p.branch ? '<span class="branch-badge">' + escapeHtml(p.branch) + '</span>' : '') +
    '</div>' +
    '<div class="card-path">' +
    escapeHtml(shortenPath(p.path)) +
    '</div>';

  if (terminals.length > 0) {
    html += '<div class="card-terminals">';
    terminals.forEach(function (t) {
      html +=
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
        '</div>';
    });
    html += '</div>';
  }

  html +=
    '<div class="card-actions">' +
    '<button class="btn btn-danger btn-sm" onclick="closeProject(\'' +
    escapeAttr(p.path) +
    '\')">Close</button>' +
    '</div>';

  return html;
}

function buildTerminalsCard(terminals) {
  var html =
    '<div class="card-header">' +
    '<span class="project-name terminals-label">Terminals</span>' +
    '</div>' +
    '<div class="card-terminals">';
  terminals.forEach(function (t) {
    html +=
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
      '</div>';
  });
  html += '</div>';
  return html;
}

function renderRecentProjects(projects) {
  if (projects.length === 0) {
    syncChildren(recentProjectsEl, [
      { key: '__empty', html: '<p class="empty-state">No recent projects</p>' },
    ]);
    return;
  }

  var items = projects.map(function (p) {
    return {
      key: 'recent-' + p.path,
      html:
        '<div class="card-header">' +
        '<span class="project-name">' +
        escapeHtml(p.name) +
        '</span>' +
        (p.gitBranch ? '<span class="branch-badge">' + escapeHtml(p.gitBranch) + '</span>' : '') +
        '</div>' +
        '<div class="card-meta">' +
        '<span>' +
        escapeHtml(shortenPath(p.path)) +
        '</span>' +
        '<span class="meta-dot">·</span>' +
        '<span>' +
        timeAgo(p.lastOpened) +
        '</span>' +
        '</div>' +
        '<div class="card-actions">' +
        '<button class="btn btn-primary btn-sm" onclick="openProject(\'' +
        escapeAttr(p.path) +
        '\')">Open</button>' +
        '</div>',
    };
  });

  syncChildren(recentProjectsEl, items);
}

// ── DOM diff helper (keyed reconciliation) ────────────
// Prevents full innerHTML replacement to avoid animation flickering.
function syncChildren(container, items) {
  var existingMap = {};
  var existingNodes = [];
  for (var i = 0; i < container.children.length; i++) {
    var child = container.children[i];
    var key = child.getAttribute('data-key');
    if (key) {
      existingMap[key] = child;
    }
    existingNodes.push(child);
  }

  // Build new node list
  var newKeys = {};
  var newNodes = [];
  items.forEach(function (item) {
    newKeys[item.key] = true;
    if (existingMap[item.key]) {
      // Update content if changed
      var el = existingMap[item.key];
      if (el.innerHTML !== item.html) {
        el.innerHTML = item.html;
      }
      newNodes.push(el);
    } else {
      // Create new node
      var div = document.createElement('div');
      div.setAttribute('data-key', item.key);
      div.className = item.key === '__empty' || item.key === '__terminals' ? '' : 'project-card';
      if (item.key === '__terminals') div.className = 'project-card terminals-only';
      div.innerHTML = item.html;
      newNodes.push(div);
    }
  });

  // Remove nodes that are no longer needed
  for (var k = 0; k < existingNodes.length; k++) {
    var node = existingNodes[k];
    var nodeKey = node.getAttribute('data-key');
    if (!nodeKey || !newKeys[nodeKey]) {
      container.removeChild(node);
    }
  }

  // Append/reorder nodes
  for (var j = 0; j < newNodes.length; j++) {
    if (container.children[j] !== newNodes[j]) {
      container.insertBefore(newNodes[j], container.children[j] || null);
    }
  }
}

// ── Bottom Sheet ──────────────────────────────────────
function openBottomSheet() {
  bottomSheetOpen = true;
  if (lastStatus) {
    renderRecentProjects(lastStatus.recentProjects || []);
  }
  bottomSheetBackdrop.classList.add('visible');
  bottomSheet.classList.add('visible');
}

function closeBottomSheet() {
  bottomSheetOpen = false;
  bottomSheetBackdrop.classList.remove('visible');
  bottomSheet.classList.remove('visible');
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
  closeBottomSheet();
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

function shortenPath(path) {
  var home = '/Users/';
  var idx = path.indexOf(home);
  if (idx === 0) {
    var rest = path.substring(home.length);
    var slashIdx = rest.indexOf('/');
    if (slashIdx !== -1) {
      return '~' + rest.substring(slashIdx);
    }
  }
  return path;
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
