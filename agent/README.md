# README

## Run examples

Navigate to `examples` directory.

### Example server

```sh
pnpm dev
```

### Token transfer using Private Key signing and Safe Signing

For Safe Signer make sure to you have a running gRPC signer server. A mock server can be started using. It will sign a safe signature using the private key in environment variable `PRIVATE_KEY`.

```sh
pnpm grpcMockEthSigner
```

To execute sample transfers

```sh
pnpm transfer
```

## Build from source

```sh
pnpm install
pnpm build
```

## Tests

There are unit tests as well as integration tests some of which requires API token. Make sure to create
`.env.test` with environment variables required for the integration tests.

To run tests

```sh
pnpm test
```

## Logs

```
socat TCP-LISTEN:12345,fork -
```

## Deploy NPM package

Update version in `package.json`

```
npm login
pnpm build
npm publish
```
