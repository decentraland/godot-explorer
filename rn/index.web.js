import { AppRegistry } from 'react-native';
import App from './App';
import { fonts } from './src/ds/tokens';

// The native builds ship Inter in the app bundle; on web the families are declared here.
const FACES = {
  [fonts.regular]: require('./android/app/src/rnassets/fonts/Inter-Regular.ttf'),
  [fonts.medium]: require('./android/app/src/rnassets/fonts/Inter-Medium.ttf'),
  [fonts.semibold]: require('./android/app/src/rnassets/fonts/Inter-SemiBold.ttf'),
  [fonts.bold]: require('./android/app/src/rnassets/fonts/Inter-Bold.ttf'),
};
const style = document.createElement('style');
style.textContent = Object.entries(FACES)
  .map(
    ([family, url]) => `@font-face{font-family:"${family}";src:url("${url}")}`,
  )
  .join('');
document.head.appendChild(style);

AppRegistry.registerComponent('main', () => App);
AppRegistry.runApplication('main', {
  rootTag: document.getElementById('root'),
});
