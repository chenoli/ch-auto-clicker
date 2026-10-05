import react from "@vitejs/plugin-react";

import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    include: ["ui/src/**/*.test.{ts,tsx}", "tools/**/*.test.ts"],
    setupFiles: ["ui/src/test-setup.ts"],
    coverage: {
      provider: "v8",
      include: ["ui/src/**/*.{ts,tsx}", "tools/**/*.ts"],
      exclude: [
        "**/*.test.{ts,tsx}",
        "ui/src/test-setup.ts",
        "ui/src/ipc/bindings.ts",
        "ui/src/styles/tokens.ts",
        "ui/src/entries/**",
        "tools/gen-tokens/gen-tokens.ts",
        "tools/eslint/**",
      ],
      thresholds: { lines: 80, branches: 80, functions: 80, statements: 80 },
    },
  },
});
