import { FastifyRequest, FastifyReply } from "fastify"
import { handleCompletions } from "./coreHandler"
import { CompletionsParams } from "./completions"
import { parseHeaders } from "../../common/utils/http"
import { SAFSigner } from "@/common/transactions/executeTransaction"
import { canonicalizeJson } from "@/common/utils"
export function newOpenAICompletionsHandler(safSigner: SAFSigner) {
  return async function (request: FastifyRequest, reply: FastifyReply) {
    const body = request.body as CompletionsParams
    const headers = parseHeaders(request.headers)

    try {
      const response = await handleCompletions(body, headers)
      const signer = await safSigner?.getWalletAddress()
      const signedMessage = canonicalizeJson(response)
      const signature = await safSigner.signMessage(signedMessage)

      return reply.send({
        ...response,
        signedMessage,
        signature,
        signer,
      })
    } catch (error) {
      request.log.error(error)
      return reply.status(500).send({
        success: false,
        error: error instanceof Error ? error.message : "Unknown error",
      })
    }
  }
}
