import { Agent, OAICompatibleModel } from "../common"
import { createChatServer } from "./server"
interface ServerConfig {
  port?: number
  host?: string
  dbConfig?: {
    type: "postgres"
    postgres: {
      host: string
      port: number
      database: string
      user: string
      password: string
      ssl?: boolean
    }
  }
  tools?: any[]
  systemPrompt?: string
  modelName?: string
}

export async function createServer(config: ServerConfig = {}) {
  const {
    port = 3002,
    host = "0.0.0.0",
    dbConfig = {
      type: "postgres",
      postgres: {
        host: process.env.POSTGRES_HOST || "localhost",
        port: parseInt(process.env.POSTGRES_PORT || "5432"),
        database: process.env.POSTGRES_DB || "postgres",
        user: process.env.POSTGRES_USER || "postgres",
        password: process.env.POSTGRES_PASSWORD || "postgres",
      },
    },
    tools = [],
    systemPrompt = "You are a helpful assistant.",
    modelName = "gpt-4o-mini",
  } = config

  const model = new OAICompatibleModel({
    modelName,
  })

  const agent = new Agent({
    model,
    tools,
    systemPrompt,
  })

  return createChatServer({
    storage: dbConfig,
    agent,
    port,
    host,
  })
}

if (require.main === module) {
  createServer().catch((err) => {
    console.error("Failed to start server:", err)
    process.exit(1)
  })
}
