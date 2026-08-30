import { defineConfig } from 'eslint/config'
import tseslint from '@electron-toolkit/eslint-config-ts'
import eslintConfigPrettier from '@electron-toolkit/eslint-config-prettier'
import eslintPluginVue from 'eslint-plugin-vue'
import vueParser from 'vue-eslint-parser'

export default defineConfig(
  {
    ignores: [
      '**/node_modules',
      '**/dist',
      '**/out',
      '**/reference',
      'scripts',
      // 复审材料包：里面是 scripts/ 下文件的副本，交付给裁判用。
      // 原文件已由上面的 'scripts' 豁免，副本同样不该进 lint。
      '_review',
      // 静态资源目录：startup-diagnostics.js 是刻意写成 ES5 传统脚本的（要先于
      // ES Module 入口执行，见文件头注释），套 TS 规则会要求它标注返回类型——
      // 那是纯 JS 文件做不到的事。它不参与打包，由 index.html 直接引用。
      'src/renderer/public/**',
      // vendored 歌词引擎（music-lyric-kit / -player 上游源码，见 tools/vendor-lyric.mjs）：
      // 保持与上游逐字节可对照，不套本项目的 prettier/lint 规约，否则每次升级都是满屏格式 diff
      'src/renderer/src/lyric/**'
    ]
  },
  tseslint.configs.recommended,
  eslintPluginVue.configs['flat/recommended'],
  {
    files: ['**/*.vue'],
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        ecmaFeatures: {
          jsx: true
        },
        extraFileExtensions: ['.vue'],
        parser: tseslint.parser
      }
    }
  },
  {
    files: ['**/*.{ts,mts,tsx,vue}'],
    rules: {
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_', caughtErrorsIgnorePattern: '^_' }
      ],
      'vue/require-default-prop': 'off',
      'vue/multi-word-component-names': 'off',
      'vue/block-lang': [
        'error',
        {
          script: {
            lang: 'ts'
          }
        }
      ]
    }
  },
  {
    // vendored 第三方源码（AMLL bg-render）：保持上游原样，不套本项目显式返回类型规约
    files: ['src/renderer/src/bg-render/**/*.ts'],
    rules: {
      '@typescript-eslint/explicit-function-return-type': 'off'
    }
  },
  {
    // 构建期一次性脚本（vendor / 校验），不进产物，无需显式返回类型
    files: ['tools/**/*.{mjs,mts}'],
    rules: {
      '@typescript-eslint/explicit-function-return-type': 'off'
    }
  },
  eslintConfigPrettier
)