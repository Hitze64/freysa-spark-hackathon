export interface OpenAIMessage {
  role: "user" | "assistant" | "system"
  content: string
}

export function transformToOpenAIMessages(messages: any[]): OpenAIMessage[] {
  return messages.map((msg) => ({
    role: msg.role.toLowerCase(),
    content: msg.text,
  }))
}
