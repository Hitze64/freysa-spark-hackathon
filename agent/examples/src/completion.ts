import OpenAI from "openai"
import dotenv from "dotenv"

dotenv.config()

console.log(process.env.OPENAI_API_KEY)
const openai = new OpenAI({
  apiKey: process.env.OPENAI_API_KEY,
  baseURL: "http://localhost:3002/v1/api/openai",
  dangerouslyAllowBrowser: true,
})

async function getCompletion() {
  try {
    const completion = await openai.chat.completions.create({
      model: "gpt-3.5-turbo",
      messages: [{ role: "user", content: "Say your name" }],
      max_tokens: 50,
    })

    console.log(completion)
  } catch (error) {
    console.error("Error:", error)
  }
}

getCompletion()
