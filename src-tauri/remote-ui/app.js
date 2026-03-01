// kiri remote - Application logic (WebSocket-only, diff-based updates)

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
      {
        key: '__empty',
        html:
          '<div class="empty-state">' +
          '<div class="empty-state-icon">' +
          '<svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round">' +
          '<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>' +
          '</svg>' +
          '</div>' +
          '<p class="empty-state-text">No open projects</p>' +
          '<p class="empty-state-hint">Tap "Open Project" to get started</p>' +
          '</div>',
      },
    ]);
    return;
  }

  var items = [];

  // Distribute terminals to projects by cwd match, rest go to first project
  var unmatched = terminals.slice();
  var projectTerminalMap = {};

  projects.forEach(function (p) {
    projectTerminalMap[p.path] = [];
  });

  // First pass: match by cwd
  var stillUnmatched = [];
  unmatched.forEach(function (t) {
    var matched = false;
    if (t.cwd) {
      projects.forEach(function (p) {
        if (!matched && t.cwd.indexOf(p.path) === 0) {
          projectTerminalMap[p.path].push(t);
          matched = true;
        }
      });
    }
    if (!matched) stillUnmatched.push(t);
  });

  // Unmatched terminals go to the first project
  if (stillUnmatched.length > 0 && projects.length > 0) {
    projectTerminalMap[projects[0].path] =
      projectTerminalMap[projects[0].path].concat(stillUnmatched);
  }

  projects.forEach(function (p) {
    items.push({
      key: 'project-' + p.path,
      html: buildProjectCard(p, projectTerminalMap[p.path] || []),
    });
  });

  syncChildren(projectsEl, items);
}

function buildProjectCard(p, terminals) {
  var html =
    '<div class="card-header">' +
    '<span class="project-name">' +
    escapeHtml(p.name) +
    '</span>' +
    (p.branch ? '<span class="branch-badge">' + escapeHtml(p.branch) + '</span>' : '') +
    '<button class="btn-close" onclick="closeProject(\'' +
    escapeAttr(p.path) +
    '\')" aria-label="Close">' +
    '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">' +
    '<line x1="18" y1="6" x2="6" y2="18"></line>' +
    '<line x1="6" y1="6" x2="18" y2="18"></line>' +
    '</svg>' +
    '</button>' +
    '</div>' +
    '<div class="card-path">' +
    escapeHtml(shortenPath(p.path)) +
    '</div>';

  if (terminals.length > 0) {
    html += '<div class="card-terminal-tags">';
    terminals.forEach(function (t) {
      var name = t.processName ? escapeHtml(t.processName) : 'idle';
      html +=
        '<span class="terminal-tag ' +
        (t.isAlive ? 'active' : 'idle') +
        '">' +
        '<span class="terminal-tag-dot"></span>' +
        name +
        ' #' +
        t.id +
        '</span>';
    });
    html += '</div>';
  }

  return html;
}

function renderRecentProjects(projects) {
  if (projects.length === 0) {
    syncChildren(recentProjectsEl, [
      {
        key: '__empty',
        html: '<p class="empty-state-text" style="padding:24px 16px;">No recent projects</p>',
      },
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
      div.className = item.key === '__empty' ? '' : 'project-card';
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
