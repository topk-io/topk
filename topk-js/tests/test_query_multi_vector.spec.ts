import { matrix as matrixData, MatrixValueType } from "../lib/data";
import { field, fn, literal, select } from "../lib/query";
import { int, keywordIndex, matrix, multiVectorIndex, text } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

// Query vectors
const Q1 = [
  -0.4449, 1.3496, 0.6855, -0.7714, -0.0942, -0.7982, -0.4429, -0.5834, -0.7113, 1.009,
  1.1826, 0.5344, 0.0189, -0.2313,
];

const Q2 = [
  1.5269, -0.2615, -0.1201, -1.495, 0.5497, 0.1703, -0.4399, 1.8301, 0.6419, -1.8175,
  1.8999, -0.3407, 0.5301, -1.1665, -1.6396, 2.2458, 0.1597, 0.8082, 0.2963, 0.1538,
  1.3943,
];

class MultiVecDataset {
  static cast(valueType: string, matrixRows: number[][]): ReturnType<typeof matrixData> {
    if (valueType === "f32") {
      return matrixData(matrixRows, "f32");
    } else if (valueType === "f16") {
      return matrixData(matrixRows, "f16");
    } else if (valueType === "f8") {
      return matrixData(matrixRows, "f8");
    } else if (valueType === "u8") {
      // Convert f32 to u8: (abs(v) * 64.0).round() as u8
      const u8Rows = matrixRows.map((row) =>
        row.map((v) => Math.round(Math.abs(v) * 64.0))
      );
      return matrixData(u8Rows, "u8");
    } else if (valueType === "i8") {
      // Convert f32 to i8: (v * 64.0).round() and clip to i8 range
      const clipToI8 = (n: number): number => Math.max(Math.min(n, 127), -128);
      const i8Rows = matrixRows.map((row) =>
        row.map((v) => clipToI8(Math.round(v * 64.0)))
      );
      return matrixData(i8Rows, "i8");
    } else {
      throw new Error(`Unsupported value type: ${valueType}`);
    }
  }

  static schema(valueType: MatrixValueType) {
    return {
      title: text().required().index(keywordIndex()),
      published_year: int().required(),
      token_embeddings: matrix({ dimension: 7, valueType: valueType }).index(
        multiVectorIndex({ metric: "maxsim" })
      ),
    };
  }

