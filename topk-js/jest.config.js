/** @type {import('ts-jest').JestConfigWithTsJest} **/
module.exports = {
  testEnvironment: "node",

  transform: {
    "^.+\\.tsx?$": ["ts-jest", {}],
  },

  // 10 seconds default timeout
  testTimeout: 10000,

  // Exclude `build` directory (tests get compiled to `build/` directory to `*.js`)
  modulePathIgnorePatterns: ["build/"],
};
