import { join } from "path"
import { existsSync, mkdirSync, writeFileSync } from "fs"

export async function saveUrlToFile(
  url: string,
  baseDirectory: string,
  filename: string
) {
  if (!existsSync(baseDirectory)) {
    mkdirSync(baseDirectory, { recursive: true })
  }
  const response = await fetch(url)
  const buffer = Buffer.from(await response.arrayBuffer())
  const imagePath = join(baseDirectory, filename)
  writeFileSync(imagePath, buffer)
}
