import { defineConfig } from 'vite'
import solid from 'vite-plugin-solid'

export default defineConfig({
  base: './',
  plugins: [solid()],
  build: {
    target: 'esnext',
    outDir: '../module/webroot',
    emptyOutDir: true,
    minify: 'esbuild',
  },
  server: {
    port: 5173,
    host: true,
  },
})
