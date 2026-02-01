//! Certificate Generation Tool

use anyhow::Result;
use clap::Parser;
use rcgen::{Certificate, CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use std::path::PathBuf;
use std::time::Duration;

/// Generate TLS certificates for SMTP Tunnel
#[derive(Parser, Debug)]
#[command(name = "smtp-tunnel-gen-certs")]
#[command(about = "Generate TLS certificates")]
#[command(version)]
struct Args {
    /// Hostname for the certificate
    #[arg(short, long, default_value = "mail.example.com")]
    hostname: String,

    /// Output directory
    #[arg(short, long, default_value = ".")]
    output: PathBuf,

    /// Validity in days
    #[arg(short, long, default_value = "365")]
    days: u64,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("Generating TLS certificates for: {}", args.hostname);
    println!("Output directory: {}", args.output.display());

    // Create output directory
    std::fs::create_dir_all(&args.output)?;

    // Use default algorithm (PKCS_RSA_SHA256)
    let alg = &rcgen::PKCS_RSA_SHA256;

    // Generate CA key pair
    let ca_key = KeyPair::generate(alg)?;

    // Generate CA certificate
    let mut ca_params = CertificateParams::new(vec!["SMTP Tunnel CA".to_string()]);
    ca_params.distinguished_name = DistinguishedName::new();
    ca_params.distinguished_name.push(DnType::OrganizationName, "SMTP Tunnel");
    ca_params.distinguished_name.push(DnType::CommonName, "SMTP Tunnel CA");
    ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![
        rcgen::KeyUsagePurpose::KeyCertSign,
        rcgen::KeyUsagePurpose::CrlSign,
    ];

    let ca_cert = Certificate::from_params(ca_params)?;

    // Generate server key pair
    let server_key = KeyPair::generate(alg)?;

    // Generate server certificate
    let mut server_params = CertificateParams::new(vec![args.hostname.clone()]);
    server_params.distinguished_name = DistinguishedName::new();
    server_params.distinguished_name.push(DnType::OrganizationName, "SMTP Tunnel");
    server_params.distinguished_name.push(DnType::CommonName, &args.hostname);
    
    // Add SAN
    server_params.subject_alt_names = vec![
        SanType::DnsName(args.hostname.parse()?),
    ];

    // Set validity
    server_params.not_before = time::OffsetDateTime::now_utc();
    server_params.not_after = server_params.not_before + Duration::from_secs(args.days * 24 * 60 * 60);

    // Key usage
    server_params.key_usages = vec![
        rcgen::KeyUsagePurpose::DigitalSignature,
        rcgen::KeyUsagePurpose::KeyEncipherment,
    ];
    server_params.extended_key_usages = vec![
        rcgen::ExtendedKeyUsagePurpose::ServerAuth,
    ];

    let server_cert = Certificate::from_params(server_params)?;

    // Write files
    let ca_cert_path = args.output.join("ca.crt");
    let server_cert_path = args.output.join("server.crt");
    let server_key_path = args.output.join("server.key");

    // Serialize PEM
    let ca_pem = ca_cert.serialize_pem_with_signer(&ca_cert)?;
    let server_pem = server_cert.serialize_pem_with_signer(&ca_cert)?;
    let server_key_pem = server_key.serialize_pem();

    std::fs::write(&ca_cert_path, ca_pem)?;
    std::fs::write(&server_cert_path, server_pem)?;
    std::fs::write(&server_key_path, server_key_pem)?;

    println!();
    println!("Generated certificates:");
    println!("  CA Certificate: {}", ca_cert_path.display());
    println!("  Server Certificate: {}", server_cert_path.display());
    println!("  Server Key: {}", server_key_path.display());
    println!();
    println!("Copy ca.crt to your clients for certificate verification.");
    println!("Server files (server.crt, server.key) stay on the server.");

    Ok(())
}
