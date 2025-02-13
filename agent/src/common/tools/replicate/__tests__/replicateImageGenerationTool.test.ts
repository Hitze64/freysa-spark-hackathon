import { ReplicateImageGenerationTool } from "../tool"

// Set the timeout to 30 seconds (30000ms)
jest.setTimeout(30000)

describe("ReplicateImageGenerationTool", () => {
  it("should generate images based on the prompt", async () => {
    const model =
      "728c47d1aaa9bca3fa37888ef1fc36bc4ab7306579dd2321afcb99e530a17a33"
    const assetsPath = "./test-assets"
    const numOutputs = 1

    const tool = new ReplicateImageGenerationTool({
      model,
      assetsPath,
      numOutputs,
      includeUrls: true,
    })

    const prompt = "A futuristic cityscape at sunset"
    const result = await tool.execute({ prompt })

    console.log(result)
    expect(result).toContain(`Generated ${numOutputs} images at ${assetsPath}.`)
  })
})
