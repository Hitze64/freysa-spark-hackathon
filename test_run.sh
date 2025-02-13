docker build -t vsock-test -f Dockerfile.test .
nitro-cli build-enclave --docker-uri vsock-test:latest --output-file vsock_test.eif
nitro-cli run-enclave --eif-path vsock_test.eif --memory 512 --cpu-count 2 --debug-mode

echo "sleep 5..."
sleep 5

# Test from host
ENCLAVE_CID=$(nitro-cli describe-enclaves | jq -r '.[0].EnclaveCID')
echo "testing socat connect"
echo "test" | socat -d -d - VSOCK-CONNECT:${ENCLAVE_CID}:5000
