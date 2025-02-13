// src/chat/example.ts
import { Role } from "./generated/graphql"
import { StorageFactory, StorageConfig } from "./storage/factory"
import { ChatStorage } from "./storage/types"

async function main() {
  const pgConfig: StorageConfig = {
    type: "postgres",
    postgres: {
      host: "localhost",
      port: 5433,
      database: "chatdb",
      user: "postgres",
      password: "password",
    },
  }

  const sqliteConfig: StorageConfig = {
    type: "sqlite",
    sqlite: {
      filename: "./chat.db",
    },
  }

  // Choose configuration based on environment or any other logic we want to use
  const config = process.env.NODE_ENV === "production" ? pgConfig : sqliteConfig

  try {
    const storage: ChatStorage = await StorageFactory.create(config)

    const chat = await storage.createChat("New Chat")
    console.log("Created chat:", chat)

    const message = await storage.createMessage(chat.id, {
      text: "Hello, World!",
      role: Role.User,
      imageUrls: [],
      toolCalls: [],
    })
    console.log("Created message:", message)

    const retrievedChat = await storage.getChat(chat.id)
    console.log("Retrieved chat with messages:", retrievedChat)
  } catch (error) {
    console.error("Error:", error)
  }
}
