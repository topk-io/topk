import { field, literal, match, select, bm25Score, vectorDistance } from '../lib/query';
import { text, keywordIndex, f32Vector, vectorIndex, int } from '../lib/schema';
import { newProjectContext, ProjectContext } from './setup';

describe('Select Queries', () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map(ctx => ctx.deleteCollections()));
  });

  test('query select literal', async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection('books', {
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: '1984', title: '1984', published_year: 1949 },
      { _id: 'pride', title: 'Pride and Prejudice', published_year: 1813 },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({ literal: literal(1.0) })
        .filter(field('title').eq('1984'))
        .topK(field('published_year'), 100, true)
    );

    expect(results).toEqual([{ _id: '1984', literal: 1.0 }]);
  });

  test('query select non-existing field', async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection('books', {
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: '1984', title: '1984', published_year: 1949 },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({ literal: field('non_existing_field') })
        .filter(field('title').eq('1984'))
        .topK(field('published_year'), 100, true)
    );

    expect(results).toEqual([{ _id: '1984' }]);
  });

  test('query topk limit', async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection('books', {
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: 'pride', published_year: 1813 },
      { _id: 'moby', published_year: 1851 },
      { _id: 'gatsby', published_year: 1925 },
      { _id: '1984', published_year: 1949 },
    ]);

    let results = await ctx.client.collection(collection.name).query(
      select({}).topK(field('published_year'), 3, true)
    );
    expect(results.length).toBe(3);

    results = await ctx.client.collection(collection.name).query(
      select({}).topK(field('published_year'), 2, true)
    );
    expect(results.length).toBe(2);

    results = await ctx.client.collection(collection.name).query(
      select({}).topK(field('published_year'), 1, true)
    );
    expect(results.length).toBe(1);
  });

  test('query topk asc', async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection('books', {
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: 'pride', published_year: 1813 },
      { _id: 'moby', published_year: 1851 },
      { _id: 'gatsby', published_year: 1925 },
      { _id: '1984', published_year: 1949 },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({ published_year: field('published_year') })
        .topK(field('published_year'), 3, true)
    );

    expect(results).toEqual([
      { _id: 'pride', published_year: 1813 },
      { _id: 'moby', published_year: 1851 },
      { _id: 'gatsby', published_year: 1925 },
    ]);
  });

  test('query topk desc', async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection('books', {
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: 'harry', published_year: 1997 },
      { _id: 'alchemist', published_year: 1988 },
      { _id: 'mockingbird', published_year: 1960 },
      { _id: '1984', published_year: 1949 },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({ published_year: field('published_year') })
        .topK(field('published_year'), 3, false)
    );

    expect(results).toEqual([
      { _id: 'harry', published_year: 1997 },
      { _id: 'alchemist', published_year: 1988 },
      { _id: 'mockingbird', published_year: 1960 },
    ]);
  });

  test('query select bm25 score', async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection('books', {
      title: text().required().index(keywordIndex()),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: 'pride', title: 'Pride and Prejudice' },
      { _id: '1984', title: '1984' },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({ bm25_score: bm25Score() })
        .filter(match('pride'))
        .topK(field('bm25_score'), 100, true)
    );

    expect(new Set(results.map(doc => doc._id))).toEqual(new Set(['pride']));
  });

  test('query select vector distance', async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection('books', {
      summary_embedding: f32Vector(16).required().index(vectorIndex({ metric: 'euclidean' })),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: '1984', summary_embedding: [1.0, ...Array(15).fill(0)] },
      { _id: 'mockingbird', summary_embedding: [1.5, ...Array(15).fill(0)] },
      { _id: 'pride', summary_embedding: [2.0, ...Array(15).fill(0)] },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({
        summary_distance: vectorDistance('summary_embedding', [2.0, ...Array(15).fill(0)]),
      }).topK(field('summary_distance'), 3, true)
    );

    expect(new Set(results.map(doc => doc._id))).toEqual(new Set(['1984', 'mockingbird', 'pride']));
  });

  test('query select null field', async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection('test', {});

    await ctx.client.collection(collection.name).upsert([
      { _id: '1984', a: null },
      { _id: 'pride' },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({ a: field('a'), b: literal(1) })
        .topK(field('b'), 100, true)
    );

    // Assert that `a` is null for all documents, even when not specified when upserting
    expect(new Set(results.map(doc => doc.a))).toEqual(new Set([null, null]));
  });
});