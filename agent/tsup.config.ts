import { defineConfig } from "tsup"

export default defineConfig({
  entry: ["src/index.ts", "src/common/storage/index.ts"],
  format: ["cjs"],
  dts: true,
  sourcemap: true,
  clean: true,
  outDir: "dist",
  outExtension({ format }) {
    return { js: format === "cjs" ? ".cjs" : ".mjs" }
  },
  external: [],
  noExternal: ["node-fetch", "@grpc/grpc-js"],
  platform: "node",
  target: "node20",
  treeshake: true,
})
