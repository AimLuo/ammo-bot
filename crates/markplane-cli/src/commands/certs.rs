//! Localhost self-signed certificate management for `markplane serve`.

use std::fs;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use rand::RngExt;
use rcgen::{
    CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose, SanType, SerialNumber,
};
use x509_parser::prelude::*;
#[cfg(test)]
use x509_parser::public_key::PublicKey;

const CERT_FILE: &str = "cert.pem";
const KEY_FILE: &str = "key.pem";
const VALIDITY_DAYS: i64 = 800;
const RENEW_BEFORE_DAYS: i64 = 30;
const NOT_BEFORE_SKEW: Duration = Duration::from_secs(3600);

/// Result of ensuring a usable localhost certificate pair exists.
#[derive(Debug, Clone)]
pub struct EnsuredCerts {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    /// True when a new certificate was written (first generate or renewal).
    pub renewed: bool,
}

/// Resolve the markplane config directory (`$XDG_CONFIG_HOME/markplane` or `~/.config/markplane`).
pub fn markplane_config_dir() -> Result<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let trimmed = xdg.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed).join("markplane"));
        }
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .context("neither HOME nor USERPROFILE is set")?;
    Ok(PathBuf::from(home).join(".config").join("markplane"))
}

/// Directory that holds `cert.pem` / `key.pem`.
pub fn markplane_certs_dir() -> Result<PathBuf> {
    Ok(markplane_config_dir()?.join("certs"))
}

/// Ensure `certs_dir` contains a valid localhost leaf certificate and private key.
///
/// Regenerates when missing, unreadable, near expiry (< 30 days), or SAN mismatch.
pub fn ensure_certs(certs_dir: &Path) -> Result<EnsuredCerts> {
    let cert_path = certs_dir.join(CERT_FILE);
    let key_path = certs_dir.join(KEY_FILE);

    if cert_path.is_file() && key_path.is_file() {
        match existing_cert_usable(&cert_path) {
            Ok(true) => {
                return Ok(EnsuredCerts {
                    cert_path,
                    key_path,
                    renewed: false,
                });
            }
            Ok(false) => {}
            Err(err) => {
                eprintln!("Existing certificate unusable ({err}); regenerating");
            }
        }
    }

    generate_certs(certs_dir)?;
    Ok(EnsuredCerts {
        cert_path,
        key_path,
        renewed: true,
    })
}

/// OS-specific commands to trust `cert_path` in local browsers.
pub fn trust_instructions(cert_path: &Path) -> String {
    let cert = cert_path.display();
    if cfg!(target_os = "macos") {
        format!(
            "Trust this certificate (macOS Keychain):\n  security add-trusted-cert -r trustRoot -p ssl \\\n    -k \"$HOME/Library/Keychains/login.keychain-db\" \\\n    {cert}"
        )
    } else if cfg!(target_os = "linux") {
        format!(
            "Trust this certificate (Linux browsers):\n  # Chrome / Chromium (NSS trusted peer):\n  certutil -d sql:\"$HOME/.local/share/pki/nssdb\" -A -t \"P,,\" -n markplane-localhost \\\n    -i {cert}\n  # Firefox (each profile; use about:profiles for the path):\n  certutil -d sql:<firefox-profile-dir> -A -t \"C,,\" -n markplane-localhost \\\n    -i {cert}"
        )
    } else {
        format!("Trust this certificate in your OS/browser trust store:\n  {cert}")
    }
}

