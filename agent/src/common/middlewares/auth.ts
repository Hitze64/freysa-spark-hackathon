import { FastifyReply, FastifyRequest } from "fastify"
import { JWTService } from "./jwt"

export const createAuthMiddlereFromJwtService = (jwtService: JWTService) => {
  return async (request: FastifyRequest, reply: FastifyReply) => {
    try {
      const authHeader = request.headers.authorization
      if (!authHeader?.startsWith("Bearer ")) {
        return reply.status(401).send({ error: "No token provided" })
      }

      const token = authHeader.split(" ")[1]

      try {
        const userId = await jwtService.verifyToken(token)
        request.userId = userId
        return userId
      } catch (error) {
        console.error("Token verification failed:", error)
        return reply.status(401).send({ error: "Invalid token" })
      }
    } catch (error) {
      console.error("Auth middleware error:", error)
      return reply.status(500).send({ error: "Internal server error" })
    }
  }
}
