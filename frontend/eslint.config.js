import js from '@eslint/js';
import svelte from 'eslint-plugin-svelte';
import ts from '@typescript-eslint/eslint-plugin';
import tsParser from '@typescript-eslint/parser';

export default [
  js.configs.recommended,
  ...svelte.configs['flat/recommended'],
  {
    files: ['**/*.ts'],
    languageOptions: {
      parser: tsParser,
    },
    plugins: {
      '@typescript-eslint': ts,
    },
    rules: {
      ...ts.configs.recommended.rules,
      'indent': 'off',
      'linebreak-style': 'off',
      'quotes': 'off',
      'semi': 'off',
      'comma-dangle': 'off',
      'no-multi-spaces': 'off',
      'key-spacing': 'off',
      'object-curly-spacing': 'off',
      'array-bracket-spacing': 'off',
      'space-in-parens': 'off',
      'no-trailing-spaces': 'off',
      'eol-last': 'off',
    },
  },
  {
    files: ['**/*.svelte'],
    rules: {
      'svelte/indent': 'off',
      'svelte/no-extra-reactive-curlies': 'warn', // 保留有用的检查
      'svelte/valid-compile': 'error', // 保留编译检查
    },
  },
];
