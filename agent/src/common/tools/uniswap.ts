import { Tool, ToolSchema, InferSchemaType } from "./types"
import { buildUniswapSwapTx } from "../transactions/swap"
import {
  SAFSigner,
  PrivateKeySignerConfig,
} from "../transactions/executeTransaction"

const uniswapSchema = {
  type: "function",
  function: {
    name: "create_swap",
    description: "Create a swap transaction on Uniswap V2.",
    parameters: {
      type: "object",
      properties: {
        token_input: {
          type: "string",
          description: "Token Input Address",
        },
        token_output: {
          type: "string",
          description: "Token Output Address",
        },
        amount_in: {
          type: "number",
          description: "Amount of input tokens to swap",
        },
      },
      required: ["token_input", "token_output", "amount_in"],
    },
  },
} as const satisfies ToolSchema

type UniswapInput = InferSchemaType<typeof uniswapSchema>

export class UniswapTool implements Tool<UniswapInput> {
  name: string = uniswapSchema.function.name
  schema: ToolSchema = uniswapSchema

  safSigner: SAFSigner
  uniswapRouterAddress: string
  providerUrl: string
  constructor(
    safSigner: SAFSigner,
    routerAddress?: string,
    providerUrl?: string
  ) {
    this.safSigner = safSigner
    this.uniswapRouterAddress =
      routerAddress || "0x4752ba5DBc23f44D87826276BF6Fd6b1C372aD24"
    this.providerUrl = providerUrl || process.env.RPC_URL!
  }

  async execute(input: UniswapInput) {
    const walletAddress = await this.safSigner.getWalletAddress()
    if (!walletAddress) {
      throw new Error("Failed to get wallet address")
    }

    const { swapTx, approveTx } = await buildUniswapSwapTx(
      this.uniswapRouterAddress,
      input.token_input,
      input.token_output,
      input.amount_in,
      walletAddress
    )

    if (approveTx) {
      await this.safSigner.signTransactionAndBroadcast({
        to: approveTx.to! as string,
        data: approveTx.data! as string,
        value: "0",
      })
    }

    const txHash = await this.safSigner.signTransactionAndBroadcast({
      to: swapTx.to! as string,
      data: swapTx.data! as string,
      value: "0",
    })

    return `Intented to swap  ${input.amount_in}  of ${input.token_input} for ${input.token_output}

    txHash: ${txHash}
    `
  }
}
