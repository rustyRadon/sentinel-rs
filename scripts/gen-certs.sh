#!/bin/bash
set -e

# Create certs directory if it doesn't exist
mkdir -p certs

echo "--------------------------------------------------"
echo "Generating Certificate Authority (CA)..."
echo "--------------------------------------------------"
# 1. Generate CA Private Key
openssl genrsa -out certs/ca.key 4096

# 2. Generate CA Certificate (The Root of Trust)
openssl req -x509 -new -nodes -key certs/ca.key -sha256 -days 3650 \
    -out certs/ca.crt \
    -subj "/C=US/ST=State/L=City/O=Sentinel/CN=Sentinel Root CA"

echo "--------------------------------------------------"
echo "Generating Server Certificate..."
echo "--------------------------------------------------"
# 3. Generate Server Private Key
openssl genrsa -out certs/server.key 2048

# 4. Create Certificate Signing Request (CSR)
openssl req -new -key certs/server.key -out certs/server.csr \
    -subj "/C=US/ST=State/L=City/O=Sentinel/CN=10.114.101.7"

# 5. Create Extension file for SAN (Subject Alternative Name)
# This is CRITICAL for connecting over a real network IP.
cat > certs/server.ext << EOF
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
IP.1 = 127.0.0.1
IP.2 = 10.114.101.7
EOF

# 6. Sign the Server Certificate using our CA
openssl x509 -req -in certs/server.csr -CA certs/ca.crt -CAkey certs/ca.key \
    -CAcreateserial -out certs/server.crt -days 825 -sha256 -extfile certs/server.ext

# Clean up temporary files
rm certs/server.csr certs/server.ext

echo "--------------------------------------------------"
echo "SUCCESS: Certificates generated in ./certs/"
echo "Files created:"
ls -F certs/
echo "--------------------------------------------------"