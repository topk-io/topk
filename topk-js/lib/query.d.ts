import { query, query_fn } from "../index";

export declare const field: typeof query.field;
export declare const filter: typeof query.filter;
export declare const fn: {
  bm25Score: typeof query_fn.bm25Score;
  semanticSimilarity: typeof query_fn.semanticSimilarity;
  vectorDistance: typeof query_fn.vectorDistance;
};
export declare const literal: typeof query.literal;
export declare const match: typeof query.match;
export declare const not: typeof query.not;
export declare const min: typeof query.min;
export declare const max: typeof query.max;
export declare const abs: typeof query.abs;
export declare const select: typeof query.select;

export declare const Query: typeof query.Query;
export declare const LogicalExpression: typeof query.LogicalExpression;
export declare const TextExpression: typeof query.TextExpression;
