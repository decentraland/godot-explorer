// Scene Logger Frontend Application

const API_BASE = '/api/v1';

// State
const state = {
    currentSession: null,
    currentView: 'entities',
    selectedEntity: null,
    selectedItem: null,
    filters: {
        entity: null,
        component: null,
        tickFrom: null,
        tickTo: null,
        operation: null,
        opName: null,
        isAsync: null,
        hasError: null,
    },
    pagination: {
        messages: { offset: 0, limit: 100 },
        ops: { offset: 0, limit: 100 },
    },
    entities: [],
    componentNames: new Set(),
};

// DOM Elements
const elements = {
    sessionSelect: document.getElementById('session-select'),
    statsSummary: document.getElementById('stats-summary'),
    tabs: document.querySelectorAll('.tab'),
    entityList: document.getElementById('entity-list'),
    entityCount: document.getElementById('entity-count'),
    entityTimeline: document.getElementById('entity-timeline'),
    selectedEntityInfo: document.getElementById('selected-entity-info'),
    messagesList: document.getElementById('messages-list'),
    messagesCount: document.getElementById('messages-count'),
    opsList: document.getElementById('ops-list'),
    opsCount: document.getElementById('ops-count'),
    detailContent: document.getElementById('detail-content'),
    filterEntity: document.getElementById('filter-entity'),
    filterComponent: document.getElementById('filter-component'),
    filterTickFrom: document.getElementById('filter-tick-from'),
    filterTickTo: document.getElementById('filter-tick-to'),
    filterOperation: document.getElementById('filter-operation'),
    filterOpName: document.getElementById('filter-op-name'),
    filterIsAsync: document.getElementById('filter-is-async'),
    applyFilters: document.getElementById('apply-filters'),
    clearFilters: document.getElementById('clear-filters'),
};

// API Functions
async function fetchJson(url) {
    const response = await fetch(url);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return response.json();
}

async function loadSessions() {
    const sessions = await fetchJson(`${API_BASE}/sessions`);
    elements.sessionSelect.innerHTML = sessions.map(s =>
        `<option value="${s.session_id}" ${s.is_current ? 'selected' : ''}>
            ${s.session_id.substring(0, 8)}... ${s.is_current ? '(current)' : ''} - ${formatBytes(s.file_size)}
        </option>`
    ).join('');

    if (sessions.length > 0) {
        state.currentSession = sessions.find(s => s.is_current)?.session_id || sessions[0].session_id;
        await loadSessionData();
    }
}

async function loadSessionData() {
    if (!state.currentSession) return;

    try {
        const [stats, entities] = await Promise.all([
            fetchJson(`${API_BASE}/sessions/${state.currentSession}/stats`),
            fetchJson(`${API_BASE}/sessions/${state.currentSession}/entities?limit=1000`),
        ]);

        elements.statsSummary.textContent =
            `CRDT: ${stats.total_crdt_messages.toLocaleString()} | Ops: ${stats.total_op_calls.toLocaleString()}`;

        state.entities = entities.data || [];
        elements.entityCount.textContent = `(${state.entities.length})`;

        // Collect component names
        state.componentNames.clear();
        for (const entity of state.entities) {
            for (const comp of entity.components || []) {
                state.componentNames.add(comp);
            }
        }
        updateComponentFilter();

        renderEntityList();
    } catch (error) {
        console.error('Failed to load session data:', error);
    }
}

async function loadEntityTimeline(entityId) {
    if (!state.currentSession) return;

    try {
        const messages = await fetchJson(
            `${API_BASE}/sessions/${state.currentSession}/entities/${entityId}`
        );

        elements.selectedEntityInfo.textContent = `Entity ${entityId} - ${messages.length} messages`;
        renderTimeline(messages);
    } catch (error) {
        console.error('Failed to load entity timeline:', error);
    }
}

