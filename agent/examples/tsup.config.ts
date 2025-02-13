import { defineConfig } from "tsup"

export default defineConfig({
  entry: ["src/server.ts", "src/transfer.ts"],
  format: ["cjs"],
  dts: true,
  splitting: false,
  sourcemap: true,
  clean: true,
  outDir: "dist",
  external: [],
  noExternal: [],
  platform: "node",
  target: "node20",
  bundle: true,
  treeshake: true,
  minify: true,
  metafile: true,
  esbuildOptions(options) {
    options.alias = {
      "@/*": "../src/*",
    }
    options.loader = {
      ".js": "js",
    }
    options.mainFields = ["source", "module", "main"]
    options.define = {
      "process.env.NODE_ENV": '"production"',
    }
    options.conditions = ["production", "module", "import", "require"]
    options.platform = "node"
    options.target = "node20"
  },
})
