import { Tool, ToolSchema, InferSchemaType } from "./types"

const generateNFTSchema = {
  type: "function",
  function: {
    name: "generate_nft_collection",
    description:
      "Generate NFT artworks with image and metadata using AI based on style prompt",
    parameters: {
      type: "object",
      properties: {
        style_prompt: {
          type: "string",
          description:
            "Detailed prompt describing the style and elements for AI art generation",
        },
        lore: {
          type: "string",
          description: "Story/description/lore of the collection",
        },
        collection_name: {
          type: "string",
          description: "Name of the NFT collection",
        },
        count: {
          type: "integer",
          description: "Number of NFTs to generate",
          minimum: 1,
          maximum: 10000,
        },
      },
      required: ["style_prompt", "count", "lore", "collection_name"],
    },
  },
} as const satisfies ToolSchema

type GenerateNFTInput = InferSchemaType<typeof generateNFTSchema>

export const GenerateNFTTool: Tool<GenerateNFTInput> = {
  name: generateNFTSchema.function.name,
  schema: generateNFTSchema,
  execute: async (input: GenerateNFTInput) => {
    // TypeScript will know these types:
    // input.style_prompt: string
    // input.lore: string
    // input.collection_name: string
    // input.count: number
    return `Generated NFT collection ${input.collection_name} with ${input.count} artworks using style prompt: ${input.style_prompt} and lore: ${input.lore}`
  },
}
