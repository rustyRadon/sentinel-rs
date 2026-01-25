use std::path::Path;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls_pemfile::{certs, private_key};
use std::fs::File;
use std::io::BufReader;

pub fn load_certs(path: &Path) -> std::io::Result<Vec<CertificateDer<'static>>> {
    let mut reader = BufReader::new(File::open(path)?);
    certs(&mut reader).collect()
}

pub fn load_private_key(path: &Path) -> std::io::Result<PrivateKeyDer<'static>> {
    let mut reader = BufReader::new(File::open(path)?);
    private_key(&mut reader)?
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No private key found"))
}