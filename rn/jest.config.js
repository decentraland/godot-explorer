module.exports = {
  preset: '@react-native/jest-preset',
  // Storybook ships ES modules only (gallery-stories.test.ts imports its id helper).
  transformIgnorePatterns: [
    'node_modules/(?!((jest-)?react-native|@react-native(-community)?|storybook)/)',
  ],
};
