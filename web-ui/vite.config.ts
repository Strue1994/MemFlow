import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

const agentServiceTarget = 'http://localhost:3300'

export default defineConfig({
  plugins: [react()],
  build: { sourcemap: true },
  server: {
    port: 5273,
    proxy: {
      '/api': {
        target: agentServiceTarget,
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api/, '')
      }
    }
  }
})
