import { Tool, ToolSchema, InferSchemaType } from "./types"

/**
 * Schema definition for the completion tool
 * Used to signal that all tool usage has been completed
 */
const schema = {
  type: "function",
  function: {
    name: "Completed",
    description:
      "Completion tool indicates that all tool usage has been completed.",
    parameters: {
      type: "object",
      properties: {},
      required: [],
    },
  },
} as const satisfies ToolSchema

/** Input type for the completion tool (empty object) */
type CompletionInput = InferSchemaType<typeof schema>

/**
 * Tool that signals the completion of all tool usage
 * Returns an empty string when executed
 */
export const CompletionTool: Tool<CompletionInput> = {
  name: "Completed",
  schema,
  execute: async (input: CompletionInput) => {
    return ""
  },
}
