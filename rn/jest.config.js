module.exports = {
  preset: '@react-native/jest-preset',
  // Fixtures live under __tests__/fixtures; only *.test files are suites.
  testMatch: ['<rootDir>/__tests__/**/*.test.[jt]s?(x)'],
  setupFiles: ['<rootDir>/jest.setup.js'],
};
