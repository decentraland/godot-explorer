const HOSTS = '(decentraland\\.(org|zone)|dclexplorer\\.com)';
const message = 'Hosts come from core.urls()';

module.exports = {
  root: true,
  extends: '@react-native',
  overrides: [
    {
      files: ['src/**/*.{ts,tsx}', 'App.tsx'],
      excludedFiles: [
        '**/*mock*',
        '**/mock/**',
        'src/hud/simulator.ts',
        '__tests__/**',
        'src/core/native.ts',
      ],
      rules: {
        'no-restricted-syntax': [
          'error',
          {
            selector: `Literal[value=/${HOSTS}/]`,
            message,
          },
          {
            selector: `TemplateElement[value.raw=/${HOSTS}/]`,
            message,
          },
        ],
      },
    },
  ],
};
