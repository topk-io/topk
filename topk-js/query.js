const { query } = require("./index");

module.exports.bm25Score = query.bm25Score;
module.exports.field = query.field;
module.exports.literal = query.literal;
module.exports.match = query.match;
module.exports.select = query.select;
module.exports.semanticSimilarity = query.semanticSimilarity;
module.exports.vectorDistance = query.vectorDistance;
