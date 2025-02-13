import {
  SAFSigner,
  PrivateKeySignerConfig,
  SafeSignerConfig,
  logger,
} from "sovereign-agent"
import dotenv from "dotenv"
import { buildUniswapSwapTx } from "@/common/transactions/swap"
import { ethers } from "ethers"

dotenv.config()

async function performSwap() {
  const ERC20_ABI = [
    "function approve(address spender, uint256 amount) returns (bool)",
    "function allowance(address owner, address spender) view returns (uint256)",
    "function balanceOf(address owner) view returns (uint256)",
    "function decimals() view returns (uint8)",
  ]

  const UNISWAP_ROUTER_ABI = [
    "function swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)",
    "function getAmountsOut(uint amountIn, address[] memory path) view returns (uint[] memory amounts)",
  ]

  // Configuration
  const config = {
    providerUrl: process.env.RPC_URL!,
    privateKey: process.env.PRIVATE_KEY!,
    routerAddress: "0x4752ba5DBc23f44D87826276BF6Fd6b1C372aD24", // Uniswap V2 Router
    tokenIn: "0x4200000000000000000000000000000000000006", // e.g., WETH
    tokenOut: "0x0b3e328455c4059EEb9e3f84b5543F74E24e7E1b", // e.g., UNI
    amountIn: "0.000001", // Amount to swap (in human-readable format)
    slippage: 0.03, // 5% slippage
  }

  const provider = new ethers.JsonRpcProvider(config.providerUrl)
  const wallet = new ethers.Wallet(config.privateKey, provider)

  // Create contract instances
  const router = new ethers.Contract(
    config.routerAddress,
    UNISWAP_ROUTER_ABI,
    wallet
  )
  const tokenIn = new ethers.Contract(config.tokenIn, ERC20_ABI, wallet)

  // Get token decimals
  const decimals = await tokenIn.decimals()
  const amountInWei = ethers.parseUnits(config.amountIn, decimals)

  // Check and approve allowance
  const allowance = await tokenIn.allowance(
    wallet.address,
    config.routerAddress
  )

  if (allowance < amountInWei) {
    const approveTx = await tokenIn.approve(config.routerAddress, amountInWei)
    await approveTx.wait()
  }

  const path = [config.tokenIn, config.tokenOut]

  const amountsOut = await router.getAmountsOut(amountInWei, path)
  const percentage = BigInt(Math.floor(config.slippage * 100))
  const amountOutMin = (amountsOut[1] * percentage) / BigInt(100)

  // Execute swap
  const deadline = Math.floor(Date.now() / 1000) + 60 * 20 // 20 minutes

  const tx = await router.swapExactTokensForTokens.populateTransaction(
    amountInWei,
    amountOutMin,
    path,
    wallet.address,
    deadline,
    { gasLimit: 250000 }
  )

  console.log(tx)
}

async function test() {
  logger.info("Testing private key signer")
  performSwap().catch(console.error)
}

test()
