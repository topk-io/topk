import { Client } from '../index.js';

export class ProjectContext {
  client: Client;
  scopePrefix: string;

  constructor(client: Client, scopePrefix: string) {
    this.client = client;
    this.scopePrefix = scopePrefix;
  }

  scope(name: string) {
    return `${this.scopePrefix}-${name}`;
  }
}

export function newProjectContext() {
  const TOPK_API_KEY = process.env.TOPK_API_KEY.split('\n')[0].trim();
  // const TOPK_HOST = process.env.TOPK_HOST || 'topk.io';
  const TOPK_REGION = process.env.TOPK_REGION || 'elastica';
  const TOPK_HTTPS = process.env.TOPK_HTTPS === 'true';

  const client = new Client({
    apiKey: TOPK_API_KEY,
    region: TOPK_REGION,
    // host: TOPK_HOST,
    https: TOPK_HTTPS,
  });

  return new ProjectContext(
    client,
    `test-${Math.random().toString().replace('.', '')}`
  );
}
