import { Telegraf } from 'telegraf';
import { MessageAdapter, ParsedMessage } from './adapter';

const TELEGRAM_BOT_TOKEN = process.env.TELEGRAM_BOT_TOKEN || '';

export class TelegramAdapter implements MessageAdapter {
  platform = 'telegram';
  private bot: Telegraf;

  constructor() {
    this.bot = new Telegraf(TELEGRAM_BOT_TOKEN);
  }

  parse(raw: any): ParsedMessage {
    const message = raw.message || raw;
    const userId = message.from?.id?.toString() || '';
    const text = message.text || message.caption || '';

    return {
      userId,
      text: text.trim(),
      raw,
      platform: 'telegram',
    };
  }

  verify(req: any): boolean {
    return true;
  }

  async send(userId: string, reply: string): Promise<void> {
    await this.bot.telegram.sendMessage(userId, reply);
  }

  getBot(): Telegraf {
    return this.bot;
  }
}

export function createTelegramAdapter(): MessageAdapter {
  return new TelegramAdapter();
}

export function setupTelegramWebhook(bot: Telegraf, webhookPath: string): void {
  bot.command('start', (ctx) => {
    ctx.reply('Welcome to MemFlow! Send me a message to get started.');
  });

  bot.on('message', async (ctx) => {
    console.log(`[Telegram] Received: ${ctx.message.text}`);
  });
}