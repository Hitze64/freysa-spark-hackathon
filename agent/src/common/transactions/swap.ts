import { ethers } from "ethers"

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

export async function buildUniswapSwapTx(
  routerAddress: string,
  tokenInput: string,
  tokenOutput: string,
  amountIn: number,
  beneficiaryAddress: string
): Promise<{
  swapTx: ethers.TransactionRequest
  approveTx: ethers.TransactionRequest | null
}> {
  const config = {
    providerUrl: process.env.RPC_URL!,
    routerAddress: routerAddress,
    slippage: 0.03,
  }

  const provider = new ethers.JsonRpcProvider(config.providerUrl)

  const router = new ethers.Contract(
    config.routerAddress,
    UNISWAP_ROUTER_ABI,
    provider
  )
  const tokenIn = new ethers.Contract(tokenInput, ERC20_ABI, provider)

  const decimals = await tokenIn.decimals()
  const amountInWei = ethers.parseUnits(amountIn.toString(), decimals)

  const allowance = await tokenIn.allowance(
    beneficiaryAddress,
    config.routerAddress
  )

  let approveTx: ethers.TransactionRequest | null = null
  if (allowance < amountInWei) {
    approveTx = await tokenIn.approve.populateTransaction(
      config.routerAddress,
      amountInWei
    )
  }

  const path = [tokenInput, tokenOutput]
  const amountsOut = await router.getAmountsOut(amountInWei, path)
  const percentage = BigInt(Math.floor(config.slippage * 100))
  const amountOutMin = (amountsOut[1] * percentage) / BigInt(100)

  const deadline = Math.floor(Date.now() / 1000) + 60 * 20 // 20 minutes

  const swapTx = await router.swapExactTokensForTokens.populateTransaction(
    amountInWei,
    amountOutMin,
    path,
    beneficiaryAddress,
    deadline,
    { gasLimit: 250000 }
  )

  return { swapTx, approveTx }
}
