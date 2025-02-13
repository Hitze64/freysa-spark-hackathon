export interface FileStorageConfig {
  type: "local" | "s3"
  local?: {
    assetsPath: string
  }
  s3?: {
    bucket: string
    region: string
    accessKeyId: string
    secretAccessKey: string
    prefix?: string
  }
}

export interface FileStorage {
  upload(url: string): Promise<string>
  getLocation(): string
}
