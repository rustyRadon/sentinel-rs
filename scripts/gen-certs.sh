
set -e

CERT_DIR="./certs"
mkdir -p $CERT_DIR

echo "Creating Sentinel-rs CA..."
openssl ecparam -name prime256v1 -genkey -noout -out $CERT_DIR/ca.key
openssl req -new -x509 -sha256 -key $CERT_DIR/ca.key -out $CERT_DIR/ca.pem \
    -subj "/C=US/ST=Tech/L=Sentinel/O=Sentinel-rs/CN=Sentinel Root CA" -days 3650

echo "Creating Server Certificate..."
openssl ecparam -name prime256v1 -genkey -noout -out $CERT_DIR/server.key
openssl req -new -key $CERT_DIR/server.key -out $CERT_DIR/server.csr \
    -subj "/C=US/ST=Tech/L=Sentinel/O=Sentinel-rs/CN=localhost"

# Create SAN config for localhost
cat > $CERT_DIR/server.ext <<EOF
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
subjectAltName = @alt_names
[alt_names]
DNS.1 = localhost
IP.1 = 127.0.0.1
EOF

openssl x509 -req -in $CERT_DIR/server.csr -CA $CERT_DIR/ca.pem -CAkey $CERT_DIR/ca.key \
    -CAcreateserial -out $CERT_DIR/server.pem -days 365 -sha256 -extfile $CERT_DIR/server.ext

echo "Creating Client Certificate (for mTLS)..."
openssl ecparam -name prime256v1 -genkey -noout -out $CERT_DIR/client.key
openssl req -new -key $CERT_DIR/client.key -out $CERT_DIR/client.csr \
    -subj "/C=US/ST=Tech/L=Sentinel/O=Sentinel-rs/CN=sentinel-client-01"

openssl x509 -req -in $CERT_DIR/client.csr -CA $CERT_DIR/ca.pem -CAkey $CERT_DIR/ca.key \
    -CAcreateserial -out $CERT_DIR/client.pem -days 365 -sha256

# Cleanup CSRs and temp files
rm $CERT_DIR/*.csr $CERT_DIR/*.ext $CERT_DIR/*.srl
echo "Certificates generated in $CERT_DIR"