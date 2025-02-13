export type Memory = {
  search(query: string): Promise<string[]>
  store(content: string): Promise<void>
  initialize?(): Promise<boolean>
  version: void | string
}
