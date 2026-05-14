import crypto from 'crypto';
import axios from 'axios';
import { MessageAdapter, ParsedMessage } from './adapter';

const WECOM_TOKEN = process.env.WECOM_TOKEN || '';
const WECOM_ENCODING_AES_KEY = process.env.WECOM_ENCODING_AES_KEY || '';
const WECOM_CORP_ID = process.env.WECOM_CORP_ID || '';
const WECOM_CORP_SECRET = process.env.WECOM_CORP_SECRET || '';

export class WeComAdapter implements MessageAdapter {
  platform = 'wecom';
  private accessToken: string | null = null;
  private tokenExpireTime = 0;

  parse(raw: any): ParsedMessage {
    const msgType = raw.MsgType;
    const userId = raw.FromUserName || raw.UserID || '';
    let text = '';

    if (msgType === 'text') {
      text = raw.Content || '';
    } else if (msgType === 'event') {
      text = raw.Event || '';
    }

    return {
      userId,
      text: text.trim(),
      raw,
      platform: 'wecom',
    };
  }

  verify(req: any): boolean {
    const signature = req.query?.signature || req.headers?.['x-wecom-signature'];
    const timestamp = req.query?.timestamp || req.headers?.['x-wecom-timestamp'];
    const nonce = req.query?.nonce || req.headers?.['x-wecom-nonce'];

    const str = [WECOM_TOKEN, timestamp, nonce].sort().join('');
    const hash = crypto.createHash('sha1').update(str).digest('hex');

    return hash === signature;
  }

  async send(userId: string, reply: string): Promise<void> {
    const token = await this.getAccessToken();
    
    await axios.post(
      `https://qyapi.weixin.qq.com/cgi-bin/message/send?access_token=${token}`,
      {
        touser: userId,
        msgtype: 'text',
        agentid: process.env.WECOM_AGENT_ID,
        text: { content: reply },
      }
    );
  }

  private async getAccessToken(): Promise<string> {
    if (this.accessToken && Date.now() < this.tokenExpireTime) {
      return this.accessToken;
    }

    const resp = await axios.get(
      `https://qyapi.weixin.qq.com/cgi-bin/gettoken?corpid=${WECOM_CORP_ID}&corpsecret=${WECOM_CORP_SECRET}`
    );

    if (resp.data.errcode !== 0) {
      throw new Error(`WeCom token error: ${resp.data.errmsg}`);
    }

    this.accessToken = resp.data.access_token;
    this.tokenExpireTime = Date.now() + (resp.data.expires_in - 300) * 1000;
    return this.accessToken!;
  }
}

export function createWeComAdapter(): MessageAdapter {
  return new WeComAdapter();
}