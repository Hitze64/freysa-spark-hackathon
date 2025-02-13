import { TChatMessage } from "../models"

/**
 * Represents an executor that can perform a task.
 *
 * The `Executor` type defines an object with a single method `execute`.
 * This method takes a `task` as a string and returns a Promise that resolves
 * to a string. The `task` parameter represents the task to be executed, and
 * the returned Promise resolves with the result of the task execution.
 */
export type Executor = {
  executeTask: (task: string) => Promise<string>
  execute: (messages: TChatMessage[]) => Promise<string>
}