async function loadMessages() {
    if (!state.currentSession) return;

    const params = new URLSearchParams({
        limit: state.pagination.messages.limit,
        offset: state.pagination.messages.offset,
    });

    if (state.filters.entity) params.set('entity', state.filters.entity);
    if (state.filters.component) params.set('component', state.filters.component);
    if (state.filters.tickFrom) params.set('tick_from', state.filters.tickFrom);
    if (state.filters.tickTo) params.set('tick_to', state.filters.tickTo);
    if (state.filters.operation) params.set('operation', state.filters.operation);

    try {
        const result = await fetchJson(
            `${API_BASE}/sessions/${state.currentSession}/messages?${params}`
        );

        elements.messagesCount.textContent = `(${result.data.length} shown)`;
        renderMessagesList(result.data);
        updatePagination('messages', result);
    } catch (error) {
        console.error('Failed to load messages:', error);
    }
}

async function loadOpCalls() {
    if (!state.currentSession) return;

    const params = new URLSearchParams({
        limit: state.pagination.ops.limit,
        offset: state.pagination.ops.offset,
    });

    if (state.filters.opName) params.set('op_name', state.filters.opName);
    if (state.filters.isAsync !== null) params.set('is_async', state.filters.isAsync);
    if (state.filters.hasError !== null) params.set('has_error', state.filters.hasError);

    try {
        const result = await fetchJson(
            `${API_BASE}/sessions/${state.currentSession}/op-calls?${params}`
        );

        elements.opsCount.textContent = `(${result.data.length} shown)`;
        renderOpsList(result.data);
        updatePagination('ops', result);
    } catch (error) {
        console.error('Failed to load op calls:', error);
    }
}

// Render Functions
function renderEntityList() {
    elements.entityList.innerHTML = state.entities.map(entity => `
        <div class="entity-item ${state.selectedEntity === entity.entity_id ? 'selected' : ''}"
             data-entity-id="${entity.entity_id}">
            <div class="entity-id">Entity ${entity.entity_number}:${entity.entity_version}</div>
            <div class="entity-info">
                Ticks ${entity.first_seen_tick} - ${entity.last_seen_tick} | ${entity.message_count} msgs
            </div>
            <div class="entity-components">
                ${(entity.components || []).slice(0, 5).map(c =>
                    `<span class="component-tag">${c}</span>`
                ).join('')}
                ${entity.components?.length > 5 ? `<span class="component-tag">+${entity.components.length - 5}</span>` : ''}
            </div>
        </div>
    `).join('');

    // Add click handlers
    elements.entityList.querySelectorAll('.entity-item').forEach(item => {
        item.addEventListener('click', () => {
            const entityId = parseInt(item.dataset.entityId);
            selectEntity(entityId);
        });
    });
}

function renderTimeline(messages) {
    elements.entityTimeline.innerHTML = messages.map((msg, idx) => `
        <div class="timeline-item" data-index="${idx}">
            <div class="timeline-tick">T${msg.tick}</div>
            <div class="timeline-content">
                <span class="timeline-operation ${msg.operation}">${msg.operation}</span>
                <span class="timeline-component">${msg.component_name}</span>
            </div>
        </div>
    `).join('');

    // Store messages for detail view
    elements.entityTimeline.messages = messages;

    // Add click handlers
    elements.entityTimeline.querySelectorAll('.timeline-item').forEach(item => {
        item.addEventListener('click', () => {
            const idx = parseInt(item.dataset.index);
            selectTimelineItem(item, elements.entityTimeline.messages[idx]);
        });
    });
}

function renderMessagesList(messages) {
    elements.messagesList.innerHTML = messages.map((msg, idx) => `
        <div class="data-item" data-index="${idx}">
            <div>T${msg.tick}</div>
            <div class="timeline-operation ${msg.operation}">${msg.operation}</div>
            <div>${msg.component_name} (E${msg.entity_number}:${msg.entity_version})</div>
            <div>${msg.raw_size_bytes}B</div>
        </div>
    `).join('');

    elements.messagesList.messages = messages;

    elements.messagesList.querySelectorAll('.data-item').forEach(item => {
        item.addEventListener('click', () => {
            const idx = parseInt(item.dataset.index);
            selectListItem(item, elements.messagesList.messages[idx]);
        });
    });
}

