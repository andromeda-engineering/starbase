import js from "@eslint/js";
import stylistic from "@stylistic/eslint-plugin";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    plugins: {
      "@stylistic": stylistic
    },
    languageOptions: {
      globals: {
        ...globals.node
      }
    },
    rules: {
      "@stylistic/semi": ["error", "always"],
      "@stylistic/quotes": ["error", "double"],
      "@stylistic/indent": ["error", 2],
      "@stylistic/comma-dangle": ["error", "never"]
    }
  },
  {
    ignores: [
      "**/node_modules/**",
      "**/dist/**",
      "**/target/**",
      "**/coverage/**",
      "**/.turbo/**"
    ]
  }
);
