# Governance Committee Protocol

The authorization of measurements for TEEs that can join a TEE key-sharing pool is governed by a committee requiring `M` out of `N` signatures.

## Safe smart contract

## AWS Nitro

Authorize measurements.

- Prerequisites PCR-{0-2} and PCR-4 are each 48 bytes.
- `code_message = 'AWS-CODE:' PCR-0 ':' PCR-1 ':' PCR-2`.
- `instance_message = 'AWS-INSTANCE:' PCR-4`.

- [AWS Nitro Enclave PCR Reference](https://docs.aws.amazon.com/enclaves/latest/user/set-up-attestation.html).

Note: AWS Nitro puts `SHA384(padright(48, instance-ID, '0'))` into `PCR-4`.

Verify that `code_message` and `instance_message` are authorized by the Safe smart contract.
