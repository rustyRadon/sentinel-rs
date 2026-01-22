use std::fs::File;
use std::io::{BufReader, Error, ErrorKind};
use std::path::Path;
use rustls::pki_types::{CertificateChain, PrivateKeyDer};
use rustls_pemfile::{certs, private_key};

pub fn load_certs(path: &Path) -> std::io::Result<Vec<CertificateChain<'static>>> {
    /// Open the file and wrap it in a BufReader for efficiency.
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let certs = certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()?;
    
    Ok(certs)
}

pub fn load_private_key(path: &Path) -> std::io::Result<PrivateKeyDer<'static>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    private_key(&mut reader)?
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "No private key found"))
}