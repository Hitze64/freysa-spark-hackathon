import dotenv from "dotenv"

dotenv.config()

import {
  Server,
  ServerCredentials,
  StatusObject,
  Metadata,
  sendUnaryData,
  ServerUnaryCall,
} from "@grpc/grpc-js"
import { ethers } from "ethers"
import {
  KeyPoolServiceServer,
  SignMessageRequest,
  SignMessageResponse,
  SignEthereumTransactionRequest,
  SignEthereumTransactionResponse,
  GetEthereumAddressResponse,
  KeyPoolServiceService,
} from "../generated/proto/key_pool"

// Hardcoded private key for mock implementation (for testing only)
const MOCK_PRIVATE_KEY = process.env.PRIVATE_KEY

if (!MOCK_PRIVATE_KEY) {
  throw new Error("PRIVATE_KEY is not set")
}

// Create wallet instance for signing operations
const wallet = new ethers.Wallet(MOCK_PRIVATE_KEY)

// Helper function to handle errors with proper typing
function handleError(error: unknown): string {
  if (error instanceof Error) {
    return error.message
  }
  return String(error)
}

// Implement the SignerServer interface
const signerServer: KeyPoolServiceServer = {
  signMessage: async (
    call: ServerUnaryCall<SignMessageRequest, SignMessageResponse>,
    callback: sendUnaryData<SignMessageResponse>
  ) => {
    console.log("Received signMessage request", call.request)

    try {
      const { message } = call.request
      if (!message || message.length === 0) {
        callback(null, { signature: Buffer.alloc(0) }) // Adjusted to match expected response type
        return
      }

      // Convert Buffer to Uint8Array for ethers
      const messageBytes = new Uint8Array(message)
      const signature = await wallet.signMessage(messageBytes)

      // Remove '0x' prefix and convert to Buffer
      callback(null, { signature: Buffer.from(signature.slice(2), "hex") })
    } catch (error) {
      callback(null, { signature: Buffer.alloc(0) }) // Adjusted to match expected response type
    }
  },

  signTransaction: async (
    call: ServerUnaryCall<
      SignEthereumTransactionRequest,
      SignEthereumTransactionResponse
    >,
    callback: sendUnaryData<SignEthereumTransactionResponse>
  ) => {
    console.log("Received signTransaction request", call.request)

    try {
      const { txData } = call.request
      if (!txData || txData.length === 0) {
        callback(null, { txData: Buffer.alloc(0) }) // Adjusted to match expected response type
        return
      }

      // Parse the transaction data as JSON
      const txJson = JSON.parse(txData.toString())

      // Sign the transaction
      const signedTx = await wallet.signTransaction({
        to: txJson.to,
        nonce: txJson.nonce,
        gasLimit: txJson.gasLimit,
        gasPrice: txJson.gasPrice,
        data: txJson.data,
        value: txJson.value,
        chainId: txJson.chainId,
        type: 0, // Legacy transaction type
      })

      // Convert signed transaction to Buffer
      callback(null, { txData: Buffer.from(signedTx.slice(2), "hex") })
    } catch (error) {
      callback(null, { txData: Buffer.alloc(0) }) // Adjusted to match expected response type
    }
  },

  getEthereumAddress: async (
    call: ServerUnaryCall<unknown, GetEthereumAddressResponse>,
    callback: sendUnaryData<GetEthereumAddressResponse>
  ) => {
    console.log("Received getAddresses request")

    try {
      callback(null, { ethereumAddress: wallet.address }) // Adjusted to match expected response type
    } catch (error) {
      const status: StatusObject = {
        code: 13, // INTERNAL
        details: handleError(error),
        metadata: new Metadata(),
      }
      callback(status, null)
    }
  },

  // Implement the missing methods
  signDigest: async (call, callback) => {
    // Implementation for signDigest
  },

  signEthereumTransaction: async (call, callback) => {
    // Implementation for signEthereumTransaction
  },

  getEthereumAddress: async (call, callback) => {
    // Implementation for getEthereumAddress
  },
}

// Create and start the gRPC server
export function startMockSignerServer(port: number = 50051): Server {
  const server = new Server()
  server.addService(KeyPoolServiceService, signerServer)

  server.bindAsync(
    `0.0.0.0:${port}`,
    ServerCredentials.createInsecure(),
    (err, port) => {
      if (err) {
        console.error("Failed to start server:", err)
        throw err
      }
      console.log(`Mock Signer Server running on port ${port}`)
      server.start()
    }
  )

  return server
}

// Start the server if this file is run directly
startMockSignerServer()
