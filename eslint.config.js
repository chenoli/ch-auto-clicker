import js from "@eslint/js";
import checkFile from "eslint-plugin-check-file";
import perfectionist from "eslint-plugin-perfectionist";
import reactHooks from "eslint-plugin-react-hooks";
import globals from "globals";
import tseslint from "typescript-eslint";

import { splitInlineTypeImports } from "./tools/eslint/rules/split-inline-type-imports.mjs";

const appContexts = ["components", "dev", "features", "i18n", "ipc", "store", "styles"];

export default tseslint.config(
  {
    ignores: [
      "dist",
      "target",
      "coverage",
      "src-tauri",
      "refs",
      "ui/src/ipc/bindings.ts",
      "playwright-report",
      "test-results",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["**/*.{ts,tsx,js,mjs}"],
    languageOptions: { globals: { ...globals.browser, ...globals.node } },
    plugins: {
      "check-file": checkFile,
      local: { rules: { "split-inline-type-imports": splitInlineTypeImports } },
      perfectionist,
      "react-hooks": reactHooks,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "error",
      "no-console": "error",
      "no-var": "error",
      "prefer-const": "error",
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/no-non-null-assertion": "warn",
      "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_", varsIgnorePattern: "^_" }],
      "@typescript-eslint/consistent-type-imports": [
        "error",
        { prefer: "type-imports", fixStyle: "separate-type-imports" },
      ],
      "local/split-inline-type-imports": "error",
      "check-file/filename-naming-convention": [
        "error",
        { "**/*.{ts,tsx,js,mjs}": "KEBAB_CASE" },
        { ignoreMiddleExtensions: true },
      ],
      "perfectionist/sort-imports": [
        "error",
        {
          type: "alphabetical",
          order: "asc",
          ignoreCase: true,
          newlinesBetween: 1,
          groups: [
            "side-effect-style",
            "side-effect",
            "react",
            ["default-external", "default-builtin"],
            ["named-external", "named-builtin"],
            ...appContexts,
            ["default-parent", "default-sibling", "default-index"],
            ["named-parent", "named-sibling", "named-index"],
            ["type-external", "type-internal", "type-builtin", "type-parent", "type-sibling", "type-index"],
          ],
          customGroups: [
            { groupName: "react", elementNamePattern: "^react(-dom)?$", modifiers: ["value"] },
            ...appContexts.map((ctx) => ({
              groupName: ctx,
              elementNamePattern: `^(\\.\\./)+(src/)?${ctx}(/|$)`,
              modifiers: ["value"],
            })),
          ],
        },
      ],
    },
  },
  {
    // Command-line scripts report progress on the terminal.
    files: ["tools/**", "ui/tests/**"],
    rules: { "no-console": "off" },
  },
);
