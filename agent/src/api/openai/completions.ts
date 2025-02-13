import { OpenAI } from "openai"
import {
  ChatCompletion,
  ChatCompletionCreateParamsBase,
} from "openai/resources/chat/completions"

export type CompletionsParams = ChatCompletionCreateParamsBase
export type CompletionsResponse = ChatCompletion & {
  signature: string
}

export type CompletionsOptions = {
  authorization: string
  organizationId: string
  projectId: string
}

export const completions = async (
  params: CompletionsParams,
  headers: Record<string, string>
) => {
  if (!headers.authorization) {
    throw new Error("Authorization header is required")
  }

  const openai = new OpenAI()
  const response = await openai.chat.completions.create(params, {
    headers: {
      Authorization: `${headers["authorization"]}`,
      "Content-Type": "application/json",
    },
  })
  return response
}
