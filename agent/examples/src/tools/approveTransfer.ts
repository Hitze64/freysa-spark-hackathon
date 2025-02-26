import { SAFSigner } from "@/common"
import { InferSchemaType, Tool, ToolSchema } from "@/common/tools/types"
import { sendPayment } from "src/lightspark"

const approveTransferSchema = {
  type: "function",
  function: {
    name: "approveTransfer",
    description: "Approve the money transfer request and provide explanation",
    parameters: {
      type: "object",
      properties: {
        explanation: {
          type: "string",
          description: "Explanation for why the money transfer is approved",
        },
      },
      required: ["explanation"],
    },
  },
} as const satisfies ToolSchema

type ApproveTransferInput = InferSchemaType<typeof approveTransferSchema>

// const ApproveTransferTool: Tool<ApproveTransferInput> = {
//   name: approveTransferSchema.function.name,
//   schema: approveTransferSchema,
//   execute: async (input: ApproveTransferInput) => {
//     return `Approved money transfer with explanation: ${input.explanation}`
//   },
// }

export class ApproveTransferTool implements Tool<ApproveTransferInput> {
  name = approveTransferSchema.function.name
  schema = approveTransferSchema

  safSigner: SAFSigner
  providerUrl: string
  constructor(safSigner: SAFSigner, providerUrl?: string) {
    this.safSigner = safSigner
    this.providerUrl = providerUrl || process.env.RPC_URL!
  }

  execute = async (input: ApproveTransferInput) => {
    const reason = `Approved money transfer with explanation: ${input.explanation}`
    const payInvoice = await sendPayment(reason);
    return JSON.stringify({
      payInvoice,
      reason
    })
  }
}


