# Stage 1: Build stage
FROM node:20 AS builder

WORKDIR /build

RUN apt-get update && \
    apt-get install -y python3 make g++ && \
    npm install -g pnpm

COPY . ./agent/

WORKDIR /build/agent
RUN pnpm install
RUN pnpm build

WORKDIR /build/agent/examples
RUN pnpm install
RUN pnpm build

# Stage 2: Save stage
FROM alpine:latest@sha256:21dc6063fd678b478f57c0e13f47560d0ea4eeba26dfc947b2a4f81f686b9f45

WORKDIR /prebuild

COPY --from=builder /build/agent/examples/dist/         /prebuild/dist/
COPY --from=builder /build/agent/examples/node_modules/ /prebuild/node_modules/

ARG TIMESTAMP=202501010000.00
RUN find /prebuild -exec touch -h -t ${TIMESTAMP} {} \; && \
    find /prebuild -type f -exec chmod 644 {} \; && \
    find /prebuild -type d -exec chmod 755 {} \;
