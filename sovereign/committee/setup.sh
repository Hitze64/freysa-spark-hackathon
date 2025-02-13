#!/bin/bash

yum group install -y "Development Tools"

amazon-linux-extras enable aws-nitro-enclaves-cli
yum clean metadata && yum makecache

# Pin version for reproducable .EIF files
yum install -y aws-nitro-enclaves-cli-devel-1.3.4

yum install -y socat

yum install -y docker
systemctl start docker
systemctl enable docker
usermod -aG docker $USER

yum install -y aws-nitro-enclaves-cli
systemctl enable nitro-enclaves-allocator.service
systemctl start nitro-enclaves-allocator.service

mkdir -p bin
mkdir -p /var/log/nitro_enclaves
chown root:ne /var/log/nitro_enclaves
chmod 775 /var/log/nitro_enclaves
usermod -aG ne $USER

yum install -y protobuf protobuf-compiler protobuf-devel
