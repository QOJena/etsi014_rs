

pub(crate) fn build_url(kme_hostname: &str, slave_sae_id: &str, path: &str, tls: bool) -> String {
    format!("http{}://{}/api/v1/keys/{}/{}", if tls { "s" } else { "" }, kme_hostname, slave_sae_id, path)
}