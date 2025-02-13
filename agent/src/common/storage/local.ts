import { FileStorage } from "./types"
import { saveUrlToFile } from "@/common/utils/filesystem"
import path from "path"

export class LocalStorage implements FileStorage {
  constructor(private readonly assetsPath: string) {}

  async upload(url: string): Promise<string> {
    const filename = `${Date.now()}-${Math.random().toString(36).substring(7)}.png`
    console.log("uploading local file", filename)
    const filepath = path.join(this.assetsPath, filename)
    await saveUrlToFile(url, this.assetsPath, filename)
    return filepath
  }

  getLocation(): string {
    return this.assetsPath
  }
}
