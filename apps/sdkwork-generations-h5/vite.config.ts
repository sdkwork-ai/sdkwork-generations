import { resolveBrowserDistOutDir } from '../../../../sdkwork-specs/tools/browser-dist-layout.mjs';

import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, path.dirname(fileURLToPath(import.meta.url)), '');
  return {
    define: {
      'process.env.SDKWORK_ACCESS_TOKEN': JSON.stringify(env.SDKWORK_ACCESS_TOKEN ?? ''),
    },
          plugins: [react()],
  server: {
    port: 5173,
    host: true
  },
  build: {
    outDir: resolveBrowserDistOutDir(resolveViteEnvironment(mode, env)),
    sourcemap: true
  }
  };
});
