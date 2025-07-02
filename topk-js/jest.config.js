/** @type {import('ts-jest').JestConfigWithTsJest} **/
module.exports = {
  testEnvironment: "node",

  transform: {
    "^.+\\.tsx?$": ["ts-jest", {}],
  },

  // 10 seconds default timeout
  testTimeout: 10000,
};
