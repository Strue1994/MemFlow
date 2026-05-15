/**
 * Minimal test MCP server for QA testing
 * Handles line-delimited JSON-RPC over stdin/stdout
 */
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin });

rl.on('line', (line) => {
  const trimmed = line.trim();
  if (!trimmed) return;

  try {
    const req = JSON.parse(trimmed);
    const res = { jsonrpc: '2.0', id: req.id };

    if (req.method === 'initialize') {
      res.result = { protocolVersion: '2025-03-26', capabilities: {}, serverInfo: { name: 'test-mcp', version: '1.0' } };
    } else if (req.method === 'notifications/initialized') {
      return; // no response expected
    } else if (req.method === 'tools/list') {
      res.result = {
        tools: [
          { name: 'echo', description: 'Echo back the input text', inputSchema: { type: 'object', properties: { text: { type: 'string' } }, required: ['text'] } },
          { name: 'add', description: 'Add two numbers', inputSchema: { type: 'object', properties: { a: { type: 'number' }, b: { type: 'number' } }, required: ['a', 'b'] } },
        ]
      };
    } else if (req.method === 'tools/call') {
      const args = req.params.arguments || {};
      if (req.params.name === 'echo') {
        res.result = { content: [{ type: 'text', text: `Echo: ${args.text || ''}` }] };
      } else if (req.params.name === 'add') {
        res.result = { content: [{ type: 'text', text: `Sum: ${(args.a || 0) + (args.b || 0)}` }] };
      } else {
        res.result = { content: [{ type: 'text', text: `Unknown tool: ${req.params.name}` }] };
      }
    }

    process.stdout.write(JSON.stringify(res) + '\n');
  } catch (e) {
    // ignore parse errors
  }
});
