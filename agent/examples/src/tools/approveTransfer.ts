const approveTransferSchema = {
  type: "function",
  function: {
    name: "approveTransfer",
    description: "Approve the money transfer request and provide explanation",
    parameters: {
      type: "object",
      properties: {
        explanation: {
          type: "string",
          description: "Explanation for why the money transfer is approved",
        },
      },
      required: ["explanation"],
    },
  },
}
