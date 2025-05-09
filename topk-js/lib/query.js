const { query } = require("../index");

module.exports.field = query.field;
module.exports.filter = query.filter;
module.exports.fn = {
  bm25Score: query.bm25Score,
  semanticSimilarity: query.semanticSimilarity,
  vectorDistance: query.vectorDistance,
};
module.exports.literal = query.literal;
module.exports.LogicalExpression = query.LogicalExpression;
module.exports.match = query.match;
module.exports.not = query.not;
module.exports.Query = query.Query;
module.exports.select = query.select;
module.exports.TextExpression = query.TextExpression;
