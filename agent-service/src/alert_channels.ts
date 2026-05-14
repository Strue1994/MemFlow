export type AlertChannel = 'feishu' | 'slack' | 'email' | 'discord' | 'webhook';

export interface AlertMessage {
  title: string;
  content: string;
  severity: 'info' | 'warning' | 'critical';
  workflowId?: string;
}

export interface ChannelConfig {
  type: AlertChannel;
  enabled: boolean;
  webhookUrl?: string;
  apiKey?: string;
}

export class MultiAlertChannel {
  private channels: Map<AlertChannel, ChannelConfig> = new Map();

  constructor(configs: ChannelConfig[]) {
    configs.forEach(c => this.channels.set(c.type, c));
  }

  async send(message: AlertMessage): Promise<void> {
    const enabledChannels = Array.from(this.channels.entries())
      .filter(([_, config]) => config.enabled)
      .map(([type, _]) => type);

    await Promise.all(
      enabledChannels.map(channel => this.sendToChannel(channel, message))
    );
  }

  private async sendToChannel(channel: AlertChannel, message: AlertMessage): Promise<void> {
    const config = this.channels.get(channel);
    if (!config?.enabled) return;

    switch (channel) {
      case 'feishu':
        await this.sendFeishu(config, message);
        break;
      case 'slack':
        await this.sendSlack(config, message);
        break;
      case 'email':
        await this.sendEmail(config, message);
        break;
      default:
        await this.sendWebhook(config, message);
    }
  }

  private async sendFeishu(config: ChannelConfig, message: AlertMessage): Promise<void> {
    if (!config.webhookUrl) return;
    await fetch(config.webhookUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        msg_type: 'text',
        content: `[${message.severity.toUpperCase()}] ${message.title}\n${message.content}`,
      }),
    });
  }

  private async sendSlack(config: ChannelConfig, message: AlertMessage): Promise<void> {
    if (!config.webhookUrl) return;
    await fetch(config.webhookUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        text: `[${message.severity.toUpperCase()}] ${message.title}`,
        blocks: [
          { type: 'section', text: { type: 'mrkdwn', text: `*${message.title}*` } },
          { type: 'section', text: { type: 'mrkdwn', text: message.content } },
        ],
      }),
    });
  }

  private async sendEmail(config: ChannelConfig, message: AlertMessage): Promise<void> {
    console.log(`[Email] ${message.title}: ${message.content}`);
  }

  private async sendWebhook(config: ChannelConfig, message: AlertMessage): Promise<void> {
    if (!config.webhookUrl) return;
    await fetch(config.webhookUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(message),
    });
  }
}

export const createAlertChannel = (configs: ChannelConfig[]) => new MultiAlertChannel(configs);