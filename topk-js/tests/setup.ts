import { v4 as uuidv4 } from "uuid";
import { Client } from "../index.js";

export class ProjectContext {
  client: Client;
  scopePrefix: string;
  collectionsCreated: string[] = [];

  constructor(client: Client, scopePrefix: string) {
    this.client = client;
    this.scopePrefix = scopePrefix;
  }

  scope(name: string) {
    return `${this.scopePrefix}-${name}`;
  }

  async createCollection(name: string, schema: any = {}) {
    const collection = await this.client
      .collections()
      .create(this.scope(name), schema);
    this.collectionsCreated.push(collection.name);
    return collection;
  }

  async deleteCollections() {
    await Promise.all(
      this.collectionsCreated.map(async (collection) => {
        try {
          await this.client.collections().delete(collection);
        } catch (e) {
          console.error(`Error deleting collection ${collection}: ${e}`);
        }
      })
    );
  }
}

export function newProjectContext() {
  const TOPK_API_KEY = process.env.TOPK_API_KEY;
  const TOPK_HOST = process.env.TOPK_HOST || "topk.io";
  const TOPK_REGION = process.env.TOPK_REGION || "elastica";
  const TOPK_HTTPS =
    process.env.TOPK_HTTPS !== "false" && process.env.TOPK_HTTPS !== "0";

  const client = new Client({
    apiKey: TOPK_API_KEY,
    region: TOPK_REGION,
    host: TOPK_HOST,
    https: TOPK_HTTPS,
  });

  return new ProjectContext(client, `topk-js-${uuidv4()}`);
}

export function isSorted(result: any[], fieldName: string): boolean {
  const values = result.map((doc) => doc[fieldName]);
  return values.every((value, i) => i === 0 || value >= values[i - 1]);
}
