import { FileStorage } from "@/common/storage"

type ReplicateInput = {
  aspect_ratio: string
  extra_lora_scale: number
  go_fast: boolean
  guidance_scale: number
  lora_scale: number
  megapixels: string
  model: string
  num_inference_steps: number
  num_outputs: number
  output_format: string
  output_quality: number
  prompt: string
  prompt_strength: number
  replicate_weights?: string
}

type ReplicateUrls = {
  cancel: string
  get: string
  stream: string
}

type ReplicateResponse = {
  id: string
  model: string
  version: string
  input: ReplicateInput
  logs: string
  output: string[]
  data_removed: boolean
  error: string | null
  status: "starting" | "processing" | "succeeded" | "failed" | "canceled"
  created_at: string
  urls: ReplicateUrls
}

export const replicateAPICall = async ({
  prompt,
  model,
  numOutputs = 1,
}: {
  prompt: string
  model: string
  numOutputs: number
}) => {
  const token = process.env.REPLICATE_API_TOKEN
  if (!token) {
    throw new Error("REPLICATE_API_TOKEN is not set")
  }
  const response = await fetch("https://api.replicate.com/v1/predictions", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
      Prefer: "wait",
    },
    body: JSON.stringify({
      version:
        "728c47d1aaa9bca3fa37888ef1fc36bc4ab7306579dd2321afcb99e530a17a33",
      input: {
        prompt: prompt,
        model: "dev",
        go_fast: false,
        lora_scale: 0.82,
        megapixels: "1",
        num_outputs: numOutputs,
        aspect_ratio: "1:1",
        output_format: "webp",
        guidance_scale: 2.5,
        output_quality: 80,
        prompt_strength: 0.8,
        extra_lora_scale: 1,
        num_inference_steps: 28,
      },
    }),
  })

  const data = (await response.json()) as ReplicateResponse
  return data.output ?? null
}

export const replicateRunAndSave = async ({
  prompt,
  model,
  storage,
  numOutputs = 1,
}: {
  prompt: string
  model: string
  storage: FileStorage
  numOutputs: number
}) => {
  const imageUrls = await replicateAPICall({ prompt, model, numOutputs })
  console.log("imageUrls", imageUrls)

  const storedUrls = await Promise.all(
    imageUrls.map((url) => storage.upload(url))
  )

  return storedUrls
}
