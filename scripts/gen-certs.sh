#!/bin/bash
set -e

# Create certs directory if it doesn't exist
mkdir -p certs

echo "Generating Certificate Authority (CA)..."
# 1. Generate CA Private Key
openssl genrsa -out certs/ca.key 4096

# 2. Generate CA Certificate
openssl req -x509 -new -nodes -key certs/ca.key -sha256 -days 3650 \
    -out certs/ca.crt \
    -subj "/C=US/ST=State/L=City/O=Sentinel/CN=Sentinel Root CA"

echo "Generating Server Certificate..."
# 3. Generate Server Private Key
openssl genrsa -out certs/server.key 2048

# 4. Create Certificate Signing Request (CSR)
openssl req -new -key certs/server.key -out certs/server.csr \
    -subj "/C=US/ST=State/L=City/O=Sentinel/CN=localhost"

# 5. Create Extension file for SAN (Subject Alternative Name)
cat > certs/server.ext << EOF
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
IP.1 = 127.0.0.1
EOF

openssl x509 -req -in certs/server.csr -CA certs/ca.crt -CAkey certs/ca.key \
    -CAcreateserial -out certs/server.crt -days 825 -sha256 -extfile certs/server.ext

rm certs/server.csr certs/server.ext

echo "Certificates generated in ./certs/"
ls -l certs/

