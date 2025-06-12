

pub(crate) fn build_url(kme_hostname: &str, slave_sae_id: &str, path: &str) -> String {
    format!("https://{}/api/v1/keys/{}/{}", kme_hostname, slave_sae_id, path)
}