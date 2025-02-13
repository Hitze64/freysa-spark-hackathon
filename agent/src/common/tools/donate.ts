import {
  InferSchemaType,
  Tool,
  ToolArguments,
  ToolSchema,
} from "@/common/tools"

const schema = {
  type: "function",
  function: {
    name: "widget_createDonateWidget",
    description:
      "Creates a donation widget when user mentions donating or supporting",
    parameters: {
      type: "object",
      properties: {
        message: {
          type: "string",
          description: "Thank you for wanting to donate, you can do it above!",
        },
      },
      required: ["message"],
    },
  },
} as const satisfies ToolSchema

//@TODO: Export to be re-used on FE from the package
type DonateWidgetType = InferSchemaType<typeof schema>

export const DonateTool: Tool<DonateWidgetType> = {
  name: schema.function.name,
  schema,
  execute: async (input: DonateWidgetType) => {
    const widgetData: ToolArguments<string> = [
      {
        name: schema.function.name,
        arguments: JSON.stringify({
          message: input.message,
        }),
      },
    ]

    return JSON.stringify(widgetData)
  },
}
