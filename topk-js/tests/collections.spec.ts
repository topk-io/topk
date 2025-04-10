import { EmbeddingDataType, f32Vector, int, keywordIndex, semanticIndex, text, VectorDistanceMetric, vectorIndex } from '../index';
import { newProjectContext, ProjectContext } from './setup';

describe('Collections', () => {
  let ctx: ProjectContext;

  beforeEach(() => {
    ctx = newProjectContext();
  });

  afterEach(async () => {
    await ctx.client.collections().delete(ctx.scope('test-collection'));
  });

  test('create collection', async () => {
    const schema = {
      title: text().required().index(keywordIndex()),
      title_embedding: f32Vector(1536)
        .required()
        .index(vectorIndex({ metric: VectorDistanceMetric.Euclidean })),
      summary: text()
        .required()
        .index(semanticIndex({ model: "dummy", embeddingType: EmbeddingDataType.Float32 })),
      published_year: int().required(),
    };

    const collection = await ctx.client.collections().create(
      ctx.scope('books'),
      schema
    );

    expect(collection.name).toBe(ctx.scope('books'));
    expect(collection.schema).toEqual(schema);
  });

  test('create collection with all data types', async () => {
    const { bool, bytes, binaryVector, float, u8Vector } = await import('../index');

    const schema = {
      text: text(),
      int: int(),
      float: float(),
      bool: bool(),
      vector: f32Vector(1536),
      float_vector: f32Vector(1536),
      byte_vector: u8Vector(1536),
      binary_vector: binaryVector(1536),
      bytes: bytes(),
    };

    const collection = await ctx.client.collections().create(
      ctx.scope('books'),
      schema
    );

    expect(collection.name).toBe(ctx.scope('books'));
    expect(collection.schema).toEqual(schema);
  });

  test('incorrect schema', async () => {
    await expect(
      ctx.client.collections().create(
        ctx.scope('books'),
        { name: text().index(vectorIndex({ metric: VectorDistanceMetric.Cosine })) }
      )
    ).rejects.toThrow();
  });

  test('list collections', async () => {
    // Note: All tests run within the same project,
    // so list of collections is shared across tests.

    const a = await ctx.client.collections().create(ctx.scope('books'), {});
    const collections1 = await ctx.client.collections().list();
    expect(collections1).toContainEqual(a);

    const b = await ctx.client.collections().create(ctx.scope('books2'), {});
    const collections2 = await ctx.client.collections().list();
    expect(collections2).toContainEqual(a);
    expect(collections2).toContainEqual(b);

    const c = await ctx.client.collections().create(ctx.scope('books3'), {});
    const collections3 = await ctx.client.collections().list();
    expect(collections3).toContainEqual(a);
    expect(collections3).toContainEqual(b);
    expect(collections3).toContainEqual(c);
  });

  test('get collections', async () => {
    // get non-existent collection
    await expect(
      ctx.client.collections().get(ctx.scope('foo'))
    ).rejects.toThrow();

    // create collection
    await ctx.client.collections().create(ctx.scope('foo'), {});

    // get collection
    const collection = await ctx.client.collections().get(ctx.scope('foo'));

    // assert collection
    expect(collection.name).toBe(ctx.scope('foo'));
  });

  test('delete collection', async () => {
    const collectionsBeforeCreate = await ctx.client.collections().list();
    expect(collectionsBeforeCreate.map(c => c.name)).not.toContain(ctx.scope('books'));

    await ctx.client.collections().create(ctx.scope('books'), {});

    const collectionsAfterCreate = await ctx.client.collections().list();
    expect(collectionsAfterCreate.map(c => c.name)).toContain(ctx.scope('books'));

    await ctx.client.collections().delete(ctx.scope('books'));

    const collectionsAfterDelete = await ctx.client.collections().list();
    expect(collectionsAfterDelete.map(c => c.name)).not.toContain(ctx.scope('books'));
  });

  test('delete non-existent collection', async () => {
    await expect(
      ctx.client.collections().delete(ctx.scope('books'))
    ).rejects.toThrow();
  });
});