  static docs(valueType: string) {
    const baseDocs = [
      {
        _id: "doc_0",
        title: "To Kill a Mockingbird",
        published_year: 1960,
        token_embeddings: this.cast(valueType, [
          [0.9719, 0.132, 0.5612, -1.1843, -0.2115, 0.1455, -1.6471],
          [-0.1054, 1.6053, -0.0901, 0.5288, -0.6347, 0.9521, -0.8853],
        ]),
      },
      {
        _id: "doc_1",
        title: "1984",
        published_year: 1949,
        token_embeddings: this.cast(valueType, [
          [0.4364, -0.4954, 0.3665, 1.5041, -1.4773, -0.701, -0.9732],
          [-1.2239, 1.7501, 0.4089, 2.0643, -1.3925, 0.4711, -0.6247],
        ]),
      },
      {
        _id: "doc_2",
        title: "Pride and Prejudice",
        published_year: 1813,
        token_embeddings: this.cast(valueType, [
          [-2.6447, 0.3202, -0.5956, 0.6756, 1.0693, -1.0891, 1.0181],
        ]),
      },
      {
        _id: "doc_3",
        title: "The Great Gatsby",
        published_year: 1925,
        token_embeddings: this.cast(valueType, [
          [0.1643, -0.2945, 1.3312, -0.3341, -0.3304, -0.029, -0.4426],
          [-0.0975, -0.3696, -0.4106, -0.451, 0.4149, 0.8296, 0.3084],
          [0.68, -0.182, -0.2652, -0.9707, -0.3433, 0.9671, -1.9293],
        ]),
      },
      {
        _id: "doc_4",
        title: "The Catcher in the Rye",
        published_year: 1951,
        token_embeddings: this.cast(valueType, [
          [0.8748, 0.9163, 1.5845, -1.303, 1.7739, 0.9365, 1.2679],
          [-0.6695, 0.5488, -1.0841, 0.3331, 0.5206, -1.2897, 0.6149],
        ]),
      },
      {
        _id: "doc_5",
        title: "Moby-Dick",
        published_year: 1851,
        token_embeddings: this.cast(valueType, [
          [-0.6367, -0.5482, -1.2782, 1.0357, 1.044, -1.7687, 0.1703],
          [-1.379, 0.0448, -0.7917, -1.693, -0.6001, 0.0598, 1.5035],
          [1.968, -0.8128, 0.7871, -1.2036, -0.6445, -0.0684, 0.3407],
        ]),
      },
      {
        _id: "doc_6",
        title: "The Hobbit",
        published_year: 1937,
        token_embeddings: this.cast(valueType, [
          [-0.4733, 0.5792, 0.1226, 0.4607, -0.3138, -0.2211, -0.1725],
          [1.0828, -0.9416, 0.0848, 1.5135, 1.0625, 0.5481, 0.1558],
          [0.71, -1.3281, 0.5986, -2.2235, -0.1252, -0.5943, 0.6521],
        ]),
      },
      {
        _id: "doc_7",
        title: "Harry Potter and the Sorcerer's Stone",
        published_year: 1997,
        token_embeddings: this.cast(valueType, [
          [-0.4046, -0.1552, 2.632, -0.5471, -0.1942, -0.731, -1.1103],
          [0.5813, 0.247, 0.0275, 0.0063, -2.4539, -0.2918, 1.1274],
          [1.0666, 0.5535, 1.184, 0.5897, 1.2976, 1.2298, 2.6738],
        ]),
      },
      {
        _id: "doc_8",
        title: "The Lord of the Rings: The Fellowship of the Ring",
        published_year: 1954,
        token_embeddings: this.cast(valueType, [
          [-0.2822, -0.4862, 2.0163, -1.4105, 2.1853, 0.583, 0.7119],
          [-1.7254, 0.3599, 0.2296, 0.1091, -0.6483, 0.3901, -0.9539],
          [-0.5296, -0.3046, 1.5027, 0.7712, -1.071, 0.7371, 0.1228],
          [1.7048, 0.182, 0.3116, 0.7806, 0.2414, -0.7322, -0.1204],
        ]),
      },
      {
        _id: "doc_9",
        title: "The Alchemist",
        published_year: 1988,
      },
    ];
    return baseDocs;
  }

  static async setup(ctx: ProjectContext, valueType: MatrixValueType) {
    const collection = await ctx.createCollection(
      `multi_vec_${valueType}`,
      this.schema(valueType)
    );

    const docsList = this.docs(valueType);
    // Upsert in chunks of 4
    let lsn = "";
    for (let i = 0; i < docsList.length; i += 4) {
      const chunk = docsList.slice(i, i + 4);
      lsn = await ctx.client.collection(collection.name).upsert(chunk);
    }

    const count = await ctx.client.collection(collection.name).count({ lsn });
    if (count !== docsList.length) {
      throw new Error(
        `Expected ${docsList.length} documents, got ${count}`
      );
    }

    return collection;
  }
}

function docIdsOrdered(result: Array<Record<string, any>>): string[] {
  return result.map((doc) => doc._id);
}