function renderOpsList(calls) {
    elements.opsList.innerHTML = calls.map((call, idx) => `
        <div class="data-item op-item" data-index="${idx}">
            <div class="op-name">${call.op_name}</div>
            <div class="op-async ${call.is_async ? 'async' : 'sync'}">${call.is_async ? 'async' : 'sync'}</div>
            <div class="op-duration">${call.duration_ms.toFixed(2)}ms</div>
            <div class="op-status ${call.error ? 'error' : 'success'}">${call.error ? 'ERR' : 'OK'}</div>
        </div>
    `).join('');

    elements.opsList.calls = calls;

    elements.opsList.querySelectorAll('.data-item').forEach(item => {
        item.addEventListener('click', () => {
            const idx = parseInt(item.dataset.index);
            selectListItem(item, elements.opsList.calls[idx]);
        });
    });
}

function renderDetail(data) {
    elements.detailContent.innerHTML = `<div class="json-viewer">${formatJson(data)}</div>`;

    // Add expand/collapse handlers
    elements.detailContent.querySelectorAll('.json-expandable').forEach(el => {
        el.addEventListener('click', (e) => {
            e.stopPropagation();
            el.classList.toggle('expanded');
        });
    });
}

// Selection Functions
function selectEntity(entityId) {
    state.selectedEntity = entityId;
    renderEntityList();
    loadEntityTimeline(entityId);
    switchView('entities');
}

function selectTimelineItem(element, data) {
    elements.entityTimeline.querySelectorAll('.timeline-item').forEach(i => i.classList.remove('selected'));
    element.classList.add('selected');
    renderDetail(data);
}

function selectListItem(element, data) {
    element.parentElement.querySelectorAll('.data-item').forEach(i => i.classList.remove('selected'));
    element.classList.add('selected');
    renderDetail(data);
}

// View Switching
function switchView(view) {
    state.currentView = view;

    elements.tabs.forEach(tab => {
        tab.classList.toggle('active', tab.dataset.view === view);
    });

    document.querySelectorAll('.view').forEach(v => {
        v.classList.toggle('active', v.id === `${view}-view`);
    });

    // Show/hide appropriate filters
    document.getElementById('entity-filters').style.display =
        view === 'messages' ? 'block' : 'none';
    document.getElementById('component-filters').style.display =
        view === 'messages' ? 'block' : 'none';
    document.getElementById('tick-filters').style.display =
        view === 'messages' ? 'block' : 'none';
    document.getElementById('operation-filters').style.display =
        view === 'messages' ? 'block' : 'none';
    document.getElementById('ops-filters').style.display =
        view === 'ops' ? 'block' : 'none';

    // Load data for the view
    if (view === 'messages') {
        loadMessages();
    } else if (view === 'ops') {
        loadOpCalls();
    }
}

// Filter Functions
function updateComponentFilter() {
    const sorted = Array.from(state.componentNames).sort();
    elements.filterComponent.innerHTML =
        '<option value="">All Components</option>' +
        sorted.map(c => `<option value="${c}">${c}</option>`).join('');
}

function applyFilters() {
    state.filters.entity = elements.filterEntity.value || null;
    state.filters.component = elements.filterComponent.value || null;
    state.filters.tickFrom = elements.filterTickFrom.value || null;
    state.filters.tickTo = elements.filterTickTo.value || null;
    state.filters.operation = elements.filterOperation.value || null;
    state.filters.opName = elements.filterOpName?.value || null;
    state.filters.isAsync = elements.filterIsAsync?.value ? elements.filterIsAsync.value === 'true' : null;

    // Reset pagination
    state.pagination.messages.offset = 0;
    state.pagination.ops.offset = 0;

    if (state.currentView === 'messages') {
        loadMessages();
    } else if (state.currentView === 'ops') {
        loadOpCalls();
    }
}

