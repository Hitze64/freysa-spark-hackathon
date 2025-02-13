import * as jose from "jose"
import dotenv from "dotenv"

dotenv.config()

export class JWTService {
  private publicKey: jose.KeyLike | null = null
  private issuer: string
  private audience: string

  constructor(
    private readonly publicKeyPEM: string,
    issuer: string,
    audience: string
  ) {
    this.issuer = issuer
    this.audience = audience
    if (!publicKeyPEM) {
      throw new Error("publicKeyPEM must be provided")
    }
  }

  private async getPublicKey(): Promise<jose.KeyLike> {
    if (!this.publicKey) {
      this.publicKey = await jose.importSPKI(this.publicKeyPEM, "RS256")
    }
    return this.publicKey
  }

  async verifyToken(token: string): Promise<string> {
    try {
      const publicKey = await this.getPublicKey()
      const { payload } = await jose.jwtVerify(token, publicKey, {
        issuer: this.issuer,
        audience: this.audience,
      })

      const userId = payload.sub
      if (!userId) {
        throw new Error("No user ID in token")
      }

      return userId
    } catch (error) {
      throw new Error(
        `Token verification failed: ${error instanceof Error ? error.message : "Unknown error"}`
      )
    }
  }
}
