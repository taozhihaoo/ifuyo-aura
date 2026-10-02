/// <reference types="vitest/config" />
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Phase 0：最小可运行前端壳。Aurora / Three.js 等按里程碑（M6+）引入。
export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'node',
  },
});
