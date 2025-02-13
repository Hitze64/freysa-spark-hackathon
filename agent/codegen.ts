import type { CodegenConfig } from "@graphql-codegen/cli"

const config: CodegenConfig = {
  schema: "src/chat/schema.graphql",
  generates: {
    "src/chat/generated/graphql.ts": {
      plugins: ["typescript", "typescript-operations", "typescript-resolvers"],
      config: {
        scalars: {
          DateTime: "Date",
        },
        useIndexSignature: true,
        contextType: "any",
        resolverTypeWrapperSignature: "Promise<T> | T",
        makeResolverTypeCallable: true,
      },
    },
  },
}

export default config
