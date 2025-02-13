# TEE Setup 

## Create AWS nitro instance

See [AWS doc](https://docs.aws.amazon.com/enclaves/latest/user/getting-started.html) for more information on AWS Nitro Enclaves.

### With AWS-CLI

If you have a key name saved locally you can run

```sh
aws ec2 run-instances \
--image-id ami-01816d07b1128cd2d \
--count 1 \
--instance-type m5.xlarge \
--key-name <key-name> \
--security-group-ids sg-07251ab2aee251dff \
--tag-specifications 'ResourceType=instance,Tags=[{Key=Name,Value=my-nitro-tee-1}]' \
--enclave-options 'Enabled=true'
```

### With AWS console (recommended)

- Pick AWS Linux 2023 AMI, enable Enclaves under advanced, pick an instance type that support enclaves and 30GiB of disk.
- Make sure to have at least 4 CPUs in the EC2 instance. Using model m5.xlarge is recommended.
- Make sure to have an AWS keypair available.
- Setup a a security group with all necessary ports open.

## Install programs on instance

1. Once you've ssh'ed into the instance, install git

    ```sh
    sudo yum install -y git
    ```

2. Clone the repo using your personal access token
    - `ssh-keygen -t ed25519 -C "your@email.com"`
    - Go to `gitHub.com` -> *Settings* -> *SSH and GPG keys*
    - Click *New SSH key*
    - Paste output of `cat ~/.ssh/id_ed25519.pub`

    ```sh
    git clone git@github.com:0xfreysa/sovereign-freysa.git
    ```

3. `cd` into the repo and run

    ```sh
    make setup
    ```

4. `exit` and ssh back in, then you can run the enclave using instructions below

## Running the enclave

From the root directory, you can run the sovereign agent in the enclave as follows.
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

## Interacting with the enclave

Run the following command to confirm that the server is running:

```sh
curl http://10.0.0.1:3002/health
```

Call the endpoint from outside the enclave

```sh
curl ifconfig.me
#get the public IP of the instance
curl http://<public-ip>:3002/health
```
