import { defineConfig } from 'vitest/config'
import { resolve } from 'path'

// Tests de la logique pure (formatage, camemberts, paliers) — environnement
// node, pas de DOM nécessaire.
export default defineConfig({
  test: {
    environment: 'node',
    include: ['src/**/*.spec.ts'],
  },
  resolve: {
    alias: { '@': resolve(__dirname, 'src') },
  },
})
