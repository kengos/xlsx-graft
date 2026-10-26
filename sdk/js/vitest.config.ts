import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['test/**/*.test.ts'],
    coverage: {
      provider: 'v8',
      include: ['src/**/*.ts'],
      // Printed only: the gate runs on a copy of the checkout, and the report
      // would have nowhere to go.
      reporter: ['text'],
      thresholds: {
        lines: 100,
      },
    },
  },
});
