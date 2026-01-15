const { query, query_fn } = require("../index");

module.exports.field = query.field;
module.exports.filter = query.filter;
module.exports.fn = {
  bm25Score: query_fn.bm25Score,
  semanticSimilarity: query_fn.semanticSimilarity,
  multiVectorDistance: query_fn.multiVectorDistance,
  vectorDistance: query_fn.vectorDistance,
};
module.exports.literal = query.literal;
module.exports.LogicalExpression = query.LogicalExpression;
module.exports.match = query.match;
module.exports.not = query.not;
module.exports.min = query.min;
module.exports.max = query.max;
module.exports.abs = query.abs;
module.exports.all = query.all;
module.exports.any = query.any;
module.exports.Query = query.Query;
module.exports.select = query.select;
module.exports.TextExpression = query.TextExpression;
