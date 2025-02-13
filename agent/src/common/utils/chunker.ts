// https://unstructured.io/blog/chunking-for-rag-best-practices
export function chunkText(text: string, chunkSize: number = 1000): string[] {
  const chunks: string[] = []
  let startIndex = 0

  while (startIndex < text.length) {
    let endIndex = startIndex + chunkSize

    // Adjust end index to not cut words in half
    if (endIndex < text.length) {
      endIndex = text.lastIndexOf(" ", endIndex) + 1
    }

    chunks.push(text.slice(startIndex, endIndex).trim())
    startIndex = endIndex
  }

  return chunks
}
