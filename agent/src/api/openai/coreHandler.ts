import { completions, CompletionsParams } from "./completions"

export async function handleCompletions(
  body: CompletionsParams,
  headers?: Record<string, string>
): Promise<any> {
  if (!headers?.authorization) {
    throw new Error("Authorization header is required")
  }

  const response = await completions(body, headers)
  return {
    ...response,
  }
}
