import { createServer, ReplicateImageGenerationTool } from "sovereign-agent"
import dotenv from "dotenv"

dotenv.config()

async function startServer() {
  const replicateTool = new ReplicateImageGenerationTool({
    model: "728c47d1aaa9bca3fa37888ef1fc36bc4ab7306579dd2321afcb99e530a17a33",
    storageConfig: {
      type: "s3",
      s3: {
        bucket: process.env.AWS_BUCKET_NAME!,
        region: process.env.AWS_REGION!,
        accessKeyId: process.env.AWS_ACCESS_KEY_ID!,
        secretAccessKey: process.env.AWS_SECRET_ACCESS_KEY!,
        prefix: "images",
      },
    },
    includeUrls: true,
  })

  const server = await createServer({
    port: parseInt(process.env.PORT || "3002"),
    host: process.env.HOST || "0.0.0.0",
    tools: [replicateTool],
    systemPrompt: "You are a helpful AI assistant.",
    dbConfig: {
      type: "postgres",
      postgres: {
        host: process.env.POSTGRES_HOST || "localhost",
        port: parseInt(process.env.POSTGRES_PORT || "5432"),
        database: process.env.POSTGRES_DB || "postgres",
        user: process.env.POSTGRES_USER || "postgres",
        password: process.env.POSTGRES_PASSWORD || "postgres",
      },
    },
  })

  // Handle graceful shutdown
  const signals = ["SIGTERM", "SIGINT"] as const
  signals.forEach((signal) => {
    process.on(signal, async () => {
      try {
        await server.close()
        process.exit(0)
      } catch (err) {
        console.error("Error during shutdown:", err)
        process.exit(1)
      }
    })
  })

  return server
}

startServer()
  .then((server) => {
    console.log("Server started successfully")
  })
  .catch((err) => {
    console.error("Failed to start server:", err)
    process.exit(1)
  })
