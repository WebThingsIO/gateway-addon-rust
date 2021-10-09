use webthings_gateway_ipc_types::{DevicePin, Link};

pub struct DeviceDescription {
    pub at_context: Option<String>,
    pub at_type: Option<Vec<String>>,
    pub base_href: Option<String>,
    pub credentials_required: Option<bool>,
    pub description: Option<String>,
    pub links: Option<Vec<Link>>,
    pub pin: Option<DevicePin>,
    pub title: Option<String>,
}
