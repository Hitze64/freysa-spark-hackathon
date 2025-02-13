import { LocalStorage } from "./local"
import { S3Storage } from "./s3"
import { FileStorage, FileStorageConfig } from "./types"

export class FileStorageFactory {
  static create(config: FileStorageConfig): FileStorage {
    switch (config.type) {
      case "local":
        if (!config.local?.assetsPath) {
          throw new Error("Local storage requires assetsPath")
        }
        return new LocalStorage(config.local.assetsPath)

      case "s3":
        if (!config.s3) {
          throw new Error("S3 storage requires configuration")
        }
        return new S3Storage(config.s3)

      default:
        throw new Error(`Unsupported storage type: ${config.type}`)
    }
  }
}
