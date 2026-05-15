import client from "prom-client";

const registry = new client.Registry();
client.collectDefaultMetrics({ register: registry });

export const httpRequestsTotal = new client.Counter({
  name: "memflow_http_requests_total",
  help: "Total HTTP requests",
  labelNames: ["method", "path", "status"] as const,
  registers: [registry],
});

export const agentExecuteDuration = new client.Histogram({
  name: "memflow_agent_execute_duration_seconds",
  help: "Agent execute duration",
  buckets: [0.05, 0.1, 0.25, 0.5, 1, 2, 5, 10, 30],
  registers: [registry],
});

export const activeSessions = new client.Gauge({
  name: "memflow_active_sessions",
  help: "Active traced sessions",
  registers: [registry],
});

export const mcpConnectionsActive = new client.Gauge({
  name: "memflow_mcp_connections_active",
  help: "Active MCP connections",
  registers: [registry],
});

export async function metricsText(): Promise<string> {
  return registry.metrics();
}