describe("Multi-Vector Queries", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test.each(["f32", "f16", "f8"] as const)(
    "query multi-vector float valueType=%s",
    async (valueType) => {
      const ctx = getContext();
      const collection = await MultiVecDataset.setup(ctx, valueType);

      const queries = [
        { q: Q1, expectedIds: ["doc_7", "doc_8", "doc_6"] },
        { q: Q2, expectedIds: ["doc_0", "doc_6", "doc_8"] },
      ];

      for (const { q, expectedIds } of queries) {
        // Convert flat list to matrix (2 rows x 7 cols for Q1, 3 rows x 7 cols for Q2)
        const numRows = Math.floor(q.length / 7);
        const matrixRows: number[][] = [];
        for (let i = 0; i < numRows; i++) {
          matrixRows.push(q.slice(i * 7, (i + 1) * 7));
        }
        const queryMatrix = MultiVecDataset.cast(valueType, matrixRows);

        const result = await ctx.client.collection(collection.name).query(
          select({
            title: field("title"),
            dist: fn.multiVectorDistance("token_embeddings", queryMatrix),
          }).topk(field("dist"), 3, false)
        );

        expect(result.length).toBe(3);
        expect(docIdsOrdered(result)).toEqual(expectedIds);
      }
    }
  );

  test("query multi-vector int u8", async () => {
    const ctx = getContext();
    const collection = await MultiVecDataset.setup(ctx, "u8");

    const queries = [
      { q: Q1, expectedIds: ["doc_1", "doc_4", "doc_6"] },
      { q: Q2, expectedIds: ["doc_1", "doc_2", "doc_4"] },
    ];

    for (const { q, expectedIds } of queries) {
      const numRows = Math.floor(q.length / 7);
      const matrixRows: number[][] = [];
      for (let i = 0; i < numRows; i++) {
        matrixRows.push(q.slice(i * 7, (i + 1) * 7));
      }
      const queryMatrix = MultiVecDataset.cast("u8", matrixRows);

      const result = await ctx.client.collection(collection.name).query(
        select({
          title: field("title"),
          dist: fn.multiVectorDistance("token_embeddings", queryMatrix),
        }).topk(field("dist"), 3, false)
      );

      expect(result.length).toBe(3);
      expect(docIdsOrdered(result)).toEqual(expectedIds);
    }
  });

  test("query multi-vector int i8", async () => {
    const ctx = getContext();
    const collection = await MultiVecDataset.setup(ctx, "i8");

    const queries = [
      { q: Q1, expectedIds: ["doc_7", "doc_8", "doc_6"] },
      { q: Q2, expectedIds: ["doc_0", "doc_6", "doc_5"] },
    ];

    for (const { q, expectedIds } of queries) {
      const numRows = Math.floor(q.length / 7);
      const matrixRows: number[][] = [];
      for (let i = 0; i < numRows; i++) {
        matrixRows.push(q.slice(i * 7, (i + 1) * 7));
      }
      const queryMatrix = MultiVecDataset.cast("i8", matrixRows);

      const result = await ctx.client.collection(collection.name).query(
        select({
          title: field("title"),
          dist: fn.multiVectorDistance("token_embeddings", queryMatrix),
        }).topk(field("dist"), 3, false)
      );

      expect(result.length).toBe(3);
      expect(docIdsOrdered(result)).toEqual(expectedIds);
    }
  });

  test("query multi-vector with filter", async () => {
    const ctx = getContext();
    const collection = await MultiVecDataset.setup(ctx, "f32");

    const queries = [
      { q: Q1, expectedIds: ["doc_7", "doc_6", "doc_1"] },
      { q: Q2, expectedIds: ["doc_0", "doc_6", "doc_5"] },
    ];

    for (const { q, expectedIds } of queries) {
      const numRows = Math.floor(q.length / 7);
      const matrixRows: number[][] = [];
      for (let i = 0; i < numRows; i++) {
        matrixRows.push(q.slice(i * 7, (i + 1) * 7));
      }
      const queryMatrix = MultiVecDataset.cast("f32", matrixRows);

      const result = await ctx.client.collection(collection.name).query(
        select({
          title: field("title"),
          dist: fn.multiVectorDistance("token_embeddings", queryMatrix),
        })
          .filter(field("_id").ne(literal("doc_8")))
          .topk(field("dist"), 3, false)
      );

      expect(result.length).toBe(3);
      expect(docIdsOrdered(result)).toEqual(expectedIds);
    }
  });

  test("query multi-vector with invalid dimension", async () => {
    const ctx = getContext();
    const collection = await MultiVecDataset.setup(ctx, "f32");

    // Use wrong dimension (2 instead of 7)
    const numRows = Math.floor(Q1.length / 2);
    const matrixRows: number[][] = [];
    for (let i = 0; i < numRows; i++) {
      matrixRows.push(Q1.slice(i * 2, (i + 1) * 2));
    }
    const queryMatrix = MultiVecDataset.cast("f32", matrixRows);

    await expect(
      ctx.client.collection(collection.name).query(
        select({
          title: field("title"),
          dist: fn.multiVectorDistance("token_embeddings", queryMatrix),
        }).topk(field("dist"), 3, false)
      )
    ).rejects.toThrow();
  });

  test("query multi-vector with invalid data type", async () => {
    const ctx = getContext();
    const collection = await MultiVecDataset.setup(ctx, "f32");

    // Use wrong data type (f16 instead of f32)
    const numRows = Math.floor(Q1.length / 7);
    const matrixRows: number[][] = [];
    for (let i = 0; i < numRows; i++) {
      matrixRows.push(Q1.slice(i * 7, (i + 1) * 7));
    }
    const queryMatrix = MultiVecDataset.cast("f16", matrixRows);

    await expect(
      ctx.client.collection(collection.name).query(
        select({
          title: field("title"),
          dist: fn.multiVectorDistance("token_embeddings", queryMatrix),
        }).topk(field("dist"), 3, false)
      )
    ).rejects.toThrow();
  });

  test("query multi-vector with empty query", async () => {
    expect(() => {
      fn.multiVectorDistance("token_embeddings", [])
    }).toThrow(/Multi-vector query must be a matrix value/);

    expect(() => {
      fn.multiVectorDistance("token_embeddings", [[]])
    }).toThrow(/Cannot create matrix from empty list/);
  });

  test("query multi-vector with missing index", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int().required(),
    });

    const numRows = Math.floor(Q1.length / 7);
    const matrixRows: number[][] = [];
    for (let i = 0; i < numRows; i++) {
      matrixRows.push(Q1.slice(i * 7, (i + 1) * 7));
    }
    const queryMatrix = MultiVecDataset.cast("f32", matrixRows);

    await expect(
      ctx.client.collection(collection.name).query(
        select({
          title: field("title"),
          dist: fn.multiVectorDistance("token_embeddings", queryMatrix),
        }).topk(field("dist"), 3, false)
      )
    ).rejects.toThrow();
  });

  test("query multi-vector list of lists f32", async () => {
    const ctx = getContext();
    const collection = await MultiVecDataset.setup(ctx, "f32");
    const numRows = Math.floor(Q1.length / 7);
    const matrixRows: number[][] = [];
    for (let i = 0; i < numRows; i++) {
      matrixRows.push(Q1.slice(i * 7, (i + 1) * 7));
    }
    // Pass raw list of lists, not wrapped in data.matrix()

    const result = await ctx.client.collection(collection.name).query(
      select({
        title: field("title"),
        dist: fn.multiVectorDistance("token_embeddings", matrixRows),
      }).topk(field("dist"), 3, false)
    );

    expect(result.length).toBe(3);
    expect(docIdsOrdered(result)).toEqual(["doc_7", "doc_8", "doc_6"]);
  });

  test.each(["f16", "f8", "u8", "i8"] as const)(
    "query multi-vector list of lists type mismatch valueType=%s",
    async (valueType) => {
      const ctx = getContext();
      const collection = await MultiVecDataset.setup(ctx, valueType);
      const numRows = Math.floor(Q1.length / 7);
      const matrixRows: number[][] = [];
      for (let i = 0; i < numRows; i++) {
        matrixRows.push(Q1.slice(i * 7, (i + 1) * 7));
      }

      await expect(
        ctx.client.collection(collection.name).query(
          select({
            title: field("title"),
            dist: fn.multiVectorDistance("token_embeddings", matrixRows),
          }).topk(field("dist"), 3, false)
        )
      ).rejects.toThrow();
    }
  );
});
