use super::*;

use crate::service_protocol::messages::{
    output_entry_message, EndMessage, InputEntryMessage, OutputEntryMessage, StartMessage,
};
use assert2::let_assert;
use test_log::test;

#[test]
fn echo() {
    let mut output = VMTestCase::new(Version::V1)
        .input(StartMessage {
            id: Bytes::from_static(b"123"),
            debug_id: "123".to_string(),
            known_entries: 1,
            state_map: vec![],
            partial_state: false,
            key: "".to_string(),
        })
        .input(InputEntryMessage {
            headers: vec![],
            value: Bytes::from_static(b"my-data"),
            ..InputEntryMessage::default()
        })
        .run(|vm| {
            let_assert!(Input { input, .. } = vm.sys_input().unwrap());
            assert_eq!(input, b"my-data".to_vec());

            vm.sys_write_output(NonEmptyValue::Success(input)).unwrap();
            vm.sys_end().unwrap();
        });

    assert_eq!(
        output.next_decoded::<OutputEntryMessage>().unwrap(),
        OutputEntryMessage {
            result: Some(output_entry_message::Result::Value(Bytes::from_static(
                b"my-data"
            ))),
            ..OutputEntryMessage::default()
        }
    );
    assert_eq!(
        output.next_decoded::<EndMessage>().unwrap(),
        EndMessage::default()
    );
    assert_eq!(output.next(), None);
}
