use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

/// Generate a stable site code from location data.
/// Format: <FIRST_LETTER><2_CHAR_HASH><2_DIGIT_SEQ>
/// Example: Hirschaid → H8K01
pub async fn generate_site_code(
    pool: &SqlitePool,
    city: &str,
    zip: &str,
    street: &str,
    house_number: &str,
    country: &str,
) -> anyhow::Result<String> {
    let normalized_city = normalize_city(city);
    let first_char = normalized_city
        .chars()
        .next()
        .unwrap_or('X')
        .to_ascii_uppercase();

    // Stable hash from location data
    let hash_input = format!("{}-{}-{}-{}-{}", normalized_city, zip, street, house_number, country);
    let mut hasher = Sha256::new();
    hasher.update(hash_input.as_bytes());
    let hash_bytes = hasher.finalize();

    // Take 2 chars from hash, A-Z and 0-9
    let hash_str = encode_hash_chars(&hash_bytes[0..2]);

    // Find next unique sequence number
    let prefix = format!("{}{}", first_char, hash_str);
    let seq = find_next_site_seq(pool, &prefix).await?;

    Ok(format!("{}{:02}", prefix, seq))
}

fn normalize_city(city: &str) -> String {
    city.to_uppercase()
        .replace("Ä", "AE")
        .replace("Ö", "OE")
        .replace("Ü", "UE")
        .replace("ä", "AE")
        .replace("ö", "OE")
        .replace("ü", "UE")
        .replace("ß", "SS")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn encode_hash_chars(bytes: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ0123456789";
    let mut result = String::new();
    for &b in bytes.iter().take(2) {
        result.push(CHARS[(b as usize) % CHARS.len()] as char);
    }
    result
}

async fn find_next_site_seq(pool: &SqlitePool, prefix: &str) -> anyhow::Result<u32> {
    let pattern = format!("{}%", prefix);
    let existing: Vec<String> = sqlx::query_scalar!(
        "SELECT site_code FROM locations WHERE site_code LIKE ?",
        pattern
    )
    .fetch_all(pool)
    .await?;

    let mut max_seq = 0u32;
    for code in &existing {
        if code.len() > prefix.len() {
            if let Ok(seq) = code[prefix.len()..].parse::<u32>() {
                if seq > max_seq {
                    max_seq = seq;
                }
            }
        }
    }

    Ok(max_seq + 1)
}

/// Generate a hostname following the schema: <SITE>-<TYPE>-<ROLE>-<NN>
pub async fn generate_hostname(
    pool: &SqlitePool,
    site_code: &str,
    device_type: &str,
    role: &str,
) -> anyhow::Result<String> {
    let device_type = device_type.to_uppercase();
    let role = role.to_uppercase();
    let prefix = format!("{}-{}-{}-", site_code, device_type, role);

    let pattern = format!("{}%", prefix);
    let existing: Vec<String> = sqlx::query_scalar!(
        "SELECT hostname FROM assets WHERE hostname LIKE ?",
        pattern
    )
    .fetch_all(pool)
    .await?;

    let mut max_seq = 0u32;
    for hostname in &existing {
        if hostname.len() > prefix.len() {
            if let Ok(seq) = hostname[prefix.len()..].parse::<u32>() {
                if seq > max_seq {
                    max_seq = seq;
                }
            }
        }
    }

    Ok(format!("{}{:02}", prefix, max_seq + 1))
}

/// Validate hostname format
pub fn validate_hostname(hostname: &str) -> bool {
    // Only uppercase letters, digits, and hyphens
    // Format: SITE-TYPE-ROLE-NN
    let valid_chars = hostname
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-');

    let no_leading_trailing_hyphen = !hostname.starts_with('-') && !hostname.ends_with('-');
    let no_double_hyphen = !hostname.contains("--");

    valid_chars && no_leading_trailing_hyphen && no_double_hyphen && hostname.len() >= 5
}

pub const DEVICE_TYPES: &[(&str, &str)] = &[
    ("GW", "Gateway"),
    ("FW", "Firewall"),
    ("RTR", "Router"),
    ("SW", "Switch"),
    ("AP", "Access Point"),
    ("WLC", "WLAN Controller"),
    ("CAM", "Kamera"),
    ("NAS", "NAS"),
    ("SAN", "Storage"),
    ("SRV", "Server"),
    ("UPS", "USV"),
    ("PDU", "Stromverteiler"),
    ("IOT", "IoT-Gerät"),
    ("PRN", "Drucker"),
    ("MGMT", "Management Appliance"),
];

pub const DEVICE_ROLES: &[&str] = &[
    "EDGE", "CORE", "AGG", "DIST", "ACC", "MGMT", "DMZ", "GUEST",
    "PROD", "BACKUP", "HA", "PRI", "SEC", "TEST", "WIFI", "NVR", "INFRA",
];
