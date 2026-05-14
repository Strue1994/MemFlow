import { Router } from 'express';
import axios from 'axios';
import { toolRegistry, HermesTool } from './tool_registry';
import { buildExecutorHeaders } from './index';

const router = Router();
const EXECUTOR_URL = process.env.EXECUTOR_URL || 'http://localhost:8080';
const AGENT_MODE = process.env.AGENT_MODE || 'standard';

router.get('/tools', (req, res) => {
  const tools = toolRegistry.getAllTools();
  res.json({
    tools,
    count: tools.length,
  });
});

router.post('/execute', async (req, res) => {
  const { tool, arguments: args } = req.body;

  if (!tool) {
    return res.status(400).json({ success: false, error: 'Missing tool name' });
  }

  const toolDef = toolRegistry.getTool(tool);
  if (!toolDef) {
    return res.status(404).json({ success: false, error: `Tool ${tool} not found` });
  }

  try {
    const response = await axios.post(
      `${EXECUTOR_URL}/execute`,
      {
        workflowId: toolDef.workflowId,
        params: args || {},
      },
      { headers: buildExecutorHeaders(true) }
    );

    res.json({
      success: true,
      output: response.data,
    });
  } catch (error: any) {
    res.json({
      success: false,
      error: error.response?.data?.error || error.message,
    });
  }
});

router.post('/chat', async (req, res) => {
  if (AGENT_MODE !== 'hermes') {
    return res.status(400).json({ error: 'Hermes mode not enabled. Set AGENT_MODE=hermes' });
  }

  const { message, context } = req.body;
  if (!message) {
    return res.status(400).json({ error: 'Missing message' });
  }

  const tools = toolRegistry.getAllTools();
  const toolDescriptions = tools.map(t => 
    `- ${t.name}: ${t.description}`
  ).join('\n');

  const systemPrompt = `You are a helpful assistant with access to MemFlow workflows as tools.

Available tools:
${toolDescriptions}

When the user asks for something that can be done with a workflow, respond with a JSON object:
{ "action": "execute_tool", "tool": "tool_name", "arguments": {...} }

For other requests, respond normally.`;

  try {
    const openai = require('openai');
    const client = new openai.OpenAI();
    
    const completion = await client.chat.completions.create({
      model: 'gpt-4o-mini',
      messages: [
        { role: 'system', content: systemPrompt },
        ...(context || []),
        { role: 'user', content: message },
      ],
    });

    const response = completion.choices[0]?.message?.content || '';

    let action = null;
    try {
      const jsonMatch = response.match(/\{[\s\S]*\}/);
      if (jsonMatch) {
        action = JSON.parse(jsonMatch[0]);
      }
    } catch {}

    if (action?.action === 'execute_tool') {
      const result = await axios.post(
        `${EXECUTOR_URL}/execute`,
        { workflowId: action.tool.replace('memflow_workflow_', ''), params: action.arguments || {} },
        { headers: buildExecutorHeaders(true) }
      );

      return res.json({
        response: `Executed ${action.tool}: ${JSON.stringify(result.data)}`,
        execution: result.data,
      });
    }

    res.json({ response });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

export default router;