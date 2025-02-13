import {
  SignMessageRequest,
  SignEthereumTransactionRequest,
  GetEthereumAddressRequest,
  SigningKey,
  HashFunction,
  KeyPoolServiceClient,
} from "../generated/proto/key_pool"
import { credentials } from "@grpc/grpc-js"

export class GrpcEthereumSignerClient {
  private client: KeyPoolServiceClient

  constructor(grpcServerUrl: string = "unix:///tmp/enclave.sock") {
    this.client = new KeyPoolServiceClient(
      grpcServerUrl,
      credentials.createInsecure()
    )
  }

  async signMessage(message: string, signingKey: SigningKey): Promise<string> {
    const request: SignMessageRequest = {
      signingKey: signingKey,
      hashFunction: HashFunction.HASH_FUNCTION_KECCAK256,
      message: Buffer.from(message),
    }
    return new Promise<string>((resolve, reject) => {
      this.client.signMessage(request, (error, response) => {
        if (error) {
          return reject(error)
        }
        if (response.signature) {
          const signatureHex =
            "0x" + Buffer.from(response.signature).toString("hex")
          resolve(signatureHex)
        } else {
          reject(new Error("Unknown error"))
        }
      })
    })
  }

  async signTransaction(
    txData: Uint8Array,
    signingKey: SigningKey
  ): Promise<Uint8Array> {
    const request: SignEthereumTransactionRequest = {
      signingKey: signingKey,
      txData: Buffer.from(txData),
    }
    return new Promise<Uint8Array>((resolve, reject) => {
      this.client.signEthereumTransaction(request, (error, response) => {
        if (error) {
          return reject(error)
        }
        if (response.txData) {
          resolve(response.txData)
        } else {
          reject(new Error("Unknown error"))
        }
      })
    })
  }

  async getAddress(signingKey: SigningKey): Promise<string> {
    const request: GetEthereumAddressRequest = {
      signingKey: signingKey,
    }
    return new Promise<string>((resolve, reject) => {
      this.client.getEthereumAddress(request, (error, response) => {
        if (error) {
          return reject(error)
        }
        const address = response.ethereumAddress
        resolve(address.startsWith("0x") ? address : `0x${address}`)
      })
    })
  }
}
