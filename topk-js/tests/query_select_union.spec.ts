import { field, select } from '../lib/query';
import { newProjectContext, ProjectContext } from './setup';
import { u8Vector as u8VectorValue, f32Vector as f32VectorValue } from '../lib/data';

describe('Select Union Queries', () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map(ctx => ctx.deleteCollections()));
  });

  test('query select union', async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection('test', {});

    // upsert documents with different types in the same field
    const lsn = await ctx.client.collection(collection.name).upsert([
      { _id: '0', rank: 0, mixed: null },
      { _id: '1', rank: 1, mixed: 1 },
      { _id: '2', rank: 2, mixed: 2 },
      { _id: '3', rank: 3, mixed: 3 },
      { _id: '4', rank: 4, mixed: 4 },
      { _id: '5', rank: 5, mixed: 5.0 },
      { _id: '6', rank: 6, mixed: 6.0 },
      { _id: '7', rank: 7, mixed: true },
      { _id: '8', rank: 8, mixed: 'hello' },
      { _id: '9', rank: 9, mixed: u8VectorValue([1, 2, 3]) },
      { _id: '10', rank: 10, mixed: f32VectorValue([1.0, 2.0, 3.0]) },
      { _id: '11', rank: 11, mixed: [1, 2, 3] },
      { _id: '12', rank: 12, mixed: [1.0, 2.0, 3.0] },
      { _id: '13', rank: 13, mixed: u8VectorValue([1, 2, 3]) },
    ]);

    // wait for writes to be flushed
    await ctx.client.collection(collection.name).count(lsn);

    const results = await ctx.client.collection(collection.name).query(
      select({ mixed: field('mixed') }).topK(field('rank'), 100, true)
    );

    // Verify we have all the documents
    expect(results).toEqual([
      { _id: '0', mixed: null },
      { _id: '1', mixed: 1 },
      { _id: '2', mixed: 2 },
      { _id: '3', mixed: 3 },
      { _id: '4', mixed: 4 },
      { _id: '5', mixed: 5.0 },
      { _id: '6', mixed: 6.0 },
      { _id: '7', mixed: true },
      { _id: '8', mixed: 'hello' },
      { _id: '9', mixed: [1, 2, 3] },
      { _id: '10', mixed: [1.0, 2.0, 3.0] },
      { _id: '11', mixed: [1, 2, 3] },
      { _id: '12', mixed: [1.0, 2.0, 3.0] },
      { _id: '13', mixed: [1, 2, 3] },
    ]);
  });
});