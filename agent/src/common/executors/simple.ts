import { Agent } from "../agents"
import { TChatMessage } from "../models"
import { Executor } from "./types"

export class SimpleExecutor implements Executor {
  private agent: Agent

  constructor(agent: Agent) {
    this.agent = agent
  }

  async executeTask(task: string) {
    return this.agent.executeTask(task)
  }

  async execute(messages: TChatMessage[]) {
    return this.agent.execute(messages)
  }
}
