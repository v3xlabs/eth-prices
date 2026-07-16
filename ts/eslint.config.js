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
            "@stylistic/indent": "off",
            "@stylistic/type-named-tuple-spacing": "off",
            "import/no-default-export": "off",
            "unicorn/no-null": "off",
            "sonarjs/no-nested-functions": "off",
        },
    },
]);
