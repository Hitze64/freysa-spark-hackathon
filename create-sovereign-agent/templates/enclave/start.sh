#!/bin/sh

# -----------------------------------------------------------------------------
# DESCRIPTION.
#
# This script runs inside the enclave.
#
# Compare the script run_enclave.sh which prepares the host and starts the
# enclave. Many of the commands in this script corresponds with commands in
# run_enclave.sh.
#
# NOTES ON THE USE OF TAP.
#
# The enclave network uses a /31 subnet (10.0.0.0/31) which provides two usable IPs:
# - 10.0.0.0: host side of the TAP interface
# - 10.0.0.1: enclave side of the TAP interface
# This is a point-to-point link between the host and enclave.
# -----------------------------------------------------------------------------

# Give time to attach a console
STARTUP_SLEEP=3
echo "$(date -Iseconds): sleeping $STARTUP_SLEEP to allow for a console to attach (to debug config errors)"
sleep $STARTUP_SLEEP

# Everything happens inside the 'app' folder.
cd /app
ls -l
ls -l /dev/net/tun
lsmod | grep tun


# -----------------------------------------------------------------------------
# DEFINITIONS.
# -----------------------------------------------------------------------------

# Specify '', '-d', '-d -d', or '-d -d -d' for increasing log levels for socat.
SOCAT_OPTIONS='-d -d'

# TAP port: bridge VSOCK:$TAP_PORT to tap interface tap0.
# Must match the corresponding port in the host's 'run_enclave.sh'.
# If not specified in environment it defaults to 12345.
TAP_PORT=${TAP_PORT:-12345}

# Configuration port: the host will send the configuration to this VSOCK
# port once the enclave has started.
# Must match the corresponding port in the host's 'run_enclave.sh'.
# If not specified in environment it defaults to 72345 (leet `Freys`).
CONFIG_PORT=${CONFIG_PORT:-72345}



# -----------------------------------------------------------------------------
# STEP 0. Cleanup environment and cleanup on error.
# -----------------------------------------------------------------------------

trap 'echo "$(date -Iseconds): ERROR: critical error occurred"; exit 1' ERR
#trap 'echo "$(date -Iseconds): ERROR: received error"' ERR
#trap 'echo "$(date -Iseconds): ERROR: received interrupt"; exit 1' INT



# -----------------------------------------------------------------------------
# STEP 1: Get configuration
# -----------------------------------------------------------------------------

echo "$(date -Iseconds): requesting configuration from host on VSOCK port $CONFIG_PORT"
# See https://docs.aws.amazon.com/enclaves/latest/user/nitro-enclave-concepts.html
# "The CID used by the parent instance is always 3."
socat -u $SOCAT_OPTIONS VSOCK-CONNECT:3:$CONFIG_PORT STDOUT > config.json || {
    echo "$(date -Iseconds): ERROR: failed to connect to host for configuration"
    exit 1
}
echo "$(date -Iseconds): configuration received"

# Verify that we got something...
if ! [ -s config.json ]; then
    echo "$(date -Iseconds): ERROR: failed to receive configuration"
    exit 1
fi

# Check that config file is valid JSON.
if ! jq empty config.json >/dev/null 2>&1; then
    echo "$(date -Iseconds): ERROR: configuration is not valid JSON"
    exit 1
fi

# Extract logging ports from config
SOVEREIGN_STDOUT=$(jq '.logging.sovereign.stdout' config.json)
AGENT_STDOUT=$(jq '.logging.agent.stdout' config.json)



# -----------------------------------------------------------------------------
# STEP 2: Setup networking
# -----------------------------------------------------------------------------

mkdir -p /run/resolvconf
echo "nameserver 1.1.1.1" > /run/resolvconf/resolv.conf
ip link set lo up
echo "$(date -Iseconds): start 'dnsmasq' with nameserver 1.1.1.1"
dnsmasq &

echo "$(date -Iseconds): configure TAP "
socat -d VSOCK-CONNECT:3:$TAP_PORT TUN:10.0.0.1/31,tun-type=tap,up &
echo "$(date -Iseconds): started socat TUN/TAP device 'tap0' on VSOCK port $TAP_PORT"

# Wait up to 5 seconds for TAP interface to appear
for i in $(seq 0 50); do
    if [ $i -eq 50 ]; then
        echo "$(date -Iseconds): ERROR: timeout waiting for TAP interface 'tap0' to appear"
        exit 1
    fi
    if ip link show tap0 >/dev/null 2>&1; then
        echo "$(date -Iseconds): TAP interface 'tap0' is up"
        break
    fi
    sleep 0.1
done

ip route add default via 10.0.0.0 dev tap0
echo "$(date -Iseconds): added default route to 10.0.0.0 (host)"



# -----------------------------------------------------------------------------
# STEP 3: Start binaries
# -----------------------------------------------------------------------------

ls -l ./enclave
ldd ./enclave
# file ./enclave
ls -l /lib

# Start enclave with output redirection
# Note: the -u option to socat makes it unidirectional.
./enclave --config="$(jq '.sovereign' config.json)" 2>&1 | socat -u $SOCAT_OPTIONS STDIN VSOCK-CONNECT:3:$SOVEREIGN_STDOUT &
echo "$(date -Iseconds): sovereign binary started"

# Extract agent configuration and set environment variables
while IFS=': ' read -r key value; do
    # Remove quotes, convert to uppercase
    key=$(echo "$key" | tr -d '"' | tr '[:lower:]' '[:upper:]')
    value=$(echo "$value" | tr -d '"')
    export "$key"="$value"
    echo "$(date -Iseconds): setting environment variable $key=$value"
done < <(jq -r '.agent | to_entries | .[] | "\(.key): \(.value)"' config.json)

# Start agent with output redirection
./start_agent.sh 2>&1 | socat -u $SOCAT_OPTIONS STDIN VSOCK-CONNECT:3:$AGENT_STDOUT &
echo "$(date -Iseconds): agent binary started"

echo "$(date -Iseconds): enclave startup complete"

if [ "$(jq -r '."debug-mode"' config.json)" = "true" ]; then
    echo "$(date -Iseconds): network connectivity tests"
    
    echo "$(date -Iseconds): request to https://1.1.1.1:443 to check TAP connectivity"
    echo -e "GET /help/ HTTP/1.1\r\nHost: one.one.one.one\r\n\r\n" | socat $SOCAT_OPTIONS STDIO OPENSSL:1.1.1.1:443,verify=0,no-sni=1 | grep "HTTP/1.1"

    echo "$(date -Iseconds): request to https://one.one.one.one:443 to check TAP connectivity"
    echo -e "GET /help/ HTTP/1.1\r\nHost: one.one.one.one\r\n\r\n" | socat $SOCAT_OPTIONS STDIO OPENSSL:one.one.one.one:443,verify=0,no-sni=1 | grep "HTTP/1.1"
fi

# Loop waiting.
while true; do
    echo "$(date -Iseconds): waiting for background processes..."
    # Show current jobs before waiting
    jobs -l
    # Wait for some job to die.
    wait -n
    exit_code=$?
    echo "$(date -Iseconds): a process exited with code $exit_code"
    # Show remaining jobs after one exits
    jobs -l
    echo "$(date -Iseconds): exiting due to process termination (code $exit_code)"
    exit $exit_code
done
