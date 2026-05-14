const fs = require('fs');
const path = require('path');

const runtimeRoot = process.env.MEMFLOW_RUNTIME_ROOT || path.resolve(__dirname, '..', '.memflow-runtime');
const configDir = path.join(runtimeRoot, 'config');
const outputPath = path.join(configDir, 'cron-workflows.json');
const executorUrl = process.env.EXECUTOR_URL || 'http://127.0.0.1:8082';
const executorKey = process.env.EXECUTOR_API_KEY || 'memflow-cron-key';

async function requestJson(url, options = {}) {
  const response = await fetch(url, options);
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

async function requestJsonSoft(url, options = {}) {
  try {
    return await requestJson(url, options);
  } catch (error) {
    return { __error: error instanceof Error ? error.message : String(error) };
  }
}

function triggerNode(id, nextId) {
  return {
    id,
    name: 'Trigger',
    type: 'trigger',
    parameters: { type: 'manual', cron: '' },
    position: [100, 180],
    inputs: [],
    outputs: nextId ? [nextId] : [],
  };
}

function httpNode(id, name, url, headers, nextId, position) {
  return {
    id,
    name,
    type: 'httpRequest',
    parameters: {
      url,
      method: 'GET',
      headers,
      response_format: 'response',
    },
    position,
    inputs: ['node_1'],
    outputs: nextId ? [nextId] : [],
  };
}

function setNode(id, variableName, value, position) {
  return {
    id,
    name: 'Set Variable',
    type: 'set',
    parameters: {
      values: {
        [variableName]: value,
      },
    },
    position,
    inputs: ['node_2'],
    outputs: [],
  };
}

function writeFileNode(id, filePath, content, append, position) {
  return {
    id,
    name: 'Write File',
    type: 'writeFile',
    parameters: {
      path: filePath,
      content,
      append,
    },
    position,
    inputs: ['node_1'],
    outputs: [],
  };
}

function buildDefinitions() {
  return [
    {
      slug: 'cron-local-runtime-heartbeat',
      name: 'Cron Local Runtime Heartbeat',
      intervalSeconds: Number(process.env.MEMFLOW_CRON_HEALTH_INTERVAL || '180'),
      description: 'Append a local runtime heartbeat file using the executor file sandbox.',
      successMode: 'soft-success-on-invalid-return',
      verification: {
        kind: 'file-append',
        path: 'cron/runtime-heartbeat.txt',
      },
      workflow: {
        name: 'Cron Local Runtime Heartbeat',
        nodes: [
          triggerNode('node_1', 'node_2'),
          writeFileNode(
            'node_2',
            'cron/runtime-heartbeat.txt',
            'runtime_heartbeat\n',
            true,
            [320, 180],
          ),
        ],
        connections: [
          { from: 'node_1', to: 'node_2' },
        ],
      },
    },
    {
      slug: 'cron-local-learning-marker',
      name: 'Cron Local Learning Marker',
      intervalSeconds: Number(process.env.MEMFLOW_CRON_WORKFLOW_INTERVAL || '300'),
      description: 'Append a local learning marker file to supply stable executor samples.',
      successMode: 'soft-success-on-invalid-return',
      verification: {
        kind: 'file-append',
        path: 'cron/learning-marker.txt',
      },
      workflow: {
        name: 'Cron Local Learning Marker',
        nodes: [
          triggerNode('node_1', 'node_2'),
          writeFileNode(
            'node_2',
            'cron/learning-marker.txt',
            'learning_marker\n',
            true,
            [320, 180],
          ),
        ],
        connections: [
          { from: 'node_1', to: 'node_2' },
        ],
      },
    },
    {
      slug: 'cron-local-workflow-marker',
      name: 'Cron Local Workflow Marker',
      intervalSeconds: Number(process.env.MEMFLOW_CRON_LLM_INTERVAL || '420'),
      description: 'Append a local workflow marker file so autonomy can learn from stable local runs.',
      successMode: 'soft-success-on-invalid-return',
      verification: {
        kind: 'file-append',
        path: 'cron/workflow-marker.txt',
      },
      workflow: {
        name: 'Cron Local Workflow Marker',
        nodes: [
          triggerNode('node_1', 'node_2'),
          writeFileNode(
            'node_2',
            'cron/workflow-marker.txt',
            'workflow_marker\n',
            true,
            [320, 180],
          ),
        ],
        connections: [
          { from: 'node_1', to: 'node_2' },
        ],
      },
    },
  ];
}

async function main() {
  fs.mkdirSync(configDir, { recursive: true });

  const existing = await requestJsonSoft(`${executorUrl}/workflows`, {
    headers: { 'X-API-Key': executorKey },
  });
  const workflows = Array.isArray(existing) ? existing : [];
  const definitions = buildDefinitions();
  const schedule = [];

  if (!Array.isArray(existing) && existing && existing.__error) {
    console.log(`[seed-cron-workflows] workflow listing unavailable: ${existing.__error}`);
  }

  for (const definition of definitions) {
    const match = workflows.find((workflow) => workflow.name === definition.name);
    let workflowId = match?.id || null;

    if (!workflowId) {
      const created = await requestJson(`${executorUrl}/compile`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': executorKey,
        },
        body: JSON.stringify({
          name: definition.name,
          n8n_json: definition.workflow,
        }),
      });
      workflowId = created.workflow_id;
    }

    schedule.push({
      slug: definition.slug,
      name: definition.name,
      description: definition.description,
      workflowId,
      intervalSeconds: definition.intervalSeconds,
      enabled: true,
      successMode: definition.successMode,
      verification: definition.verification,
    });
  }

  fs.writeFileSync(
    outputPath,
    JSON.stringify(
      {
        generatedAt: new Date().toISOString(),
        executorUrl,
        executorKey,
        entries: schedule,
      },
      null,
      2,
    ),
    'utf8',
  );

  console.log(`Seeded ${schedule.length} cron workflows into ${outputPath}`);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
