import { PerplexityApiResponse } from "./types"

const defaultModel = "llama-3.1-sonar-small-128k-online"

export async function completions(opts: {
  userPrompt: string
  model?: string
  systemPrompt?: string
  recencyFilter?: "month" | "week" | "day" | "hour"
}): Promise<string> {
  const token = process.env.PERPLEXITY_API
  if (!token) {
    throw new Error("PERPLEXITY_API environment variable is not set")
  }

  const messages = []
  if (opts.systemPrompt) {
    messages.push({ role: "system", content: opts.systemPrompt })
  }
  messages.push({ role: "user", content: opts.userPrompt })

  const options = {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model: opts.model || defaultModel,
      messages: messages,
      search_recency_filter: opts.recencyFilter,
      stream: false,
    }),
  }

  try {
    const response = await fetch(
      "https://api.perplexity.ai/chat/completions",
      options
    )
    const data = (await response.json()) as PerplexityApiResponse

    const content = data.choices[0]?.message?.content || ""
    return content
  } catch (err) {
    console.error(err)
    throw new Error("Failed to fetch completions")
  }
}
