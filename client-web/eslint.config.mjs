import next from "eslint-config-next/core-web-vitals";

// ESLint 9 flat config. eslint-config-next v16 ships a native flat config
// array, so we spread it directly (no FlatCompat bridge needed).
const eslintConfig = [...next, { ignores: [".next/**"] }];

export default eslintConfig;
