import { defineConfig } from 'oxfmt';

export default defineConfig({
  singleQuote: true,
  quoteProps: 'consistent',
  sortImports: {
    ignoreCase: false,
    newlinesBetween: false,
    // "sortSideEffects": true,
    groups: [
      'type-import',
      'type-internal',
      'type-parent',
      'type-sibling',
      'type-index',
      // "side-effect",
      'value-builtin',
      'value-external',
      'value-internal',
      'value-parent',
      'value-sibling',
      'value-index',
      'unknown',
    ],
  },
  ignorePatterns: ['tauri/gen/**'],
});
