
import { text } from '../lib/schema';
import { newProjectContext, ProjectContext } from './setup';

describe('Upsert', () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map(ctx => ctx.deleteCollections()));
  });

  test('upsert to non-existent collection', async () => {
    const ctx = getContext();
    await expect(
      ctx.client.collection('missing').upsert([{ _id: 'one' }])
    ).rejects.toThrow(/collection not found/);
  });

  test('upsert basic', async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection('test', {});

    const lsn = await ctx.client.collection(collection.name).upsert([{ _id: 'one' }]);
    expect(lsn).toBe(1);
  });

  test('upsert batch', async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection('test', {});

    const lsn = await ctx.client.collection(collection.name).upsert([
      { _id: 'one' },
      { _id: 'two' }
    ]);
    expect(lsn).toBe(1);
  });

  test('upsert sequential', async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection('test', {});

    let lsn = await ctx.client.collection(collection.name).upsert([{ _id: 'one' }]);
    expect(lsn).toBe(1);

    lsn = await ctx.client.collection(collection.name).upsert([{ _id: 'two' }]);
    expect(lsn).toBe(2);

    lsn = await ctx.client.collection(collection.name).upsert([{ _id: 'three' }]);
    expect(lsn).toBe(3);
  });

  test('upsert no documents', async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection('test', {});

    await expect(
      ctx.client.collection(collection.name).upsert([])
    ).rejects.toThrow(/invalid argument/);
  });

  test('upsert invalid document', async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection('test', {});

    await expect(
      ctx.client.collection(collection.name).upsert([{}])
    ).rejects.toThrow(/invalid argument/);
  });

  test('upsert schema validation', async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection('test', {
      name: text().required()
    });

    await expect(
      ctx.client.collection(collection.name).upsert([{ _id: 'one' }])
    ).rejects.toThrow(/invalid argument/);
  });
});
