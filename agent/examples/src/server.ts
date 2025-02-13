import Fastify from "fastify"
import cors from "@fastify/cors"
import * as dotenv from "dotenv"

dotenv.config()

import {
  Agent,
  OAICompatibleModel,
  GenerateNFTTool,
  Scheduler,
  PerplexitySearchTool,
  GetLatestTweetsTool,
  newOpenAICompletionsHandler,
  ReplicateImageGenerationTool,
  logger,
  SAFSigner,
  UniswapTool,
  PrivateKeySignerConfig,
  PostTweetTool,
  TransactionMode,
} from "sovereign-agent"
async function startServer() {
  logger.info("Server starting...")

  const fastify = Fastify({
    logger: true,
  })

  await fastify.register(cors, {
    origin: true,
  })

  fastify.get("/health", async () => {
    return { status: "ok", timestamp: new Date().toISOString() }
  })

  const model = new OAICompatibleModel({
    modelName: "gpt-4o-mini",
  })

  const perplexityTool = new PerplexitySearchTool()
  const replicateTool = new ReplicateImageGenerationTool({
    model: "728c47d1aaa9bca3fa37888ef1fc36bc4ab7306579dd2321afcb99e530a17a33",
    storageConfig: {
      type: "local",
      local: {
        assetsPath: "./assets",
      },
    },
    includeUrls: true,
  })

  const privateKeySignerConfig: PrivateKeySignerConfig = {
    type: TransactionMode.PRIVATE_KEY,
    privateKey: process.env.PRIVATE_KEY!,
    rpcUrl: process.env.RPC_URL!,
  }
  const safSigner = new SAFSigner(privateKeySignerConfig)
  const uniswapTool = new UniswapTool(safSigner)

  const agent = new Agent({
    model: model,
    tools: [
      GenerateNFTTool,
      // perplexityTool,
      replicateTool,
      GetLatestTweetsTool,
      uniswapTool,
      PostTweetTool,
    ],
    systemPrompt: "You are a helpful assistant.",
  })

  fastify.post("/agent/execute", async (request, reply) => {
    const { task } = request.body as { task: string }

    try {
      const result = await agent.executeTask(task)
      logger.info("Agent Result:")
      logger.info(JSON.stringify(result))
      return { success: true, result }
    } catch (error) {
      fastify.log.error(error)
      return reply.status(500).send({
        success: false,
        error: error instanceof Error ? error.message : "Unknown error",
      })
    }
  })

  const openaiCompletionsHandler = newOpenAICompletionsHandler(safSigner)
  fastify.post("/v1/api/openai/chat/completions", openaiCompletionsHandler)

  const scheduler = new Scheduler("Etc/UTC")
  scheduler.scheduleEveryMinutes(5, async () => {
    console.log("Checking Instagram messages")
  })

  scheduler.scheduleEveryHours(3, async () => {
    agent.executeTask("Get latest tweets of @freysa.ai and generate a meme.")
  })

  scheduler.schedule("0 12 * * *", async () => {
    agent.executeTask(
      "Get last 5 users mentioning @elonmusk and reply funny message."
    )
  })

  const port = process.env.PORT ? parseInt(process.env.PORT) : 3002
  const host = process.env.HOST || "0.0.0.0"

  try {
    logger.info(`Attempting to start server on ${host}:${port}`)
    const address = await fastify.listen({ port, host })
    logger.info(`Server running at ${address}`)
  } catch (err) {
    logger.error({ err }, "Failed to start server")
    if (err instanceof Error) {
      logger.error(`Error details: ${err.message}`)
      if (err.stack) logger.error(`Stack trace: ${err.stack}`)
    }
    process.exit(1)
  }
}

startServer().catch((err) => {
  console.error("Failed to start server:", err)
  process.exit(1)
})
