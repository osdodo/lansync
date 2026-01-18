const sharedTextDom = document.getElementById('sharedText');
const statusDom = document.getElementById('status');
const UPDATE_DELAY = 1000;
let ws = null, reconnectInterval = null, sendTimeout = null, isComposing = false, lastUpdateTime = 0;

function connect() {
    ws = new WebSocket((window.location.protocol === 'https:' ? 'wss:' : 'ws:') + '//' + window.location.host + '/ws');
    ws.onopen = () => {
        if (reconnectInterval) {
            clearInterval(reconnectInterval);
            reconnectInterval = null;
        }
        showStatus('Connected', 'success');
    };
    ws.onmessage = event => {
        const text = event.data;
        const textChanged = text !== sharedTextDom.value;
        if (textChanged && Date.now() - lastUpdateTime >= UPDATE_DELAY) {
            sharedTextDom.value = text;
        }
    };
    ws.onclose = () => {
        showStatus('Disconnected', 'error');
        if (!reconnectInterval) {
            reconnectInterval = setInterval(connect, 3000);
        }
    };
    ws.onerror = () => showStatus('Error', 'error');
}

function showStatus(text, type) {
    statusDom.textContent = text;
    statusDom.className = 'status ' + type;
    if (type !== 'sync') setTimeout(() => {
        if (statusDom.textContent === text) statusDom.className = 'status';
    }, 1500);
}

function sendText() {
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(sharedTextDom.value);
        showStatus('Synced', 'success');
        lastUpdateTime = Date.now();
    }
}

function scheduleSend() {
    clearTimeout(sendTimeout);
    sendTimeout = setTimeout(sendText, 300);
}

sharedTextDom.addEventListener('compositionstart', () => isComposing = true);
sharedTextDom.addEventListener('compositionend', () => { isComposing = false; scheduleSend(); });
sharedTextDom.addEventListener('input', () => { if (!isComposing) scheduleSend(); });
sharedTextDom.addEventListener('paste', () => { lastUpdateTime = Date.now(); setTimeout(sendText, 100); });
sharedTextDom.addEventListener('blur', () => { if (!isComposing) sendText(); });

function tryConnect() {
    connect();
    setTimeout(() => { if (!ws || ws.readyState !== WebSocket.OPEN) tryConnect(); }, 3000);
}

document.readyState === 'complete' ? tryConnect() : window.addEventListener('load', tryConnect);
