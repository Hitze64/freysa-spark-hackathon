import { Tool, ToolSchema, InferSchemaType } from "./types"

const schema = {
  type: "function",
  function: {
    name: "plan",
    description:
      "Generates a plan to accomplish a task. When given a task planning should be done first.",
    parameters: {
      type: "object",
      properties: {
        plan: {
          type: "string",
          description: "Plan",
        },
      },
      required: ["plan"],
    },
  },
} as const satisfies ToolSchema

type PlanInput = InferSchemaType<typeof schema>

export const PlanTool: Tool<PlanInput> = {
  name: schema.function.name,
  schema,
  execute: async (input: PlanInput) => {
    return input.plan
  },
}
