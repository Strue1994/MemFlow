import axios from 'axios';

const EXECUTOR_URL = process.env.EXECUTOR_URL || 'http://127.0.0.1:8082';
const EXECUTOR_API_KEY = process.env.EXECUTOR_API_KEY || '';

interface ConversationContext {
  platform: string;
  userId: string;
  messages: { text: string; role: 'user' | 'assistant'; timestamp: number }[];
}

const MAX_CONTEXT_MESSAGES = 5;

export class NlpWorkflowCreator {
  private contextStore: Map<string, ConversationContext> = new Map();

  async createFromNaturalLanguage(
    platform: string,
    userId: string,
    userText: string
  ): Promise<{ workflowId: string; message: string }> {
    const key = `${platform}:${userId}`;
    const context = this.getContext(key);
    
    let fullDescription = userText;
    if (context.messages.length > 0) {
      const recentHistory = context.messages
        .slice(-3)
        .map(m => m.text)
        .join('; ');
      fullDescription = `用户历史: ${recentHistory}。当前需求: ${userText}`;
    }

    try {
      const response = await axios.post(
        `${EXECUTOR_URL}/create_workflow_v2`,
        {
          description: fullDescription,
          user_id: userId,
        },
        {
          headers: {
            'Content-Type': 'application/json',
            'X-API-Key': EXECUTOR_API_KEY,
          },
        }
      );

      const workflowId = response.data.workflow_id || response.data.id;
      
      this.addToContext(key, userText, 'user');
      this.addToContext(key, `Created workflow ${workflowId}`, 'assistant');

      return {
        workflowId,
        message: `✅ 已为您创建工作流 [ID: ${workflowId}]，您可以使用"运行 ${workflowId}"来执行。`,
      };
    } catch (error: any) {
      console.error('[NlpWorkflowCreator] Error:', error.message);
      
      this.addToContext(key, userText, 'user');
      this.addToContext(key, `创建失败: ${error.message}`, 'assistant');

      return {
        workflowId: '',
        message: `❌ 创建工作流失败: ${error.response?.data?.error || error.message}\n请更详细地描述您的需求。`,
      };
    }
  }

  async executeWorkflow(workflowId: string, params?: Record<string, any>): Promise<any> {
    try {
      const response = await axios.post(
        `${EXECUTOR_URL}/execute`,
        {
          workflow_id: workflowId,
          params: params || {},
        },
        {
          headers: {
            'Content-Type': 'application/json',
            'X-API-Key': EXECUTOR_API_KEY,
          },
        }
      );
      return response.data;
    } catch (error: any) {
      return { success: false, error: error.message };
    }
  }

  private getContext(key: string): ConversationContext {
    if (!this.contextStore.has(key)) {
      this.contextStore.set(key, {
        platform: key.split(':')[0],
        userId: key.split(':')[1],
        messages: [],
      });
    }
    return this.contextStore.get(key)!;
  }

  addToContext(key: string, text: string, role: 'user' | 'assistant'): void {
    const context = this.getContext(key);
    context.messages.push({ text, role, timestamp: Date.now() });
    
    if (context.messages.length > MAX_CONTEXT_MESSAGES) {
      context.messages = context.messages.slice(-MAX_CONTEXT_MESSAGES);
    }
  }

  getContextMessages(key: string): string[] {
    const context = this.contextStore.get(key);
    if (!context) return [];
    
    return context.messages.map(m => `[${m.role}]: ${m.text}`);
  }

  clearContext(key: string): void {
    this.contextStore.delete(key);
  }
}

export const workflowCreator = new NlpWorkflowCreator();
