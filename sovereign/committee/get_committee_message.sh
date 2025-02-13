#!/bin/bash
set -euo pipefail

if [ $# -ne 2 ]; then
    echo "Usage: $0 <git-repo-url> <git-hash>" >&2
    echo "Example: $0 git@github.com:your-org/repo.git abc123" >&2
    exit 1
fi

REPO_URL=$1
GIT_HASH=$2
REPO_NAME=$(basename ${REPO_URL} .git)

git clone ${REPO_URL} >&2
cd ${REPO_NAME}
git checkout ${GIT_HASH} >&2

make -s enclave.eif 1>&2
PCR_STRING=$(nitro-cli describe-eif --eif-path enclave.eif | jq -r '"AWS-CODE:" + .Measurements.PCR0 + ":" + .Measurements.PCR1 + ":" + .Measurements.PCR2')
echo "$PCR_STRING"

rm -rf ${REPO_NAME} enclave.eif
