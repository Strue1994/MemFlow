import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
var agentServiceTarget = 'http://localhost:3300';
export default defineConfig({
    plugins: [react()],
    server: {
        port: 5273,
        proxy: {
            '/api': {
                target: agentServiceTarget,
                changeOrigin: true,
                rewrite: function (path) { return path.replace(/^\/api/, ''); }
            }
        }
    }
});
