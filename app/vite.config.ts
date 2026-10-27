import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

// The Tauri host serves the built files from dist/; the dev server keeps a fixed port so
// `npm run tauri dev` finds it.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'es2022',
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,tsx}'],
  },
})
