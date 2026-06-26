import { defineConfig } from 'oxlint';

export default defineConfig({
  plugins: ['eslint', 'unicorn', 'typescript', 'oxc', 'import', 'jsdoc'],
  categories: {},
  rules: {
    'no-console': ['error', { allow: ['error', 'info', 'trace', 'warn', 'time', 'timeEnd'] }],
    'typescript/no-explicit-any': 'error',
  },
  settings: {
    jsdoc: {
      ignorePrivate: false,
      ignoreInternal: false,
      ignoreReplacesDocs: true,
      overrideReplacesDocs: true,
      augmentsExtendsReplacesDocs: false,
      implementsReplacesDocs: false,
      exemptDestructuredRootsFromChecks: false,
      tagNamePreference: {},
    },
  },
  env: {
    builtin: true,
  },
  globals: {},
  ignorePatterns: ['server/migrations/**/*'],
});
