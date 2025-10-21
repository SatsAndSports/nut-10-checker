// IndexedDB setup
const DB_NAME = 'nut10-checker';
const DB_VERSION = 1;

let db;

async function initDB() {
    return new Promise((resolve, reject) => {
        const request = indexedDB.open(DB_NAME, DB_VERSION);

        request.onerror = () => reject(request.error);
        request.onsuccess = () => {
            db = request.result;
            resolve(db);
        };

        request.onupgradeneeded = (event) => {
            const db = event.target.result;

            // Mints store
            if (!db.objectStoreNames.contains('mints')) {
                const mintsStore = db.createObjectStore('mints', { keyPath: 'url' });
                mintsStore.createIndex('url', 'url', { unique: true });
            }

            // Test results store
            if (!db.objectStoreNames.contains('results')) {
                const resultsStore = db.createObjectStore('results', { keyPath: 'testId' });
                resultsStore.createIndex('timestamp', 'timestamp', { unique: false });
                resultsStore.createIndex('mintUrl', 'mintUrl', { unique: false });
                resultsStore.createIndex('testType', 'testType', { unique: false });
            }
        };
    });
}

// Mints management
async function saveMint(url, info) {
    // Check if this is the first mint - if so, make it active
    const allMints = await getMints();
    const isActive = allMints.length === 0;

    const tx = db.transaction(['mints'], 'readwrite');
    const store = tx.objectStore('mints');
    await store.put({ url, info, addedAt: Date.now(), isActive });
    return tx.complete;
}

async function setActiveMint(url) {
    const tx = db.transaction(['mints'], 'readwrite');
    const store = tx.objectStore('mints');

    // Get all mints
    const allMints = await new Promise((resolve, reject) => {
        const request = store.getAll();
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
    });

    // Update all mints - set isActive based on url match
    for (const mint of allMints) {
        mint.isActive = (mint.url === url);
        await store.put(mint);
    }

    return tx.complete;
}

async function getActiveMint() {
    const mints = await getMints();
    return mints.find(m => m.isActive) || (mints.length > 0 ? mints[0] : null);
}

async function getMints() {
    const tx = db.transaction(['mints'], 'readonly');
    const store = tx.objectStore('mints');
    return new Promise((resolve, reject) => {
        const request = store.getAll();
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
    });
}

async function getMint(url) {
    const tx = db.transaction(['mints'], 'readonly');
    const store = tx.objectStore('mints');
    return new Promise((resolve, reject) => {
        const request = store.get(url);
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
    });
}

async function deleteMint(url) {
    const tx = db.transaction(['mints'], 'readwrite');
    const store = tx.objectStore('mints');
    await store.delete(url);
    return tx.complete;
}

// Test results management
async function saveTestResult(result) {
    const tx = db.transaction(['results'], 'readwrite');
    const store = tx.objectStore('results');
    await store.put(result);
    return tx.complete;
}

async function getTestResults(filter = null) {
    const tx = db.transaction(['results'], 'readonly');
    const store = tx.objectStore('results');

    return new Promise((resolve, reject) => {
        const request = store.getAll();
        request.onsuccess = () => {
            let results = request.result;

            // Sort by timestamp descending
            results.sort((a, b) => b.timestamp - a.timestamp);

            // Apply filter
            if (filter === 'passed') {
                results = results.filter(r => r.passed);
            } else if (filter === 'failed') {
                results = results.filter(r => !r.passed);
            }

            resolve(results);
        };
        request.onerror = () => reject(request.error);
    });
}

async function clearTestResults() {
    const tx = db.transaction(['results'], 'readwrite');
    const store = tx.objectStore('results');
    await store.clear();
    return tx.complete;
}

// Export results to JSON
function exportResults(results) {
    const blob = new Blob([JSON.stringify(results, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `nut10-test-results-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
}

// UI rendering
function renderMints(mints) {
    const container = document.getElementById('mints-list');

    if (mints.length === 0) {
        container.innerHTML = '<p class="empty-state">No mints added yet. Add one below.</p>';
        return;
    }

    container.innerHTML = mints.map(mint => `
        <div class="mint-item ${mint.isActive ? 'active' : ''}" data-url="${mint.url}">
            <div>
                <div class="mint-url">
                    ${mint.isActive ? '⭐ ' : ''}${mint.url}
                    ${mint.isActive ? ' <span class="active-badge">(Active)</span>' : ''}
                </div>
                <div class="mint-balance">Balance: Loading...</div>
            </div>
            <div class="mint-actions">
                ${!mint.isActive ? `<button class="set-active-btn" data-url="${mint.url}">Set Active</button>` : ''}
                <button class="delete-mint-btn" data-url="${mint.url}">Remove</button>
            </div>
        </div>
    `).join('');

    // Add set active handlers
    container.querySelectorAll('.set-active-btn').forEach(btn => {
        btn.addEventListener('click', async () => {
            const url = btn.dataset.url;
            await setActiveMint(url);
            await loadAndRenderMints();

            // Re-fetch balances after re-rendering
            const mints = await getMints();
            for (const mint of mints) {
                // Dispatch event to trigger balance update
                window.dispatchEvent(new CustomEvent('updateBalance', { detail: { mintUrl: mint.url } }));
            }
        });
    });

    // Add delete handlers
    container.querySelectorAll('.delete-mint-btn').forEach(btn => {
        btn.addEventListener('click', async () => {
            const url = btn.dataset.url;
            if (confirm(`Remove mint ${url}?`)) {
                await deleteMint(url);
                await loadAndRenderMints();
            }
        });
    });
}

function renderResults(results) {
    const container = document.getElementById('results-list');

    if (results.length === 0) {
        container.innerHTML = '<p class="empty-state">No test results yet. Run some tests!</p>';
        return;
    }

    container.innerHTML = results.map(result => `
        <div class="test-result ${result.passed ? 'passed' : 'failed'}">
            <div class="test-result-header">
                <span class="test-result-title">${result.testType} - ${result.testCase}</span>
                <span class="test-result-status">${result.passed ? '✓ PASS' : '✗ FAIL'}</span>
            </div>
            <div class="test-result-details">
                <div>Mint: ${result.mintUrl}</div>
                <div>Expected: ${result.expected}, Actual: ${result.actual}</div>
                <div>Time: ${new Date(result.timestamp).toLocaleString()}</div>
                ${result.details ? `<div>Details: ${result.details}</div>` : ''}
            </div>
        </div>
    `).join('');
}

async function loadAndRenderMints() {
    const mints = await getMints();
    renderMints(mints);
}

async function loadAndRenderResults(filter = 'all') {
    const results = await getTestResults(filter);
    renderResults(results);
}

export {
    initDB,
    saveMint,
    getMints,
    getMint,
    setActiveMint,
    getActiveMint,
    deleteMint,
    saveTestResult,
    getTestResults,
    clearTestResults,
    exportResults,
    renderMints,
    renderResults,
    loadAndRenderMints,
    loadAndRenderResults
};
