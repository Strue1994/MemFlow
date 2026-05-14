import axios from 'axios';
import { MessageAdapter, ParsedMessage } from './adapter';

const FEISHU_APP_ID = process.env.FEISHU_APP_ID || '';
const FEISHU_APP_SECRET = process.env.FEISHU_APP_SECRET || '';

export class FeishuAdapter implements MessageAdapter {
  platform = 'feishu';
  private tenantAccessToken: string | null = null;
  private tokenExpireTime = 0;

  parse(raw: any): ParsedMessage {
    const header = raw.header || {};
    const eventType = header.event_type || '';
    
    let userId = '';
    let text = '';

    if (eventType === 'im.message') {
      const message = raw.event?.message || raw;
      userId = message.sender_id?.user_id || message.sender_id?.open_id || '';
      
      const messageType = message.message_type;
      if (messageType === 'text') {
        text = message.elements?.[0]?.text || message.content?.text || '';
      } else if (messageType === 'image') {
        text = '[图片]';
      }
    } else if (eventType === 'im.message.reaction.added') {
      userId = raw.event?.user_id || '';
      text = '[Reaction]';
    }

    return {
      userId,
      text: text.trim(),
      raw,
      platform: 'feishu',
    };
  }

  verify(req: any): boolean {
    return true;
  }

  async send(userId: string, reply: string): Promise<void> {
    const token = await this.getTenantAccessToken();
    
    await axios.post(
      'https://open.feishu.cn/open-apis/im/v1/messages',
      {
        receive_id_type: 'user_id',
        receive_id: userId,
        msg_type: 'text',
        content: JSON.stringify({ text: reply }),
      },
      {
        headers: { Authorization: `Bearer ${token}` },
      }
    );
  }

  private async getTenantAccessToken(): Promise<string> {
    if (this.tenantAccessToken && Date.now() < this.tokenExpireTime) {
      return this.tenantAccessToken;
    }

    const resp = await axios.post(
      'https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal',
      { app_id: FEISHU_APP_ID, app_secret: FEISHU_APP_SECRET }
    );

    if (!resp.data.tenant_access_token) {
      throw new Error('Failed to get Feishu tenant token');
    }

    this.tenantAccessToken = resp.data.tenant_access_token;
    this.tokenExpireTime = Date.now() + (resp.data.expire - 300) * 1000;
    return this.tenantAccessToken!;
  }
}

export function createFeishuAdapter(): MessageAdapter {
  return new FeishuAdapter();
}