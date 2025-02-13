// migrations/YYYYMMDDHHMMSS-init-chat-schema.js

/* eslint-disable camelcase */

// DATABASE_URL=postgres://postgres:docker@localhost:5433/postgres npm run migrate up
// DATABASE_URL=postgres://postgres:docker@localhost:5433/postgres npm run migrate down

// CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

// CREATE TABLE chats (
//     id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
//     name TEXT NOT NULL,
//     created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
// );

// CREATE TABLE messages (
//     id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
//     chat_id UUID NOT NULL,
//     text TEXT,
//     tool_calls TEXT,
//     tool_call_id TEXT,
//     image_urls TEXT, -- can be stringified array of urls or a sitrngied json
//     role TEXT,
//     created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
//     pending to add toolArgs
//     FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE
// );

exports.up = (pgm) => {
  // Enable UUID extension
  //  pgm.createExtension("uuid-ossp", { ifNotExists: true })
  // Note: Using gen_random_uuid() which is available by default in Supabase

  // Create chats table
  pgm.createTable("chats", {
    id: {
      type: "uuid",
      primaryKey: true,
      default: pgm.func("gen_random_uuid()"),
    },
    user_id: {
      type: "text",
      notNull: true,
    },
    name: {
      type: "text",
      notNull: true,
    },
    created_at: {
      type: "timestamp",
      notNull: true,
      default: pgm.func("current_timestamp"),
    },
  })

  // Create messages table
  pgm.createTable("messages", {
    id: {
      type: "uuid",
      primaryKey: true,
      default: pgm.func("gen_random_uuid()"),
    },
    chat_id: {
      type: "uuid",
      notNull: true,
      references: "chats",
      onDelete: "cascade",
    },
    text: { type: "text" },
    tool_calls: { type: "jsonb" },
    tool_call_id: { type: "text" },
    tool_args: { type: "jsonb" },
    image_urls: { type: "jsonb" },
    role: { type: "text" },
    created_at: {
      type: "timestamp",
      notNull: true,
      default: pgm.func("current_timestamp"),
    },
  })
}

exports.down = (pgm) => {
  // Drop tables in reverse order
  pgm.dropTable("messages")
  pgm.dropTable("chats")
  // pgm.dropExtension("uuid-ossp")
}
