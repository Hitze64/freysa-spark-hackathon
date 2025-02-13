export const parseHeaders = (
  headers: Record<string, string | string[] | undefined>
): Record<string, string> => {
  const parsedHeaders: Record<string, string> = {}

  for (const [key, value] of Object.entries(headers)) {
    if (typeof value === "string") {
      parsedHeaders[key] = value
    } else if (Array.isArray(value)) {
      parsedHeaders[key] = value[0] || ""
    }
  }

  return parsedHeaders
}
