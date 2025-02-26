const rejectTransferSchema = {
  type: "function",
  function: {
    name: "rejectTransfer",
    description: "Reject the money transfer request and provide explanation",
    parameters: {
      type: "object",
      properties: {
        explanation: {
          type: "string",
          description: "Explanation for why the money transfer is rejected",
        },
      },
      required: ["explanation"],
    },
  },
}
