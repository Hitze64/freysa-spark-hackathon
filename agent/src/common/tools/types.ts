import { ChatCompletionTool } from "openai/resources/index"

/** OpenAI tool schema type alias */
export type ToolSchema = ChatCompletionTool

/**
 * Represents a JSON Schema type definition
 * Used for defining the structure and validation rules of JSON data
 */
type JSONSchemaType = {
  /** The data type of the schema property */
  type: "string" | "number" | "integer" | "boolean" | "object" | "array"
  /** Optional description of the schema property */
  description?: string
  /** Optional minimum value for number types */
  minimum?: number
  /** Optional maximum value for number types */
  maximum?: number
  /** Schema definition for array items */
  items?: JSONSchemaType
  /** Nested property definitions for object types */
  properties?: Record<string, JSONSchemaType>
  /** List of required property names */
  required?: string[]
}

/**
 * Utility type that converts JSON Schema definitions to TypeScript types
 * @template T - The JSON Schema type definition to convert
 */
type PropertyType<T> = T extends { type: string }
  ? T extends { type: "string" }
    ? string
    : T extends { type: "number" }
      ? number
      : T extends { type: "integer" }
        ? number
        : T extends { type: "boolean" }
          ? boolean
          : T extends {
                type: "object"
                properties: Record<string, JSONSchemaType>
              }
            ? { [K in keyof T["properties"]]: PropertyType<T["properties"][K]> }
            : T extends {
                  type: "array"
                  items: JSONSchemaType
                }
              ? PropertyType<T["items"]>[]
              : unknown
  : unknown

/**
 * Extracts the TypeScript type from a function parameter schema
 * @template T - The function schema to extract parameters from
 */
export type InferSchemaType<T> = T extends {
  type: "function"
  function: {
    parameters: {
      type: "object"
      properties: infer P
    }
  }
}
  ? { [K in keyof P]: PropertyType<P[K]> }
  : Record<string, never>

/**
 * Represents an executable tool that can be used by the AI
 * @template TInput - The expected input type for the tool
 */
export interface Tool<TInput = unknown> {
  /** Unique name identifier for the tool */
  name: string
  /** OpenAI-compatible schema describing the tool's parameters */
  schema: ToolSchema
  /**
   * Executes the tool's functionality
   * @param input - The parameters passed to the tool
   * @returns A promise that resolves to the tool's output as a string
   */
  execute(input: TInput): Promise<string>
}

//@TODO Make this based on the tool names in the tool schema
export type ToolName =
  | "replicate_image_generation"
  | "plan"
  | "widget_createDonateWidget"

export type ToolArguments<T> = {
  name: ToolName
  arguments: T
}[]

export const TOOL_EVENTS = {
  TOOL_START: "TOOL_START",
  TOOL_END: "TOOL_END",
  TOOL_START_PROCESSED: "TOOL_START_PROCESSED",
} as const

export const WIDGET_TOOL_PREFIX = "widget_"
