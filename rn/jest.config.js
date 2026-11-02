module.exports = {
  preset: '@react-native/jest-preset',
  // Fixtures live under __tests__/fixtures; everything else the preset matches is a suite (a
  // `testMatch` override would silently drop tests added under packages/).
  testPathIgnorePatterns: ['/node_modules/', '<rootDir>/__tests__/fixtures/'],
  setupFiles: ['<rootDir>/jest.setup.js'],
};
