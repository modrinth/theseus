import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import alias from '@rollup/plugin-alias'
import { resolve } from 'path'
import eslint from 'vite-plugin-eslint'
import svgLoader from 'vite-svg-loader'
import { icuMessages } from '@vintl/unplugin/vite'
import virtual from '@rollup/plugin-virtual'
import { basename } from 'pathe'
import { globIterateSync } from 'glob'

const projectRootDir = resolve(__dirname)

/** RegExp that matches our special import parameter for locale files. */
const messagesParamRegex = /(\?|&)icu-messages(&|$)/

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    vue(),
    alias({
      entries: [
        {
          find: '@',
          replacement: resolve(projectRootDir, 'src'),
        },
      ],
    }),
    svgLoader({
      svgoConfig: {
        plugins: [
          {
            name: 'preset-default',
            params: {
              overrides: {
                removeViewBox: false,
              },
            },
          },
        ],
      },
    }),
    eslint(),
    // Process ICU MessageFormat messages using VIntl Unplugin
    icuMessages({
      filter(id) {
        const paramsIndex = id.indexOf('?')
        return (
          // Checking that ID has parameters at all, and whether they contain our special marker parameter.
          paramsIndex !== -1 && messagesParamRegex.test(id.slice(paramsIndex))
        )
      },
      // Wrap Vite's built-in JSON plugin to avoid conflicting pre-processing.
      pluginsWrapping: true,
    }),
    virtual({
      '@modrinth/theseus/locales/index.js': (() => {
        let output = 'export const localeDefinitions = Object.create(null);\n'

        const localeDirsIt = globIterateSync('node_modules/omorphia/locales/*', {
          cwd: __dirname,
          absolute: true,
        })

        for (const localeDir of localeDirsIt) {
          const tag = basename(localeDir)

          output += `localeDefinitions[${JSON.stringify(tag)}] = {\n`
          output += '\tasync importFunction() {\n'

          output += `\t\tconst messages = Object.create(null);\n`
          output += `\t\tconst resources = Object.create(null);\n`

          const filesIt = globIterateSync('*', {
            cwd: localeDir,
            absolute: true,
          })

          for (const filePath of filesIt) {
            const fileName = basename(filePath)

            if (fileName === 'index.json') {
              const importPath = JSON.stringify(`${filePath}?icu-messages`)
              output += `\t\tObject.assign(messages, await import(${importPath}).then((mod) => mod['default']));\n`
            }
          }

          output += '\t\treturn { messages, resources }\n'
          output += '\t},\n'
          output += '}\n'
        }

        return output
      })(),
    }),
  ],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  // prevent vite from obscuring rust errors
  clearScreen: false,
  // tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
  },
  // to make use of `TAURI_DEBUG` and other env variables
  // https://tauri.studio/v1/api/config#buildconfig.beforedevcommand
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    // Tauri supports es2021
    target: process.env.TAURI_PLATFORM == 'windows' ? 'chrome105' : 'safari13',
    // don't minify for debug builds
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    // produce sourcemaps for debug builds
    sourcemap: !!process.env.TAURI_DEBUG,
    commonjsOptions: {
      esmExternals: true,
    },
  },
})
