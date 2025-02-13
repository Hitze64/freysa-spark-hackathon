import { Tool, ToolSchema, InferSchemaType } from "../types"
import { completions } from "./api"

const schema = {
  type: "function",
  function: {
    name: "Search",
    description: "Use Perplexity to search the web for information",
    parameters: {
      type: "object",
      properties: {
        userPrompt: {
          type: "string",
          description: "The search query or question to ask Perplexity",
        },
        recencyFilter: {
          type: "string",
          description: "How recent the search results should be",
          enum: ["month", "week", "day", "hour"],
        },
      },
      required: ["userPrompt"],
    },
  },
} as const satisfies ToolSchema

type SchemaInput = InferSchemaType<typeof schema>

export class PerplexitySearchTool implements Tool<SchemaInput> {
  name: string
  schema: ToolSchema
  model?: string

  constructor(model?: string) {
    this.name = schema.function.name
    this.schema = schema
    this.model = model

    const token = process.env.PERPLEXITY_API
    if (!token) {
      throw new Error("PERPLEXITY_API environment variable is not set")
    }
  }

  async execute(input: SchemaInput) {
    return completions({
      systemPrompt: "",
      userPrompt: input.userPrompt,
      model: this.model,
      //   recencyFilter: input.recencyFilter,
    })
  }
}
