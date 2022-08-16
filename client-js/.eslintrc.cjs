/* eslint-env node */
require("@rushstack/eslint-patch/modern-module-resolution");

module.exports = {
    root: true,
    extends: [
        "plugin:vue/vue3-essential",
        "eslint:recommended",
        "@vue/eslint-config-typescript/recommended",
        "@vue/eslint-config-prettier",
    ],
    parserOptions: {
        ecmaVersion: "latest",
    },
    rules: {
        "max-len": ["error", { code: 120, comments: 120 }],
        "prettier/prettier": [
            "error",
            {
                printWidth: 120,
                arrowParens: "avoid",
            },
        ],
    },
    overrides: [
        {
            files: ["*.vue"],
            rules: {
                "no-undef": "off",
            },
        },
        {
            files: ["*.vue", "*.ts"],
            rules: {
                "@typescript-eslint/no-non-null-assertion": "off",
            },
        },
    ],
};
