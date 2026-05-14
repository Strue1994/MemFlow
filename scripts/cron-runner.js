const fs = require('fs');
const path = require('path');

const runtimeRoot = process.env.MEMFLOW_RUNTIME_ROOT || path.resolve(__dirname, '..', '.memflow-runtime');
const configPath = path.join(runtimeRoot, 'config', 'cron-workflows.json');
const statePath = path.join(runtimeRoot, 'state', 'cron-runner-state.json');
const loopIntervalMs = Number(process.env.MEMFLOW_CRON_LOOP_MS || '10000');
const RETURNLESS_SUCCESS_MESSAGE = 'Invalid return: no value to return';

function readJson(filePath, fallback) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function writeJson(filePath, data) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, JSON.stringify(data, null, 2), 'utf8');
}

function ensureStateShape(state) {
  return {
    lastRunAt: state?.lastRunAt || {},
    lastSuccessAt: state?.lastSuccessAt || {},
    lastSoftSuccessAt: state?.lastSoftSuccessAt || {},
    lastResult: state?.lastResult || {},
  };
}

async function executeWorkflow(executorUrl, executorKey, workflowId) {
  const response = await fetch(`${executorUrl}/execute`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-API-Key': executorKey,
    },
    body: JSON.stringify({
      workflow_id: workflowId,
      params: {},
    }),
  });

  const text = await response.text();
  let payload = null;
  try {
    payload = text ? JSON.parse(text) : null;
  } catch {
    payload = text;
  }

  if (!response.ok) {
    const message = payload && typeof payload === 'object' ? payload.error || payload.message : String(payload);
    throw new Error(message || `HTTP ${response.status}`);
  }

  return payload;
}

async function tick() {
  const config = readJson(configPath, { entries: [] });
  const state = ensureStateShape(readJson(statePath, {}));
  const executorUrl = process.env.EXECUTOR_URL || config.executorUrl || 'http://127.0.0.1:8082';
  const executorKey = process.env.EXECUTOR_API_KEY || config.executorKey || 'memflow-cron-key';
  const now = Date.now();

  for (const entry of config.entries || []) {
    if (!entry.enabled) {
      continue;
    }

    const lastRunAt = state.lastRunAt[entry.slug] ? new Date(state.lastRunAt[entry.slug]).getTime() : 0;
    if (now - lastRunAt < entry.intervalSeconds * 1000) {
      continue;
    }

    try {
      const result = await executeWorkflow(executorUrl, executorKey, entry.workflowId);
      console.log(`[cron-runner] executed ${entry.slug}`, JSON.stringify(result).slice(0, 200));
      state.lastSuccessAt[entry.slug] = new Date().toISOString();
      state.lastResult[entry.slug] = {
        status: 'success',
        at: state.lastSuccessAt[entry.slug],
      };
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      const isSoftSuccess =
        entry.successMode === 'soft-success-on-invalid-return' &&
        typeof message === 'string' &&
        message.includes(RETURNLESS_SUCCESS_MESSAGE);

      if (isSoftSuccess) {
        const at = new Date().toISOString();
        console.log(`[cron-runner] soft-success ${entry.slug}: ${message}`);
        state.lastSuccessAt[entry.slug] = at;
        state.lastSoftSuccessAt[entry.slug] = at;
        state.lastResult[entry.slug] = {
          status: 'soft-success',
          at,
          message,
        };
      } else {
        console.log(`[cron-runner] workflow ${entry.slug} reported: ${message}`);
        state.lastResult[entry.slug] = {
          status: 'failed',
          at: new Date().toISOString(),
          message,
        };
      }
    }

    state.lastRunAt[entry.slug] = new Date().toISOString();
    writeJson(statePath, state);
  }
}

async function main() {
  console.log(`[cron-runner] using config ${configPath}`);
  if (!fs.existsSync(configPath)) {
    console.log('[cron-runner] config file not found, waiting for seed step');
  }

  await tick();
  setInterval(() => {
    void tick();
  }, loopIntervalMs);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
