/* eslint-env jest */
// The catalogue installs itself as the translator of `@dcl/ui-native`, as it does in the app.
require('./src/i18n');

// The masked view is a native component shipped as untranspiled Flow: tests render its children.
jest.mock('@react-native-masked-view/masked-view', () => {
  const { View } = require('react-native');
  const { createElement } = require('react');
  return ({ maskElement: _mask, ...props }) => createElement(View, props);
});