function clearFilters() {
    elements.filterEntity.value = '';
    elements.filterComponent.value = '';
    elements.filterTickFrom.value = '';
    elements.filterTickTo.value = '';
    elements.filterOperation.value = '';
    if (elements.filterOpName) elements.filterOpName.value = '';
    if (elements.filterIsAsync) elements.filterIsAsync.value = '';

    state.filters = {
        entity: null,
        component: null,
        tickFrom: null,
        tickTo: null,
        operation: null,
        opName: null,
        isAsync: null,
        hasError: null,
    };

    applyFilters();
}

// Pagination
function updatePagination(type, result) {
    const prevBtn = document.getElementById(`${type}-prev`);
    const nextBtn = document.getElementById(`${type}-next`);
    const pageInfo = document.getElementById(`${type}-page-info`);

    const { offset, limit } = state.pagination[type];
    const page = Math.floor(offset / limit) + 1;

    prevBtn.disabled = offset === 0;
    nextBtn.disabled = !result.has_more;
    pageInfo.textContent = `Page ${page}`;
}

// Utility Functions
function formatBytes(bytes) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatJson(obj, indent = 0) {
    if (obj === null) return '<span class="json-null">null</span>';
    if (typeof obj === 'boolean') return `<span class="json-boolean">${obj}</span>`;
    if (typeof obj === 'number') return `<span class="json-number">${obj}</span>`;
    if (typeof obj === 'string') return `<span class="json-string">"${escapeHtml(obj)}"</span>`;

    if (Array.isArray(obj)) {
        if (obj.length === 0) return '[]';
        const items = obj.map(item => formatJson(item, indent + 1)).join(',<br>');
        return `<span class="json-expandable">Array(${obj.length})</span><div class="json-children">[<br>${items}<br>]</div>`;
    }

    if (typeof obj === 'object') {
        const keys = Object.keys(obj);
        if (keys.length === 0) return '{}';
        const items = keys.map(key =>
            `<span class="json-key">"${escapeHtml(key)}"</span>: ${formatJson(obj[key], indent + 1)}`
        ).join(',<br>');
        return `<span class="json-expandable expanded">Object</span><div class="json-children" style="display:block">{<br>${items}<br>}</div>`;
    }

    return String(obj);
}

function escapeHtml(str) {
    return str.replace(/&/g, '&amp;')
              .replace(/</g, '&lt;')
              .replace(/>/g, '&gt;')
              .replace(/"/g, '&quot;');
}

// Event Listeners
elements.sessionSelect.addEventListener('change', (e) => {
    state.currentSession = e.target.value;
    state.selectedEntity = null;
    loadSessionData();
});

elements.tabs.forEach(tab => {
    tab.addEventListener('click', () => switchView(tab.dataset.view));
});

elements.applyFilters.addEventListener('click', applyFilters);
elements.clearFilters.addEventListener('click', clearFilters);

document.getElementById('messages-prev').addEventListener('click', () => {
    state.pagination.messages.offset = Math.max(0, state.pagination.messages.offset - state.pagination.messages.limit);
    loadMessages();
});

document.getElementById('messages-next').addEventListener('click', () => {
    state.pagination.messages.offset += state.pagination.messages.limit;
    loadMessages();
});

document.getElementById('ops-prev').addEventListener('click', () => {
    state.pagination.ops.offset = Math.max(0, state.pagination.ops.offset - state.pagination.ops.limit);
    loadOpCalls();
});

document.getElementById('ops-next').addEventListener('click', () => {
    state.pagination.ops.offset += state.pagination.ops.limit;
    loadOpCalls();
});

// Auto-refresh for current session
setInterval(() => {
    if (state.currentSession) {
        fetchJson(`${API_BASE}/sessions/${state.currentSession}/stats`)
            .then(stats => {
                elements.statsSummary.textContent =
                    `CRDT: ${stats.total_crdt_messages.toLocaleString()} | Ops: ${stats.total_op_calls.toLocaleString()}`;
            })
            .catch(() => {});
    }
}, 5000);

// Initialize
loadSessions();
