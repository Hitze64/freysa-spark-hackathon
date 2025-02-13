#!/bin/sh
echo "sleep 3..."
sleep 3

echo "starting VSOCK test listener..."
socat -d -d VSOCK-LISTEN:5000,reuseaddr,fork SYSTEM:'echo "hello from enclave $(date)"' &

# Keep container running
while true; do 
    echo "heartbeat..."
    sleep 10
done
