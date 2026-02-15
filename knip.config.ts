import type { KnipConfig } from 'knip';

const config: KnipConfig = {
  entry: ['src/main.tsx', 'vite.config.ts'],
  project: ['src/**/*.{ts,tsx}'],
  ignore: ['src/routeTree.gen.ts'],
  ignoreDependencies: [],
};

export default config;