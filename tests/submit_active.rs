mod common;

use gossip::GossipService;
use crate::common::TextMessageListener;

#[test]
fn submit_active() {
    let address_1 = "127.0.0.1:9000";
    let mut service_1 = GossipService::new_with_defaults(address_1.parse().unwrap());
    service_1.start(
        Box::new( || None),
        Box::new(TextMessageListener::new(address_1.to_owned()))
    ).unwrap();

    // JSON message
    let message_content = "{{ \"id\": \"toto\", \"name\": \"John Doe\" }}";

    assert!(service_1.submit(message_content.as_bytes().to_vec()).is_ok());
    assert_eq!(service_1.submit(message_content.as_bytes().to_vec()).err().unwrap().to_string(), "Message already active");
}
