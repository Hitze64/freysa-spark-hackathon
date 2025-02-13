import { S3Client, PutObjectCommand } from "@aws-sdk/client-s3"
import { FileStorage } from "./types"

export class S3Storage implements FileStorage {
  private client: S3Client

  constructor(
    private readonly config: {
      bucket: string
      region: string
      accessKeyId: string
      secretAccessKey: string
      prefix?: string
    }
  ) {
    this.client = new S3Client({
      region: config.region,
      credentials: {
        accessKeyId: config.accessKeyId,
        secretAccessKey: config.secretAccessKey,
      },
    })
  }

  async upload(url: string): Promise<string> {
    const response = await fetch(url)
    if (!response.ok) {
      throw new Error(`Failed to download image: ${response.statusText}`)
    }
    const buffer = Buffer.from(await response.arrayBuffer())

    const timestamp = Date.now()
    const randomString = Math.random().toString(36).substring(7)
    const filename = `${timestamp}-${randomString}.png`

    const prefix = this.config.prefix?.replace(/^\/+|\/+$/g, "") // Remove leading/trailing slashes
    const key = prefix ? `${prefix}/${filename}` : filename

    await this.client.send(
      new PutObjectCommand({
        Bucket: this.config.bucket,
        Key: key,
        Body: buffer,
        ContentType: "image/png",
      })
    )

    console.log("uploaded to s3", key)
    return `https://${this.config.bucket}.s3.${this.config.region}.amazonaws.com/${key}`
  }

  getLocation(): string {
    return this.config.bucket
  }
}
