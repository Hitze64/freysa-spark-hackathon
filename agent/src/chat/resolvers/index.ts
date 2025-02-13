import { Resolvers } from "../generated/graphql"
import { messageEventService } from "../services/MessageEventService"

export const resolvers: Resolvers = {
  Query: {
    getChats: (_, { first, offset }, { chatService, userId }) =>
      chatService.getChats(first, offset, userId),
    getChat: (_, { id }, { chatService }) => chatService.getChat(id),
  },

  Mutation: {
    createChat: (_, { name }, { chatService, userId }) =>
      chatService.createChat(name, userId),
    sendMessage: (_, { chatId, text }, { chatService }) =>
      chatService.sendMessage(chatId, text),
    updateTitle: (_, { chatId, title }, { chatService }) =>
      chatService.updateChatTitle(chatId, title),
    deleteChat: (_, { chatId }, { chatService }) =>
      chatService.deleteChat(chatId),
  },

  Subscription: {
    messageAdded: {
      subscribe: (_, { chatId }, { storage }) => ({
        [Symbol.asyncIterator]: () => {
          const subscription = messageEventService.createSubscription(
            chatId,
            storage
          )

          return {
            next: () => subscription.next(),
            return: () => {
              subscription.cleanup()
              return Promise.resolve({ value: undefined, done: true })
            },
            throw: (error) => {
              subscription.cleanup()
              return Promise.reject(error)
            },
          }
        },
      }),
    },
  },
}
