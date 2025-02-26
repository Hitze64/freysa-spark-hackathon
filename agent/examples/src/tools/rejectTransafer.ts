import { InferSchemaType, Tool, ToolSchema } from "@/common/tools/types"

const rejectTransferSchema = {
  type: "function",
  function: {
    name: "rejectTransfer",
    description: "Reject the money transfer request and provide explanation",
    parameters: {
      type: "object",
      properties: {
        explanation: {
          type: "string",
          description: "Explanation for why the money transfer is rejected",
        },
      },
      required: ["explanation"],
    },
  },
} as const satisfies ToolSchema

type RejectTransferInput = InferSchemaType<typeof rejectTransferSchema>

const RejectTransferTool: Tool<RejectTransferInput> = {
  name: rejectTransferSchema.function.name,
  schema: rejectTransferSchema,
  execute: async (input: RejectTransferInput) => {
    return `Rejected money transfer with explanation: ${input.explanation}`
  },
}

export { RejectTransferTool }