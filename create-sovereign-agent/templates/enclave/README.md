# Unleash Your Sovereign Agent

## Setting up your agent repository for sovereign framework

1. First, look at `Dockerfile.agent`---the template runs `pnpm install` then `pnpm build` to build the code---you may have distinct commands for this. It also assumes that the build will be in `dist/` directory. Please change this if that is different for you.

2. Build and push your agent docker image to a registry with `Dockerfile.agent`
   
  For instance, using Dockerhub with `image-url=<account>/<image-name>`, one can do
  ```sh
  docker build --push --platform linux/amd64 -f Dockerfile.agent -t image-url .
  ```

3. In `enclave/` replace `{{DOCKER_IMAGE_URL}}` with your `image-url`

4. In `enclave/` add your agent start command to `start_agent.sh`

  For instance, if you built your code into a `dist/` folder with entrypoint `index.js`, this can just be
  ```sh
  node dist/index.js
  ````

5. You'll need to copy over the key,value pairs from your `.env` file to the `"agent"` value object in `config.json`. You can do so by running the following command.

  ```sh
  make populate-config ENV_FILE=path/to/.env

  ```
6. If one wants to use key-sync, then one has to fill out the `"safe"` value corresponding to their committee. This should have the following format.
  ```
  "safe" : {
    "wallet-address": "0x..."
    "threshold": n,
    "http-endpoint": "https://safe-transaction-sepolia.safe.global:443/api/v1/messages",
    "http-endpoint-port": 50000,
    "chain-id": 11155111
  }
  ```

Then you're ready to run your enclave!

## Create AWS nitro instance

See [AWS doc](https://docs.aws.amazon.com/enclaves/latest/user/getting-started.html) for more information on AWS Nitro Enclaves.

### With AWS-CLI

If you have your AWS key saved locally you can run

```sh
aws ec2 run-instances \
--image-id ami-01816d07b1128cd2d \
--count 1 \
--instance-type m5.xlarge \
--key-name <key-name> \
--security-group-ids sg-07251ab2aee251dff \
--tag-specifications 'ResourceType=instance,Tags=[{Key=Name,Value=<instance-name>}]' \
--enclave-options 'Enabled=true'
```

### With AWS console

- Pick AWS Linux 2023 AMI, enable Enclaves under advanced, pick an instance type that support enclaves and 30GiB of disk.
- Make sure to have at least 4 CPUs in the EC2 instance. Using model m5.xlarge is recommended.
- Make sure to have an AWS keypair available.
- Chose security group `launch-wizard-4`.

## Install dependencies on instance

1. Once you've ssh'ed into the instance, install git as follows.
    ```sh
    sudo yum install -y git
    ```
2. Clone this repo and `cd` into the `enclave/` folder, which is where you will run all of the following commands.

3. Install the needed depencies by running `make setup`.

4. You then have to `exit` the instance and ssh back in.

## Running the enclave

From the `enclave/` directory, you can run the sovereign agent in the enclave as follows.
- If you are not on a fresh instance, shut down any currently running enclaves by running `make stop`.
- Build the image file by running `make enclave.eif`.
- Start the enclave by running `make run-enclave`.

### Performing a Secure Key Sync

If you already have a sovereign agent running and want to transfer over their secrets to a new enclave, do the following.

- populate your `config.json` in follower mode by running
  ```sh
  make populate-config ENV_FILE=path/to/.env FOLLOWER=true
  ```

- run `export KEY_SYNC_IP=<LEADER_IP>`
  - can get `LEADER_IP` via running `curl ifconfig.me` on leader
  
- run `make enclave.eif`
- run `make run-enclave`

After these commands complete: 
- can run `cat sovereign.log` to check logs
- if key-sync succeeded, in `sovereign.log` one should see
    ```sh
    INFO enclave/src/key_sync.rs:155: key-sync successful (follower)
    ```

### Other potentially useful commands.

- run `make restart`: sometimes the service `nitro-enclaves-allocator.service` gets out of whack and needs to be restarted. Symptoms vary.
- run `make describe`: describe the running enclave(s). Will print `[]` if no enclave is running.
- run `make prune`: if you run out of disk space, it may be because it has too many docker artifacts.

## Interacting with the agent from the enclave

Run the following command to confirm that the server is running:

```sh
curl http://10.0.0.1:3002/health
```

Call the endpoint from outside the enclave

```sh
curl ifconfig.me
# get the public IP of the instance
curl http://<public-ip>:3002/health
```
