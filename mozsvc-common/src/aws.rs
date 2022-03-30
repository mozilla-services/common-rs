use std::time::Duration;

use reqwest;

lazy_static! {
    static ref EC2_INSTANCE_ID: Option<String> = _get_ec2_instance_id().ok();
}

/// Fetch the EC2 instance-id
///
/// Incurs a web request (potentially blocking) when called for the
/// first time
pub fn get_ec2_instance_id() -> Option<&'static str> {
    EC2_INSTANCE_ID.as_ref().map(String::as_ref)
}

fn _get_ec2_instance_id() -> reqwest::Result<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()?;
    client
        .get("http://169.254.169.254/latest/meta-data/instance-id")
        .send()?
        .error_for_status()?
        .text()
}
