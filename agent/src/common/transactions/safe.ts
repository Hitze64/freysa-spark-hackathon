import { SigningMethod } from "@safe-global/protocol-kit"
import { ethers, Wallet } from "ethers"
import Safe from "@safe-global/protocol-kit"
import SafeApiKit from "@safe-global/api-kit"
import { MetaTransactionData, OperationType } from "@safe-global/types-kit"
import { GrpcEthereumSignerClient } from "./grpcEthereumSignerClient"
import { logger } from "@/common/utils"
import { isHex, Hex, Hash, recoverAddress } from "viem"
import { SigningKey } from "../generated/proto/key_pool"

export function asHash(hash: string): Hash {
  return hash as Hash
}

export function asHex(hex?: string): Hex {
  return isHex(hex) ? (hex as Hex) : (`0x${hex}` as Hex)
}

export function sameString(str1?: string, str2?: string): boolean {
  return !!str1 && !!str2 && str1.toLowerCase() === str2.toLowerCase()
}

type AdjustVOverload = {
  (
    signingMethod: SigningMethod.ETH_SIGN_TYPED_DATA,
    signature: string
  ): Promise<string>
  (
    signingMethod: SigningMethod.ETH_SIGN,
    signature: string,
    safeTxHash: string,
    sender: string
  ): Promise<string>
}

export async function isTxHashSignedWithPrefix(
  txHash: string,
  signature: string,
  ownerAddress: string
): Promise<boolean> {
  let hasPrefix
  try {
    const recoveredAddress = await recoverAddress({
      hash: asHash(txHash),
      signature: asHex(signature),
    })

    hasPrefix = !sameString(recoveredAddress, ownerAddress)
  } catch (e) {
    hasPrefix = true
  }
  return hasPrefix
}

export const adjustVInSignature: AdjustVOverload = async (
  signingMethod: SigningMethod.ETH_SIGN | SigningMethod.ETH_SIGN_TYPED_DATA,
  signature: string,
  safeTxHash?: string,
  signerAddress?: string
): Promise<string> => {
  const ETHEREUM_V_VALUES = [0, 1, 27, 28]
  const MIN_VALID_V_VALUE_FOR_SAFE_ECDSA = 27
  let signatureV = parseInt(signature.slice(-2), 16)
  if (!ETHEREUM_V_VALUES.includes(signatureV)) {
    throw new Error("Invalid signature")
  }
  if (signingMethod === SigningMethod.ETH_SIGN) {
    /*
      The Safe's expected V value for ECDSA signature is:
      - 27 or 28
      - 31 or 32 if the message was signed with a EIP-191 prefix. Should be calculated as ECDSA V value + 4
      Some wallets do that, some wallets don't, V > 30 is used by contracts to differentiate between
      prefixed and non-prefixed messages. The only way to know if the message was signed with a
      prefix is to check if the signer address is the same as the recovered address.

      More info:
      https://docs.safe.global/safe-core-protocol/signatures
    */
    if (signatureV < MIN_VALID_V_VALUE_FOR_SAFE_ECDSA) {
      signatureV += MIN_VALID_V_VALUE_FOR_SAFE_ECDSA
    }
    const adjustedSignature = signature.slice(0, -2) + signatureV.toString(16)
    const signatureHasPrefix = await isTxHashSignedWithPrefix(
      safeTxHash as string,
      adjustedSignature,
      signerAddress as string
    )
    if (signatureHasPrefix) {
      signatureV += 4
    }
  }
  if (signingMethod === SigningMethod.ETH_SIGN_TYPED_DATA) {
    // Metamask with ledger returns V=0/1 here too, we need to adjust it to be ethereum's valid value (27 or 28)
    if (signatureV < MIN_VALID_V_VALUE_FOR_SAFE_ECDSA) {
      signatureV += MIN_VALID_V_VALUE_FOR_SAFE_ECDSA
    }
  }
  signature = signature.slice(0, -2) + signatureV.toString(16)
  return signature
}

export async function signAndBroadcastSafeTransaction(opts: {
  to: string
  value: string
  data: string
  grpcClient: GrpcEthereumSignerClient
  safeAddress: string
  rpcUrl: string
  deployerProxy?: string
  signingKey: SigningKey
}) {
  const provider = new ethers.JsonRpcProvider(opts.rpcUrl)
  const chainId = await provider.getNetwork().then((network) => network.chainId)

  const protocolKitOwner1 = await Safe.init({
    provider: opts.rpcUrl,
    signer: process.env.PRIVATE_KEY,
    safeAddress: opts.safeAddress,
  })

  const safeTransactionData: MetaTransactionData = {
    to: opts.deployerProxy ? opts.deployerProxy : opts.to,
    value: opts.value,
    data: opts.data,
    operation: opts.deployerProxy
      ? OperationType.DelegateCall
      : OperationType.Call,
  }

  logger.info("Creating safe transaction", safeTransactionData)
  const safeTransaction = await protocolKitOwner1.createTransaction({
    transactions: [safeTransactionData],
  })

  const safeTxHash = await protocolKitOwner1.getTransactionHash(safeTransaction)
  const walletAddress = await opts.grpcClient.getAddress(opts.signingKey)
  let signatureV2 = await opts.grpcClient.signMessage(
    safeTxHash,
    opts.signingKey
  )

  signatureV2 = await adjustVInSignature(
    SigningMethod.ETH_SIGN,
    signatureV2,
    safeTxHash,
    walletAddress
  )

  const apiKit = new SafeApiKit({
    chainId: chainId,
  })

  const res = await apiKit.proposeTransaction({
    safeAddress: opts.safeAddress,
    safeTransactionData: safeTransaction.data,
    safeTxHash,
    senderAddress: walletAddress,
    senderSignature: signatureV2,
  })

  console.log(res)
}
