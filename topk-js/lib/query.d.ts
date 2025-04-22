import { query } from "../index";

export declare const count: typeof query.count;
export declare const field: typeof query.field;
export declare const filter: typeof query.filter;
export declare const fn: {
  bm25Score: typeof query.bm25Score;
  semanticSimilarity: typeof query.semanticSimilarity;
  vectorDistance: typeof query.vectorDistance;
};
export declare const literal: typeof query.literal;
export declare const LogicalExpression: typeof query.LogicalExpression;
export declare type LogicalExpressionUnion =
  import("../index").query.LogicalExpressionUnion;
export declare const match: typeof query.match;
export declare const Query: typeof query.Query;
export declare const select: typeof query.select;
export declare const TextExpression: typeof query.TextExpression;
export declare const topk: typeof query.topk;
