// IndexedDB setup
const DB_NAME = 'nut10-checker';
const DB_VERSION = 2;

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
            const oldVersion = event.oldVersion;

            // Mints store
            if (!db.objectStoreNames.contains('mints')) {
                const mintsStore = db.createObjectStore('mints', { keyPath: 'url' });
                mintsStore.createIndex('url', 'url', { unique: true });
            }

            // Test results store - migrate to new schema
            if (oldVersion < 2) {
                // Drop old results store if it exists
                if (db.objectStoreNames.contains('results')) {
                    db.deleteObjectStore('results');
                }

                // Create new results store with updated schema
                const resultsStore = db.createObjectStore('results', {
                    keyPath: 'id',
                    autoIncrement: true
                });
                resultsStore.createIndex('testId', 'testId', { unique: false });
                resultsStore.createIndex('timestamp', 'timestamp', { unique: false });
                resultsStore.createIndex('mintUrl', 'mintUrl', { unique: false });
                resultsStore.createIndex('state', 'state', { unique: false });
            }
        };
    });
}

// Mints management
async function saveMint(url, info) {
    const tx = db.transaction(['mints'], 'readwrite');
    const store = tx.objectStore('mints');

    // Get all existing mints
    const allMints = await new Promise((resolve, reject) => {
        const request = store.getAll();
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
    });

    // Deactivate all existing mints
    for (const mint of allMints) {
        mint.isActive = false;
        await store.put(mint);
    }

    // Add new mint as active
    await store.put({ url, info, addedAt: Date.now(), isActive: true });
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

            // Sort by timestamp descending, then by testId ascending, then by step number ascending
            results.sort((a, b) => {
                if (b.timestamp !== a.timestamp) {
                    return b.timestamp - a.timestamp;  // Most recent batch first
                }
                if (a.testId !== b.testId) {
                    return a.testId.localeCompare(b.testId);  // Tests in order within batch
                }
                return a.stepNumber - b.stepNumber;  // Steps in order within test
            });

            // Apply filter
            if (filter === 'passed') {
                results = results.filter(r => r.state === 'Passed');
            } else if (filter === 'failed') {
                results = results.filter(r => r.state === 'Failed');
            }

            resolve(results);
        };
        request.onerror = () => reject(request.error);
    });
}

// Get all results for a specific test run
async function getTestRunResults(testId) {
    const tx = db.transaction(['results'], 'readonly');
    const store = tx.objectStore('results');
    const index = store.index('testId');

    return new Promise((resolve, reject) => {
        const request = index.getAll(testId);
        request.onsuccess = () => {
            const results = request.result;
            results.sort((a, b) => a.stepNumber - b.stepNumber);
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
                <div class="mint-url">${mint.url}</div>
                <div class="mint-balance">Balance: Loading...</div>
            </div>
            <div class="mint-actions">
                ${mint.isActive
                    ? '<button class="active-badge-btn" disabled>Active</button>'
                    : `<button class="set-active-btn" data-url="${mint.url}">Set Active</button>`
                }
                <button class="withdraw-btn" data-url="${mint.url}">Withdraw</button>
                <button class="check-state-btn" data-url="${mint.url}">Check State</button>
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

    // Add withdraw handlers
    container.querySelectorAll('.withdraw-btn').forEach(btn => {
        btn.addEventListener('click', async () => {
            const url = btn.dataset.url;
            window.dispatchEvent(new CustomEvent('withdraw', { detail: { mintUrl: url } }));
        });
    });

    // Add check state handlers
    container.querySelectorAll('.check-state-btn').forEach(btn => {
        btn.addEventListener('click', async () => {
            const url = btn.dataset.url;
            window.dispatchEvent(new CustomEvent('checkState', { detail: { mintUrl: url } }));
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

    // Group results by testId
    const grouped = {};
    for (const result of results) {
        if (!grouped[result.testId]) {
            grouped[result.testId] = {
                testId: result.testId,
                testName: result.testName,
                mintUrl: result.mintUrl,
                timestamp: result.timestamp,
                expectedSteps: result.expectedSteps,
                steps: []
            };
        }
        grouped[result.testId].steps.push(result);
    }

    // Render grouped results
    const tests = Object.values(grouped);
    let previousTimestamp = null;
    let html = '';

    tests.forEach(test => {
        // Determine overall status
        const hasFailed = test.steps.some(s => s.state === 'Failed');
        const hasSkipped = test.steps.some(s => s.state === 'Skipped');
        const allPassed = test.steps.every(s => s.state === 'Passed');

        // Get expected steps from test metadata (fallback to steps length for old tests)
        const expectedSteps = test.expectedSteps || test.steps.length;
        const isComplete = test.steps.length >= expectedSteps;

        const overallStatus = hasFailed ? 'failed' :
                             (allPassed && isComplete) ? 'passed' :
                             'partial';

        // Check if this is the start of a new batch
        const isNewBatch = test.timestamp !== previousTimestamp;

        // Add batch header if this is a new batch
        if (isNewBatch) {
            html += `
                <div class="batch-header">
                    <div class="batch-mint">${test.mintUrl}</div>
                    <div class="batch-time">${new Date(test.timestamp).toLocaleString()}</div>
                </div>
            `;
            previousTimestamp = test.timestamp;
        }

        html += `
            <div class="test-result ${overallStatus}">
                <div class="test-result-header">
                    <span class="test-result-title">${test.testName}</span>
                    <span class="test-result-status">
                        ${hasFailed ? '✗ FAILED' : (allPassed && isComplete ? '✓ PASSED' : '⚠ IN PROGRESS')}
                    </span>
                </div>
                <div class="test-result-details">
                    <div>Test ID: ${test.testId}</div>
                    <div class="test-steps">
                        ${test.steps.map(step => {
                            const icon = step.state === 'Passed' ? '✓' :
                                        step.state === 'Failed' ? '✗' : '⊘';
                            const stateClass = step.state.toLowerCase();
                            return `
                                <div class="test-step ${stateClass}">
                                    <span class="step-icon">${icon}</span>
                                    <span class="step-name">Step ${step.stepNumber}: ${step.stepName}</span>
                                    <span class="step-result">(Expected: ${step.expected}, Got: ${step.actual})</span>
                                    ${step.details ? `<div class="step-details">${step.details}</div>` : ''}
                                </div>
                            `;
                        }).join('')}
                    </div>
                </div>
            </div>
        `;
    });

    container.innerHTML = html;
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
    getTestRunResults,
    clearTestResults,
    exportResults,
    renderMints,
    renderResults,
    loadAndRenderMints,
    loadAndRenderResults
};
