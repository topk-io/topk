import { EmbeddingDataType, VectorDistanceMetric } from '../index';
import { binaryVector, bool, bytes, f32Vector, float, int, semanticIndex, text, u8Vector, vectorIndex } from '../schema';
import { newProjectContext, ProjectContext } from './setup';

describe('Collections', () => {
  let ctx: ProjectContext;
  let collectionsCreated: string[] = [];

  beforeEach(() => {
    ctx = newProjectContext();
    collectionsCreated = [];
  });

  afterEach(async () => {
    // Only delete collections created within the test
    for (const collectionName of collectionsCreated) {
      try {
        await ctx.client.collections().delete(collectionName);
      } catch (error) {
        console.error(`Error deleting collection ${collectionName}:`, error);
      }
    }
  });

  // Helper function to create a collection and track it
  const createCollection = async (name: string, schema: any) => {
    const collection = await ctx.client.collections().create(name, schema);
    collectionsCreated.push(name);
    return collection;
  };

  test('create collection', async () => {
    const schema = {
      title: text(),
      title_embedding: f32Vector(1536)
        .required()
        .index(vectorIndex({ metric: "Euclidean" })),
      summary: text()
        .required()
        .index(semanticIndex({ model: "dummy", embeddingType: "Float32" })),
      published_year: int().required(),
    };

    const collection = await createCollection(ctx.scope('books'), schema);

    expect(collection.name).toBe(ctx.scope('books'));
    expect(collection.schema).toEqual(schema);
  });

  test('create collection with all data types', async () => {
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

    const collection = await createCollection(ctx.scope('books'), schema);

    expect(collection.name).toBe(ctx.scope('books'));
    expect(collection.schema).toEqual(schema);
  });

  test('incorrect schema', async () => {
    await expect(
      ctx.client.collections().create(
        ctx.scope('books'),
        { name: text().index(vectorIndex({ metric: "Cosine" })) }
      )
    ).rejects.toThrow();
    // No need to track this collection as it fails to create
  });

  test('list collections', async () => {
    // Note: All tests run within the same project,
    // so list of collections is shared across tests.

    const a = await createCollection(ctx.scope('books'), {});
    const collections1 = await ctx.client.collections().list();
    expect(collections1).toContainEqual(a);

    const b = await createCollection(ctx.scope('books2'), {});
    const collections2 = await ctx.client.collections().list();
    expect(collections2).toContainEqual(a);
    expect(collections2).toContainEqual(b);

    const c = await createCollection(ctx.scope('books3'), {});
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
    await createCollection(ctx.scope('foo'), {});

    // get collection
    const collection = await ctx.client.collections().get(ctx.scope('foo'));

    // assert collection
    expect(collection.name).toBe(ctx.scope('foo'));
  });

  test('delete collection', async () => {
    const collectionsBeforeCreate = await ctx.client.collections().list();
    expect(collectionsBeforeCreate.map(c => c.name)).not.toContain(ctx.scope('books'));

    await createCollection(ctx.scope('books'), {});

    const collectionsAfterCreate = await ctx.client.collections().list();
    expect(collectionsAfterCreate.map(c => c.name)).toContain(ctx.scope('books'));

    await ctx.client.collections().delete(ctx.scope('books'));
    // Remove from tracking since we manually deleted it
    collectionsCreated = collectionsCreated.filter(name => name !== ctx.scope('books'));

    const collectionsAfterDelete = await ctx.client.collections().list();
    expect(collectionsAfterDelete.map(c => c.name)).not.toContain(ctx.scope('books'));
  });

  test('delete non-existent collection', async () => {
    await expect(
      ctx.client.collections().delete(ctx.scope('books'))
    ).rejects.toThrow();
  });
});