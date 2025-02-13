export function canonicalizeJson(obj: any): string {
  if (Array.isArray(obj)) {
    return "[" + obj.map(canonicalizeJson).join(",") + "]"
  } else if (typeof obj === "object" && obj !== null) {
    return (
      "{" +
      Object.keys(obj)
        .sort()
        .map((key: string) => `"${key}":${canonicalizeJson(obj[key])}`)
        .join(",") +
      "}"
    )
  } else if (typeof obj === "string") {
    return JSON.stringify(obj)
  } else {
    return JSON.stringify(obj)
  }
}