fn generate_certs(certs_dir: &Path) -> Result<()> {
    create_dir_private(certs_dir)?;

    let key_pair = KeyPair::generate().context("generate ECDSA P-256 key")?;
    let params = localhost_params()?;
    let cert = params
        .self_signed(&key_pair)
        .context("self-sign localhost certificate")?;

    let key_pem = key_pair.serialize_pem();
    let cert_pem = cert.pem();

    let key_tmp = certs_dir.join(format!(".{KEY_FILE}.tmp"));
    let cert_tmp = certs_dir.join(format!(".{CERT_FILE}.tmp"));

    write_private_file(&key_tmp, key_pem.as_bytes())?;
    write_public_file(&cert_tmp, cert_pem.as_bytes())?;

    fs::rename(&key_tmp, certs_dir.join(KEY_FILE))
        .with_context(|| format!("install {KEY_FILE}"))?;
    fs::rename(&cert_tmp, certs_dir.join(CERT_FILE))
        .with_context(|| format!("install {CERT_FILE}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(certs_dir.join(KEY_FILE), fs::Permissions::from_mode(0o600))?;
        fs::set_permissions(certs_dir.join(CERT_FILE), fs::Permissions::from_mode(0o644))?;
        fs::set_permissions(certs_dir, fs::Permissions::from_mode(0o700))?;
    }

    Ok(())
}

fn localhost_params() -> Result<CertificateParams> {
    let mut params = CertificateParams::default();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "markplane localhost");
    params.distinguished_name = dn;
    params.subject_alt_names = vec![
        SanType::DnsName("localhost".try_into().context("SAN DNS localhost")?),
        SanType::IpAddress(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        SanType::IpAddress(IpAddr::V6(Ipv6Addr::LOCALHOST)),
    ];
    params.is_ca = IsCa::ExplicitNoCa;
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

    let mut serial = [0u8; 16];
    rand::rng().fill(&mut serial);
    // Keep serial positive / non-zero leading bit clear for DER INTEGER.
    serial[0] &= 0x7f;
    if serial.iter().all(|&b| b == 0) {
        serial[15] = 1;
    }
    params.serial_number = Some(SerialNumber::from_slice(&serial));

    let now = ::time::OffsetDateTime::now_utc();
    params.not_before = now - ::time::Duration::seconds(NOT_BEFORE_SKEW.as_secs() as i64);
    params.not_after = params.not_before + ::time::Duration::days(VALIDITY_DAYS);

    Ok(params)
}

fn existing_cert_usable(cert_path: &Path) -> Result<bool> {
    let pem = fs::read(cert_path).with_context(|| format!("read {}", cert_path.display()))?;
    let der = pem_to_der(&pem)?;
    let (_, cert) = X509Certificate::from_der(&der).context("parse certificate DER")?;

    let not_after = cert_time_to_utc(cert.validity().not_after)?;
    let remaining = not_after.signed_duration_since(Utc::now());
    if remaining < chrono::Duration::days(RENEW_BEFORE_DAYS) {
        return Ok(false);
    }

    if !san_matches_localhost(&cert)? {
        return Ok(false);
    }

    Ok(true)
}

fn san_matches_localhost(cert: &X509Certificate<'_>) -> Result<bool> {
    let Some(san_ext) = cert
        .subject_alternative_name()
        .context("read SAN extension")?
    else {
        return Ok(false);
    };

    let mut has_dns_localhost = false;
    let mut has_v4 = false;
    let mut has_v6 = false;
    for name in &san_ext.value.general_names {
        match name {
            GeneralName::DNSName(dns) if *dns == "localhost" => has_dns_localhost = true,
            GeneralName::IPAddress(bytes) => match bytes.len() {
                4 if *bytes == [127, 0, 0, 1] => has_v4 = true,
                16 if bytes[..15].iter().all(|&b| b == 0) && bytes[15] == 1 => has_v6 = true,
                _ => {}
            },
            _ => {}
        }
    }
    Ok(has_dns_localhost && has_v4 && has_v6)
}

fn cert_time_to_utc(t: ASN1Time) -> Result<DateTime<Utc>> {
    let ts = t.timestamp();
    Utc.timestamp_opt(ts, 0)
        .single()
        .context("certificate time out of range")
}

fn pem_to_der(pem_bytes: &[u8]) -> Result<Vec<u8>> {
    let (_, pem) = x509_parser::pem::parse_x509_pem(pem_bytes).context("parse certificate PEM")?;
    if pem.label != "CERTIFICATE" {
        anyhow::bail!("expected CERTIFICATE PEM, got {}", pem.label);
    }
    Ok(pem.contents)
}

fn create_dir_private(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn write_private_file(path: &Path, bytes: &[u8]) -> Result<()> {
    write_with_mode(path, bytes, 0o600)
}

fn write_public_file(path: &Path, bytes: &[u8]) -> Result<()> {
    write_with_mode(path, bytes, 0o644)
}

fn write_with_mode(
    path: &Path,
    bytes: &[u8],
    #[cfg_attr(not(unix), allow(unused))] mode: u32,
) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(mode)
            .open(path)
            .with_context(|| format!("create {}", path.display()))?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let mut file =
            fs::File::create(path).with_context(|| format!("create {}", path.display()))?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    Ok(())
}

/// Inspect certificate fields for tests and diagnostics.
#[cfg(test)]
fn inspect_cert(cert_path: &Path) -> Result<CertInfo> {
    let pem_bytes = fs::read(cert_path)?;
    let der = pem_to_der(&pem_bytes)?;
    let (_, cert) = X509Certificate::from_der(&der)?;

    let not_before = cert_time_to_utc(cert.validity().not_before)?;
    let not_after = cert_time_to_utc(cert.validity().not_after)?;
    let days = not_after.signed_duration_since(not_before).num_days();

    let mut dns = Vec::new();
    let mut ips = Vec::new();
    if let Some(san_ext) = cert.subject_alternative_name()? {
        for name in &san_ext.value.general_names {
            match name {
                GeneralName::DNSName(d) => dns.push(d.to_string()),
                GeneralName::IPAddress(bytes) => match bytes.len() {
                    4 => ips.push(format!(
                        "{}.{}.{}.{}",
                        bytes[0], bytes[1], bytes[2], bytes[3]
                    )),
                    16 => {
                        let mut addr = [0u8; 16];
                        addr.copy_from_slice(bytes);
                        ips.push(Ipv6Addr::from(addr).to_string());
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    let mut has_server_auth = false;
    let mut ku_digital_signature = false;
    let mut ku_critical = false;
    let mut bc_not_ca = false;
    let mut bc_critical = false;

    for ext in cert.extensions() {
        match ext.parsed_extension() {
            ParsedExtension::ExtendedKeyUsage(eku) => {
                has_server_auth = eku.server_auth;
            }
            ParsedExtension::KeyUsage(ku) => {
                ku_digital_signature = ku.digital_signature();
                ku_critical = ext.critical;
            }
            ParsedExtension::BasicConstraints(bc) => {
                bc_not_ca = !bc.ca;
                bc_critical = ext.critical;
            }
            _ => {}
        }
    }

    let is_ec_public_key = matches!(
        cert.tbs_certificate.subject_pki.parsed(),
        Ok(PublicKey::EC(_))
    );

    let _ = (not_before, not_after);
    Ok(CertInfo {
        validity_days: days,
        dns_names: dns,
        ip_addresses: ips,
        has_server_auth,
        key_usage_digital_signature: ku_digital_signature,
        key_usage_critical: ku_critical,
        basic_constraints_not_ca: bc_not_ca,
        basic_constraints_critical: bc_critical,
        is_ec_public_key,
    })
}

#[cfg(test)]
#[derive(Debug)]
struct CertInfo {
    pub validity_days: i64,
    pub dns_names: Vec<String>,
    pub ip_addresses: Vec<String>,
    pub has_server_auth: bool,
    pub key_usage_digital_signature: bool,
    pub key_usage_critical: bool,
    pub basic_constraints_not_ca: bool,
    pub basic_constraints_critical: bool,
    pub is_ec_public_key: bool,
}

/// Write a short-lived or wrong-SAN cert for renewal tests.
#[cfg(test)]
fn write_test_cert(certs_dir: &Path, validity_days: i64, sans: &[SanType]) -> Result<()> {
    create_dir_private(certs_dir)?;
    let key_pair = KeyPair::generate()?;
    let mut params = CertificateParams::default();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "markplane test");
    params.distinguished_name = dn;
    params.subject_alt_names = sans.to_vec();
    params.is_ca = IsCa::ExplicitNoCa;
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let now = ::time::OffsetDateTime::now_utc();
    params.not_before = now - ::time::Duration::hours(1);
    params.not_after = now + ::time::Duration::days(validity_days);
    let cert = params.self_signed(&key_pair)?;
    write_private_file(
        &certs_dir.join(KEY_FILE),
        key_pair.serialize_pem().as_bytes(),
    )?;
    write_public_file(&certs_dir.join(CERT_FILE), cert.pem().as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[cfg(unix)]
    fn mode(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn generates_localhost_cert_with_required_fields_and_permissions() {
        let tmp = TempDir::new().unwrap();
        let certs = tmp.path().join("certs");
        let ensured = ensure_certs(&certs).unwrap();
        assert!(ensured.renewed);
        assert!(ensured.cert_path.is_file());
        assert!(ensured.key_path.is_file());

        let info = inspect_cert(&ensured.cert_path).unwrap();
        assert!(info.is_ec_public_key);
        assert!(info.has_server_auth);
        assert!(info.key_usage_digital_signature);
        assert!(info.key_usage_critical);
        assert!(info.basic_constraints_not_ca);
        assert!(info.basic_constraints_critical);
        assert!(info.dns_names.iter().any(|d| d == "localhost"));
        assert!(info.ip_addresses.iter().any(|ip| ip == "127.0.0.1"));
        assert!(info.ip_addresses.iter().any(|ip| ip == "::1"));
        assert!(
            (790..=810).contains(&info.validity_days),
            "validity_days={}",
            info.validity_days
        );

        #[cfg(unix)]
        {
            assert_eq!(mode(&certs), 0o700);
            assert_eq!(mode(&ensured.key_path), 0o600);
            assert_eq!(mode(&ensured.cert_path), 0o644);
        }
    }

    #[test]
    fn reuses_valid_cert_on_second_ensure() {
        let tmp = TempDir::new().unwrap();
        let certs = tmp.path().join("certs");
        let first = ensure_certs(&certs).unwrap();
        let pem_before = fs::read(&first.cert_path).unwrap();
        let second = ensure_certs(&certs).unwrap();
        assert!(!second.renewed);
        let pem_after = fs::read(&second.cert_path).unwrap();
        assert_eq!(pem_before, pem_after);
    }

    #[test]
    fn regenerates_when_cert_missing() {
        let tmp = TempDir::new().unwrap();
        let certs = tmp.path().join("certs");
        let first = ensure_certs(&certs).unwrap();
        fs::remove_file(&first.cert_path).unwrap();
        let second = ensure_certs(&certs).unwrap();
        assert!(second.renewed);
        assert!(second.cert_path.is_file());
    }

    #[test]
    fn regenerates_when_san_mismatches() {
        let tmp = TempDir::new().unwrap();
        let certs = tmp.path().join("certs");
        write_test_cert(
            &certs,
            800,
            &[SanType::DnsName("wrong.example".try_into().unwrap())],
        )
        .unwrap();
        let before = fs::read(certs.join(CERT_FILE)).unwrap();
        let ensured = ensure_certs(&certs).unwrap();
        assert!(ensured.renewed);
        let after = fs::read(&ensured.cert_path).unwrap();
        assert_ne!(before, after);
        let info = inspect_cert(&ensured.cert_path).unwrap();
        assert!(info.dns_names.iter().any(|d| d == "localhost"));
    }

    #[test]
    fn regenerates_when_near_expiry() {
        let tmp = TempDir::new().unwrap();
        let certs = tmp.path().join("certs");
        write_test_cert(
            &certs,
            10,
            &[
                SanType::DnsName("localhost".try_into().unwrap()),
                SanType::IpAddress(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                SanType::IpAddress(IpAddr::V6(Ipv6Addr::LOCALHOST)),
            ],
        )
        .unwrap();
        let before = fs::read(certs.join(CERT_FILE)).unwrap();
        let ensured = ensure_certs(&certs).unwrap();
        assert!(ensured.renewed);
        assert_ne!(before, fs::read(&ensured.cert_path).unwrap());
        let info = inspect_cert(&ensured.cert_path).unwrap();
        assert!((790..=810).contains(&info.validity_days));
    }

    #[test]
    fn trust_instructions_mention_cert_path() {
        let path = PathBuf::from("/tmp/markplane/certs/cert.pem");
        let text = trust_instructions(&path);
        assert!(text.contains("/tmp/markplane/certs/cert.pem"));
        if cfg!(target_os = "macos") {
            assert!(text.contains("security add-trusted-cert"));
        } else if cfg!(target_os = "linux") {
            assert!(text.contains("certutil"));
            assert!(text.contains("P,,"));
            assert!(text.contains("C,,"));
        }
    }

    #[test]
    fn markplane_certs_dir_respects_xdg_config_home() {
        let tmp = TempDir::new().unwrap();
        let prev = std::env::var_os("XDG_CONFIG_HOME");
        // SAFETY: single-threaded test; restored before return.
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        }
        let dir = markplane_certs_dir().unwrap();
        assert_eq!(dir, tmp.path().join("markplane").join("certs"));
        unsafe {
            match prev {
                Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }
    }
}
