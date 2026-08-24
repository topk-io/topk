use super::IndexName;

// Kibana never writes `.kibana` directly: it creates `<base>_<version>_<seq>` and points
// `<base>` and `<base>_<version>` at it, flipping them on migration. We derive that mapping from
// the name instead of storing one, which covers Kibana but no alias a user picks freely.
pub fn aliases_of(index: &IndexName) -> Vec<String> {
    let index = index.as_str();

    let (rest, seq) = match index.rsplit_once('_') {
        Some(parts) => parts,
        None => return Vec::new(),
    };
    if seq.is_empty() || !seq.chars().all(|c| c.is_ascii_digit()) {
        return Vec::new();
    }

    let (base, version) = match rest.rsplit_once('_') {
        Some(parts) => parts,
        None => return Vec::new(),
    };
    if version.is_empty() || !version.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return Vec::new();
    }

    vec![base.to_string(), rest.to_string()]
}

pub fn points_at(alias: &IndexName, index: &IndexName) -> bool {
    aliases_of(index).iter().any(|a| a == alias.as_str())
}
