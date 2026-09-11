/** @type {import('ts-jest').JestConfigWithTsJest} **/
module.exports = {
  testEnvironment: "node",

  transform: {
    "^.+\\.tsx?$": ["ts-jest", {}],
  },

  // 10 seconds default timeout
  testTimeout: 10000,

  // the suite runs against one emulator, and saturating it makes reads wait past their lsn
  // deadline; fewer workers is both greener and faster
  maxWorkers: 2,
};
