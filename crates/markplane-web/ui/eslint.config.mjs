import { defineConfig, globalIgnores } from "eslint/config";
import nextVitals from "eslint-config-next/core-web-vitals";
import nextTs from "eslint-config-next/typescript";

const eslintConfig = defineConfig([
  ...nextVitals,
  ...nextTs,
  {
    rules: {
      "no-restricted-imports": [
        "error",
        {
          paths: [
            {
              name: "next/link",
              message:
                "Use ProjectLink from @/components/project-link instead of next/link.",
            },
            {
              name: "next/navigation",
              importNames: ["useRouter"],
              message:
                "Use useProjectRouter from @/lib/project-prefix instead of useRouter.",
            },
          ],
        },
      ],
    },
  },
  {
    // Only the Project-aware wrappers may import the restricted APIs.
    files: ["src/components/project-link.tsx", "src/lib/project-prefix.ts"],
    rules: {
      "no-restricted-imports": "off",
    },
  },
  // Override default ignores of eslint-config-next.
  globalIgnores([
    // Default ignores of eslint-config-next:
    ".next/**",
    "out/**",
    "build/**",
    "next-env.d.ts",
  ]),
]);

export default eslintConfig;
