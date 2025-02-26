import { Tool, ToolSchema, InferSchemaType } from "./types"
import { TwitterApi } from "twitter-api-v2"

const getLatestTweetsSchema = {
  type: "function",
  function: {
    name: "GetLatestTweets",
    description: "Get the latest tweets from a specific Twitter handle",
    parameters: {
      type: "object",
      properties: {
        handle: {
          type: "string",
          description: "The Twitter handle (without @)",
        },
        count: {
          type: "integer",
          description: "Number of tweets to retrieve",
          minimum: 5,
          maximum: 100,
        },
        since: {
          type: "string",
          description:
            "Optional timestamp to get tweets since (ISO 8601 format)",
        },
      },
      required: ["handle", "count"],
    },
  },
} as const satisfies ToolSchema

type GetLatestTweetsInput = InferSchemaType<typeof getLatestTweetsSchema>

const GetLatestTweetsTool: Tool<GetLatestTweetsInput> = {
  name: getLatestTweetsSchema.function.name,
  schema: getLatestTweetsSchema,
  execute: async (input: GetLatestTweetsInput) => {
    const client = new TwitterApi({
      // @ts-ignore
      appKey: process.env.TWITTER_APP_KEY,
      appSecret: process.env.TWITTER_APP_SECRET,
      accessToken: process.env.TWITTER_ACCESS_TOKEN,
      accessSecret: process.env.TWITTER_ACCESS_SECRET,
    })

    try {
      const user = await client.v2.userByUsername(input.handle)

      if (!user.data) {
        throw new Error(`User @${input.handle} not found`)
      }

      // X API limits the number of results to between 5 and 100
      const numberOfResults = Math.min(Math.max(input.count, 5), 100)

      const tweets = await client.v2.userTimeline(user.data.id, {
        max_results: numberOfResults,
        ...(input.since && { start_time: new Date(input.since).toISOString() }),
        "tweet.fields": ["created_at", "text"],
      })

      const tweetData = tweets.data.data

      if (!tweetData || tweetData.length === 0) {
        return `No tweets found for @${input.handle}${input.since ? ` since ${input.since}` : ""}`
      }

      const formattedTweets = tweetData.map((tweet) => ({
        created_at: tweet.created_at,
        text: tweet.text,
      }))

      return JSON.stringify(formattedTweets, null, 2)
    } catch (error) {
      console.error("Error fetching tweets:", error)
      return `Error: ${error}`
    }
  },
}

const postTweetSchema = {
  type: "function",
  function: {
    name: "PostTweet",
    description: "Post a new tweet",
    parameters: {
      type: "object",
      properties: {
        content: {
          type: "string",
          description: "The content of the tweet",
        },
      },
      required: ["content"],
    },
  },
} as const satisfies ToolSchema

type PostTweetInput = InferSchemaType<typeof postTweetSchema>

const PostTweetTool: Tool<PostTweetInput> = {
  name: postTweetSchema.function.name,
  schema: postTweetSchema,
  execute: async (input: PostTweetInput) => {
    try {
      const client = new TwitterApi({
        // @ts-ignore
        appKey: process.env.TWITTER_APP_KEY,
        appSecret: process.env.TWITTER_APP_SECRET,
        accessToken: process.env.TWITTER_ACCESS_TOKEN,
        accessSecret: process.env.TWITTER_ACCESS_SECRET,
      })

      const rwClient = client.readWrite

      const tweet = await rwClient.v2.tweet(input.content)

      return `Successfully posted tweet with ID: ${tweet.data.id}`
    } catch (error) {
      console.log("ERROR", error)
      return `Error posting tweet: ${error}`
    }
  },
}

const getLatestMentionsSchema = {
  type: "function",
  function: {
    name: "GetLatestMentions",
    description: "Get the latest mentions for a specific Twitter handle",
    parameters: {
      type: "object",
      properties: {
        handle: {
          type: "string",
          description: "The Twitter handle (without @)",
        },
        count: {
          type: "integer",
          description: "Number of mentions to retrieve",
          minimum: 10,
          maximum: 100,
        },
        since: {
          type: "string",
          description:
            "Optional timestamp to get mentions since (ISO 8601 format)",
        },
      },
      required: ["handle", "count"],
    },
  },
} as const satisfies ToolSchema

type GetLatestMentionsInput = InferSchemaType<typeof getLatestMentionsSchema>

const GetLatestMentionsTool: Tool<GetLatestMentionsInput> = {
  name: getLatestMentionsSchema.function.name,
  schema: getLatestMentionsSchema,
  execute: async (input: GetLatestMentionsInput) => {
    const client = new TwitterApi({
      // @ts-ignore
      appKey: process.env.TWITTER_APP_KEY,
      appSecret: process.env.TWITTER_APP_SECRET,
      accessToken: process.env.TWITTER_ACCESS_TOKEN,
      accessSecret: process.env.TWITTER_ACCESS_SECRET,
    })

    try {
      const query = `@${input.handle}`

      const options: any = {
        max_results: input.count,
        "tweet.fields": ["author_id", "created_at", "text"],
      }

      if (input.since) {
        options.start_time = new Date(input.since).toISOString()
      }

      const tweets = await client.v2.search(query, options)

      const formattedTweets = tweets.data.data.map((tweet) => ({
        id: tweet.id,
        text: tweet.text,
        author_id: tweet.author_id,
        created_at: tweet.created_at,
      }))

      return JSON.stringify(formattedTweets, null, 2)
    } catch (error) {
      console.error("Error fetching tweets:", error)
      return `Error: Unable to retrieve mentions for @${input.handle}`
    }
  },
}

export { GetLatestTweetsTool, PostTweetTool, GetLatestMentionsTool }
