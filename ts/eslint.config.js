import { defineConfig } from "eslint/config";
import v3xlint from "eslint-plugin-v3xlabs";

export default defineConfig([
    {
        ignores: [
            "**/dist/**",
            "**/node_modules/**",
            "**/*.js",
            "**/*.gen.ts",
        ],
    },
    ...v3xlint.configs.recommended,
    {
        files: ["**/test/**"],
        rules: {
            "@typescript-eslint/no-namespace": "off",
        },
    },
    {
        rules: {
            "unicorn/no-useless-undefined": "off",
            "unicorn/no-null": "off",
            "unicorn/name-replacements": "off",
            "unicorn/prefer-iterator-to-array": "off",
            "unicorn/no-declarations-before-early-exit": "off",
            "unicorn/no-unreadable-for-of-expression": "off",
            "unicorn/no-break-in-nested-loop": "off",
            "unicorn/no-array-reverse": "off",
            "@stylistic/indent": "off",
            "@stylistic/type-named-tuple-spacing": "off",
            "@stylistic/operator-linebreak": "off",
            "import/no-default-export": "off",
            "sonarjs/no-nested-functions": "off",
        },
    },
]);
