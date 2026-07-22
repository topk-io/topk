import { query, query_fn, query_agg } from "../index";

export declare const field: typeof query.field;
export declare const filter: typeof query.filter;
export declare const fn: {
  bm25Score: typeof query_fn.bm25Score;
  multiVectorDistance: typeof query_fn.multiVectorDistance;
  semanticSimilarity: typeof query_fn.semanticSimilarity;
  vectorDistance: typeof query_fn.vectorDistance;
};
export declare const agg: {
  count: typeof query_agg.count;
  sum: typeof query_agg.sum;
  min: typeof query_agg.min;
  max: typeof query_agg.max;
  avg: typeof query_agg.avg;
};
export declare const groupBy: typeof query.groupBy;
export declare const literal: typeof query.literal;
export declare const match: typeof query.match;
export declare const matchTokens: typeof query.matchTokens;
export declare const not: typeof query.not;
export declare const min: typeof query.min;
export declare const max: typeof query.max;
export declare const abs: typeof query.abs;
export declare const all: typeof query.all;
export declare const any: typeof query.any;
export declare const select: typeof query.select;

export declare const Query: typeof query.Query;
export declare const LogicalExpression: typeof query.LogicalExpression;
export declare const AggregateExpression: typeof query.AggregateExpression;
export declare const TextExpression: typeof query.TextExpression;
