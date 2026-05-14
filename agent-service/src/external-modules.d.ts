declare module "axios" {
  const axios: any;
  export default axios;
}

declare module "redis" {
  export type RedisClientType = any;
  export function createClient(...args: any[]): any;
}

declare module "telegraf" {
  export class Telegraf {
    telegram: any;
    constructor(...args: any[]);
    command(...args: any[]): any;
    on(...args: any[]): any;
  }
}
